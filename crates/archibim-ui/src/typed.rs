//! Values typed at the keyboard while drawing, shared by every drawing
//! tool: a length, an angle and offsets in x and y, alone or together. The
//! next point goes at a typed distance from the last one, along a typed
//! angle or towards the pointer, or at typed offsets along the plan's axes.
//! Before the first point, a typed length runs along the guide the pointer
//! is on, and typed offsets measure from the plan's origin.

use archibim_model::Point;

use crate::Snap;

/// The value being typed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Field {
    #[default]
    Length,
    Angle,
    X,
    Y,
}

/// What was typed for the next point.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Typed {
    length: String,
    /// In degrees anticlockwise from the previous segment carried on, or
    /// from the x axis for the first segment: 0 goes straight on, 90
    /// turns left, -90 turns right.
    angle: String,
    /// Offsets from the last point (or the reference) along the plan's
    /// axes: x to the right, y up the plan.
    x: String,
    y: String,
    field: Field,
}

impl Typed {
    /// Whether nothing was typed.
    pub fn is_empty(&self) -> bool {
        self.length.is_empty()
            && self.angle.is_empty()
            && self.x.is_empty()
            && self.y.is_empty()
            && self.field == Field::Length
    }

    /// What to show next to the pointer, given `live`, the length and the
    /// angle from the last point if there is one, and `offset`, the
    /// pointer's x and y from it: with nothing typed, the live values; else
    /// each typed value, the live ones in brackets, and a bar after the
    /// value being typed. The offsets show on a second line once used.
    pub fn label(&self, live: Option<(f64, f64)>, offset: Option<(f64, f64)>) -> String {
        let polar = self.polar_label(live);
        if self.x.is_empty() && self.y.is_empty() && !matches!(self.field, Field::X | Field::Y) {
            return polar;
        }
        let value = |typed: &str, live: Option<f64>, field| {
            let v = match live {
                Some(v) if typed.is_empty() => format!("({v:.0})"),
                _ => typed.to_owned(),
            };
            format!("{v}{}", if self.field == field { "|" } else { "" })
        };
        let offsets = format!(
            "x {}  y {}",
            value(&self.x, offset.map(|o| o.0), Field::X),
            value(&self.y, offset.map(|o| o.1), Field::Y)
        );
        if polar.is_empty() {
            offsets
        } else {
            format!("{polar}\n{offsets}")
        }
    }

    /// The length and angle part of the label.
    fn polar_label(&self, live: Option<(f64, f64)>) -> String {
        let bar = |field| if self.field == field { "|" } else { "" };
        let length = match (self.length.is_empty(), live) {
            (false, _) => self.length.clone(),
            (true, Some((l, _))) => format!("({l:.0})"),
            (true, None) => String::new(),
        };
        let untouched =
            self.length.is_empty() && self.angle.is_empty() && self.field != Field::Angle;
        match live {
            None if self.length.is_empty() && self.field != Field::Length => String::new(),
            None if self.is_empty() => String::new(),
            None => format!("{length}{}", bar(Field::Length)),
            Some((l, a)) if untouched => format!("{l:.0}  {a:.1}°"),
            Some((_, a)) => {
                let angle = if self.angle.is_empty() {
                    format!("({a:.1}°)")
                } else {
                    format!("{}°", self.angle)
                };
                format!(
                    "{length}{} < {angle}{}",
                    bar(Field::Length),
                    bar(Field::Angle)
                )
            }
        }
    }

    /// The typed length, if it is a length.
    pub fn length(&self) -> Option<f64> {
        number(&self.length).filter(|l| *l > 0.0)
    }

    /// Tab or `<`: typing goes to the next value, length, angle, x, y and
    /// back to the length. An angle needs a point to turn around, so it is
    /// skipped before the first point.
    pub fn switch_field(&mut self, drawing: bool) {
        self.field = match self.field {
            Field::Length if drawing => Field::Angle,
            Field::Length | Field::Angle => Field::X,
            Field::X => Field::Y,
            Field::Y => Field::Length,
        };
    }

    /// Forgets what was typed.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// The value being typed.
    fn active(&mut self) -> &mut String {
        match self.field {
            Field::Length => &mut self.length,
            Field::Angle => &mut self.angle,
            Field::X => &mut self.x,
            Field::Y => &mut self.y,
        }
    }

    /// A typed character: digits and a decimal separator build the value
    /// being typed; `<` goes to the next value, `x` and `y` straight to
    /// that offset. The angle and the offsets may be negative.
    pub fn type_char(&mut self, c: char, drawing: bool) {
        match c {
            '<' => self.switch_field(drawing),
            'x' | 'X' => self.field = Field::X,
            'y' | 'Y' => self.field = Field::Y,
            '-' if self.field != Field::Length && self.active().is_empty() => {
                self.active().push(c);
            }
            c if c.is_ascii_digit() || c == '.' || c == ',' => self.active().push(c),
            _ => {}
        }
    }

    /// Backspace: erases the last typed character, or goes back to the
    /// value before once this one is empty.
    pub fn erase(&mut self) {
        if self.active().pop().is_none() {
            self.field = match self.field {
                Field::Y => Field::X,
                Field::X if !self.angle.is_empty() => Field::Angle,
                _ => Field::Length,
            };
        }
    }

    /// The point placed with what was typed, or nothing if what was typed
    /// does not give one. From the last point `start`: the typed length
    /// along the typed angle, turned from `base`, the direction in degrees
    /// of the previous segment (0 with none), or towards the pointer; with
    /// an angle and no length, the pointer's distance along that angle.
    /// A typed x or y fixes the point along the plan's axes, the pointer
    /// giving what was not typed. Before the first point: the typed length
    /// along the guide the pointer is on, from where the guide starts, or
    /// the typed offsets from the plan's origin.
    pub fn point(&self, start: Option<Point>, base: f64, snap: &Snap) -> Option<Point> {
        let (length, angle) = (number(&self.length), number(&self.angle));
        let (x, y) = (number(&self.x), number(&self.y));
        if x.is_some() || y.is_some() {
            return offset_point(start, base, snap.point, length, angle, x, y);
        }
        let Some(start) = start else {
            let (origin, u) = snap.along?;
            let l = length.filter(|l| *l > 0.0)?;
            return Some(Point::new(origin.x + u.x * l, origin.y + u.y * l));
        };
        let toward = snap.point;
        let (dx, dy) = match angle {
            Some(degrees) => {
                let a = (base + degrees).to_radians();
                (a.cos(), a.sin())
            }
            None => {
                let (dx, dy) = (toward.x - start.x, toward.y - start.y);
                let norm = (dx * dx + dy * dy).sqrt();
                if norm == 0.0 {
                    return None;
                }
                (dx / norm, dy / norm)
            }
        };
        let length = match (length, angle) {
            (Some(l), _) => l,
            (None, Some(_)) => (toward.x - start.x) * dx + (toward.y - start.y) * dy,
            (None, None) => return None,
        };
        (length > 0.0).then(|| Point::new(start.x + dx * length, start.y + dy * length))
    }
}

/// The point fixed by a typed x or y, from `start` or the plan's origin,
/// with the length or the angle if one was typed too; x and y together win
/// over both, and the pointer `toward` gives the rest. A length shorter
/// than the offset, an angle that never reaches it, or no move at all give
/// no point: the label warns, nothing is guessed.
fn offset_point(
    start: Option<Point>,
    base: f64,
    toward: Point,
    length: Option<f64>,
    angle: Option<f64>,
    x: Option<f64>,
    y: Option<f64>,
) -> Option<Point> {
    let origin = start.unwrap_or(Point::new(0.0, 0.0));
    let (px, py) = (toward.x - origin.x, toward.y - origin.y);
    let side = |v: f64| if v < 0.0 { -1.0 } else { 1.0 };
    // An angle turns from the previous segment, so it needs a point.
    let direction = angle
        .filter(|_| start.is_some())
        .map(|a| (base + a).to_radians());
    let (dx, dy) = match (x, y, length.filter(|l| *l > 0.0), direction) {
        (Some(x), Some(y), ..) => (x, y),
        (Some(x), None, Some(l), None) if l >= x.abs() => (x, side(py) * (l * l - x * x).sqrt()),
        (None, Some(y), Some(l), None) if l >= y.abs() => (side(px) * (l * l - y * y).sqrt(), y),
        // Too short, or a length and an angle both with one offset: one
        // value too many, nothing guessed.
        (Some(_), None, Some(_), _) | (None, Some(_), Some(_), _) => return None,
        (Some(x), None, None, Some(a)) => {
            let l = x / a.cos();
            if a.cos().abs() < 1e-9 || l <= 0.0 {
                return None;
            }
            (x, l * a.sin())
        }
        (None, Some(y), None, Some(a)) => {
            let l = y / a.sin();
            if a.sin().abs() < 1e-9 || l <= 0.0 {
                return None;
            }
            (l * a.cos(), y)
        }
        (Some(x), None, None, None) => (x, py),
        (None, Some(y), None, None) => (px, y),
        (None, None, ..) => return None,
    };
    // A zero move places nothing, except at the plan's origin itself.
    let moved = dx.hypot(dy) > 1e-9;
    (moved || start.is_none()).then(|| Point::new(origin.x + dx, origin.y + dy))
}

/// A typed number, a comma or a dot as the decimal separator.
pub(crate) fn number(s: &str) -> Option<f64> {
    s.replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SnapKind;

    fn at(x: f64, y: f64) -> Snap {
        Snap {
            point: Point::new(x, y),
            kind: SnapKind::Free,
            guides: Vec::new(),
            along: None,
        }
    }

    fn close(a: Point, x: f64, y: f64) -> bool {
        (a.x - x).abs() < 1e-6 && (a.y - y).abs() < 1e-6
    }

    #[test]
    fn a_typed_length_goes_towards_the_pointer() {
        let mut t = Typed::default();
        "4000".chars().for_each(|c| t.type_char(c, true));
        let p = t.point(Some(Point::new(3000.0, 0.0)), 0.0, &at(3000.0, 10.0));
        assert!(p.is_some_and(|p| close(p, 3000.0, 4000.0)), "{p:?}");
        assert!(
            t.point(Some(Point::new(0.0, 0.0)), 0.0, &at(0.0, 0.0))
                .is_none()
        );
    }

    #[test]
    fn a_typed_angle_sets_the_direction_with_or_without_a_length() {
        let start = Some(Point::new(0.0, 0.0));
        let mut t = Typed::default();
        "2000<90".chars().for_each(|c| t.type_char(c, true));
        assert_eq!(t.label(Some((500.0, 0.0)), None), "2000 < 90°|");
        let p = t.point(start, 0.0, &at(500.0, 0.0));
        assert!(p.is_some_and(|p| close(p, 0.0, 2000.0)), "{p:?}");
        // An angle alone: the pointer gives the length along it.
        t.clear();
        t.switch_field(true);
        "-45".chars().for_each(|c| t.type_char(c, true));
        let p = t.point(start, 0.0, &at(1000.0, 0.0));
        assert!(p.is_some_and(|p| close(p, 500.0, -500.0)), "{p:?}");
        // Backspace empties the angle, then goes back to the length.
        (0..4).for_each(|_| t.erase());
        assert!(t.is_empty());
    }

    #[test]
    fn a_typed_angle_turns_from_the_previous_segment() {
        // The previous segment went up the plan (90°): 90 turns left, -90
        // right, 0 goes straight on.
        let start = Some(Point::new(0.0, 0.0));
        for (angle, x, y) in [
            ("90", -1000.0, 0.0),
            ("-90", 1000.0, 0.0),
            ("0", 0.0, 1000.0),
        ] {
            let mut t = Typed::default();
            "1000".chars().for_each(|c| t.type_char(c, true));
            t.switch_field(true);
            angle.chars().for_each(|c| t.type_char(c, true));
            let p = t.point(start, 90.0, &at(5.0, 5.0));
            assert!(p.is_some_and(|p| close(p, x, y)), "{angle}: {p:?}");
        }
    }

    #[test]
    fn before_the_first_point_a_length_runs_along_the_guide() {
        let mut t = Typed::default();
        "1500".chars().for_each(|c| t.type_char(c, false));
        let on_guide = Snap {
            along: Some((Point::new(1000.0, 0.0), Point::new(0.0, 1.0))),
            ..at(1000.0, 800.0)
        };
        let p = t.point(None, 0.0, &on_guide);
        assert!(p.is_some_and(|p| close(p, 1000.0, 1500.0)), "{p:?}");
        assert!(t.point(None, 0.0, &at(5.0, 5.0)).is_none(), "off any guide");
        // No angle without a point to turn around: Tab goes to x.
        t.switch_field(false);
        "90".chars().for_each(|c| t.type_char(c, false));
        assert_eq!(t.label(None, Some((5.0, 6.0))), "1500\nx 90|  y (6)");
    }

    fn keys(keys: &str, drawing: bool) -> Typed {
        let mut t = Typed::default();
        keys.chars().for_each(|c| t.type_char(c, drawing));
        t
    }

    #[test]
    fn typed_offsets_fix_the_point_along_the_plan_axes() {
        let start = Some(Point::new(100.0, 100.0));
        let pointer = at(400.0, -300.0);
        // (keys, expected point): x and y from the last point, the pointer
        // giving what was not typed.
        for (k, x, y) in [
            ("x300y-200", 400.0, -100.0),
            ("x-50", 50.0, -300.0),
            ("y250", 400.0, 350.0),
            ("x0", 100.0, -300.0),
            // A length with x: y on the pointer's side, below here.
            ("500x300", 400.0, -300.0),
            ("500y-400", 400.0, -300.0),
            // x and y win over the length.
            ("9999x10y20", 110.0, 120.0),
        ] {
            let p = keys(k, true).point(start, 0.0, &pointer);
            assert!(p.is_some_and(|p| close(p, x, y)), "{k}: {p:?}");
        }
        // An angle with x: the length reaches that x along the angle.
        let p = keys("<45x1000", true).point(start, 0.0, &pointer);
        assert!(p.is_some_and(|p| close(p, 1100.0, 1100.0)), "{p:?}");
        let p = keys("<45y-1000", true).point(start, 180.0, &pointer);
        assert!(p.is_some_and(|p| close(p, -900.0, -900.0)), "{p:?}");
    }

    #[test]
    fn offsets_that_give_no_point_place_nothing() {
        let start = Some(Point::new(0.0, 0.0));
        let pointer = at(10.0, 10.0);
        for k in [
            "x0y0",       // no move
            "100x300",    // shorter than x
            "100y-101",   // shorter than y
            "<90x100",    // straight up never reaches x
            "<0y100",     // along x never reaches y
            "<45x-100",   // x behind the angle
            "500<45x300", // a length, an angle and x: one too many
            "x-",         // a lone minus
            "x.",         // a lone dot
        ] {
            assert!(keys(k, true).point(start, 0.0, &pointer).is_none(), "{k}");
        }
        // The pointer on the start: x alone keeps y = 0, still a move.
        let p = keys("x100", true).point(start, 0.0, &at(0.0, 0.0));
        assert!(p.is_some_and(|p| close(p, 100.0, 0.0)), "{p:?}");
    }

    #[test]
    fn before_the_first_point_offsets_measure_from_the_origin() {
        let p = keys("x1500y-200", false).point(None, 0.0, &at(5.0, 5.0));
        assert!(p.is_some_and(|p| close(p, 1500.0, -200.0)), "{p:?}");
        let p = keys("x0y0", false).point(None, 0.0, &at(5.0, 5.0));
        assert!(p.is_some_and(|p| close(p, 0.0, 0.0)), "the origin itself");
        // x alone: y from the pointer.
        let p = keys("x700", false).point(None, 0.0, &at(5.0, 6.0));
        assert!(p.is_some_and(|p| close(p, 700.0, 6.0)), "{p:?}");
        // Tab before the first point skips the angle.
        let mut t = Typed::default();
        t.switch_field(false);
        "3".chars().for_each(|c| t.type_char(c, false));
        assert_eq!(t.label(None, Some((1.0, 2.0))), "x 3|  y (2)");
    }

    #[test]
    fn tab_cycles_the_values_and_backspace_goes_back() {
        let mut t = keys("1000", true);
        for want in [
            "1000 < (0.0°)|",
            "1000 < (0.0°)\nx (5)|  y (6)",
            "1000 < (0.0°)\nx (5)  y (6)|",
            "1000| < (0.0°)",
        ] {
            t.switch_field(true);
            assert_eq!(t.label(Some((7.0, 0.0)), Some((5.0, 6.0))), want);
        }
        // y typed, then erased back through x to the length.
        let mut t = keys("x1y2", true);
        (0..2).for_each(|_| t.erase());
        assert_eq!(t.label(None, Some((0.0, 0.0))), "x 1|  y (0)");
        (0..2).for_each(|_| t.erase());
        assert!(t.is_empty(), "{t:?}");
    }

    #[test]
    fn garbage_zero_or_negative_values_place_nothing() {
        let start = Some(Point::new(0.0, 0.0));
        let toward = at(1000.0, 0.0);
        let typed = |keys: &str, angle_keys: &str| {
            let mut t = Typed::default();
            keys.chars().for_each(|c| t.type_char(c, true));
            if !angle_keys.is_empty() {
                t.switch_field(true);
                angle_keys.chars().for_each(|c| t.type_char(c, true));
            }
            t
        };
        for keys in [",", ".", "0", "0.0", "1.2.3", ".."] {
            assert!(
                typed(keys, "").point(start, 0.0, &toward).is_none(),
                "{keys}"
            );
        }
        // A lone minus or dot in the angle is ignored: the pointer's way is used.
        for angle in ["-", "."] {
            let p = typed("500", angle).point(start, 0.0, &toward);
            assert!(p.is_some_and(|p| close(p, 500.0, 0.0)), "{p:?}");
        }
        // A comma is a decimal separator.
        let p = typed("1,5", "").point(start, 0.0, &toward);
        assert!(p.is_some_and(|p| close(p, 1.5, 0.0)), "{p:?}");
        // An angle alone with the pointer behind it gives no point.
        assert!(typed("", "180").point(start, 0.0, &toward).is_none());
        // Letters and a second minus are dropped.
        let mut t = Typed::default();
        "a1b-2".chars().for_each(|c| t.type_char(c, true));
        assert_eq!(t.label(Some((7.0, 45.0)), None), "12| < (45.0°)");
        t.switch_field(true);
        "--3".chars().for_each(|c| t.type_char(c, true));
        assert_eq!(t.label(Some((7.0, 45.0)), None), "12 < -3°|");
        t.clear();
        assert_eq!(
            t.label(Some((7.0, 45.0)), None),
            "7  45.0°",
            "nothing typed"
        );
        assert_eq!(t.label(None, None), "");
    }

    #[test]
    fn before_the_first_point_garbage_places_nothing() {
        let on_guide = Snap {
            along: Some((Point::new(0.0, 0.0), Point::new(1.0, 0.0))),
            ..at(10.0, 0.0)
        };
        let mut t = Typed::default();
        assert!(t.point(None, 0.0, &on_guide).is_none(), "nothing typed");
        ".".chars().for_each(|c| t.type_char(c, false));
        assert!(t.point(None, 0.0, &on_guide).is_none(), "garbage");
    }

    fn typed(length: &str, angle: &str) -> Typed {
        let mut t = Typed::default();
        length.chars().for_each(|c| t.type_char(c, true));
        if !angle.is_empty() {
            t.switch_field(true);
            angle.chars().for_each(|c| t.type_char(c, true));
        }
        t
    }

    #[test]
    fn a_typed_angle_turns_from_any_base_including_half_turns_and_negatives() {
        let start = Some(Point::new(100.0, 200.0));
        let near = at(0.0, 0.0);
        // (base, typed angle, expected direction of travel)
        for (base, angle, x, y) in [
            (180.0, "0", -1.0, 0.0),
            (180.0, "90", 0.0, -1.0),
            (180.0, "-90", 0.0, 1.0),
            (180.0, "180", 1.0, 0.0),
            (-180.0, "0", -1.0, 0.0),
            (-90.0, "90", 1.0, 0.0),
            (-135.0, "-45", -1.0, 0.0),
            (0.0, "-270", 0.0, 1.0),
            (0.0, "360", 1.0, 0.0),
            (90.0, "720", 0.0, 1.0),
            (-180.0, "-180", 1.0, 0.0),
        ] {
            let p = typed("1000", angle).point(start, base, &near);
            let want = (100.0 + 1000.0 * x, 200.0 + 1000.0 * y);
            assert!(
                p.is_some_and(|p| close(p, want.0, want.1)),
                "base {base} angle {angle}: {p:?}"
            );
        }
    }

    #[test]
    fn the_base_is_ignored_without_an_angle_and_applies_to_an_angle_alone() {
        let start = Some(Point::new(0.0, 0.0));
        // A length alone goes towards the pointer whatever the base.
        for base in [0.0, 90.0, 180.0, -37.0] {
            let p = typed("500", "").point(start, base, &at(0.0, 3.0));
            assert!(p.is_some_and(|p| close(p, 0.0, 500.0)), "{base}: {p:?}");
        }
        // An angle alone: the pointer's distance along the turned direction.
        let p = typed("", "90").point(start, 90.0, &at(-700.0, 20.0));
        assert!(p.is_some_and(|p| close(p, -700.0, 0.0)), "{p:?}");
        // Pointer behind the turned direction: no point.
        assert!(
            typed("", "90")
                .point(start, 90.0, &at(700.0, 0.0))
                .is_none()
        );
        // Before the first point the base means nothing.
        let on_guide = Snap {
            along: Some((Point::new(0.0, 0.0), Point::new(1.0, 0.0))),
            ..at(10.0, 0.0)
        };
        let p = typed("300", "").point(None, 123.0, &on_guide);
        assert!(p.is_some_and(|p| close(p, 300.0, 0.0)), "{p:?}");
    }

    #[test]
    fn a_turned_angle_never_gives_a_non_finite_point() {
        let start = Some(Point::new(0.0, 0.0));
        for base in [-180.0, -1e-9, 0.0, 179.999_999, 180.0, 360.0, 1e6] {
            for angle in ["0", "-0", "45", "-45", "999999"] {
                let p = typed("1", angle).point(start, base, &at(1.0, 0.0));
                assert!(
                    p.is_some_and(|p| p.x.is_finite() && p.y.is_finite()),
                    "{base} {angle}"
                );
            }
        }
    }

    #[test]
    fn the_typed_length_is_only_a_positive_number() {
        assert_eq!(Typed::default().length(), None);
        assert_eq!(typed("1500", "").length(), Some(1500.0));
        assert_eq!(typed("1,5", "").length(), Some(1.5));
        for keys in [",", ".", "0", "0.0", "1.2.3"] {
            assert_eq!(typed(keys, "").length(), None, "{keys}");
        }
        // An angle typed does not make a length.
        let mut t = Typed::default();
        t.switch_field(true);
        "45".chars().for_each(|c| t.type_char(c, true));
        assert_eq!(t.length(), None);
        // A minus is only for the angle.
        let mut t = Typed::default();
        "-5".chars().for_each(|c| t.type_char(c, true));
        assert_eq!(t.length(), Some(5.0));
    }
}
