//! Lengths and angles typed at the keyboard while drawing, shared by every
//! drawing tool: the next point at a typed distance from the last one, along
//! a typed angle or towards the pointer, or, before the first point, at a
//! typed distance along the guide the pointer is on.

use archibim_model::Point;

use crate::Snap;

/// What was typed for the next point.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Typed {
    length: String,
    /// In degrees anticlockwise from the previous segment carried on, or
    /// from the x axis for the first segment: 0 goes straight on, 90
    /// turns left, -90 turns right.
    angle: String,
    /// Typing goes to the angle rather than the length.
    on_angle: bool,
}

impl Typed {
    /// Whether nothing was typed.
    pub fn is_empty(&self) -> bool {
        self.length.is_empty() && self.angle.is_empty() && !self.on_angle
    }

    /// What to show next to the pointer, given `live`, the length and the
    /// angle from the last point if there is one: with nothing typed, the
    /// live values; else each typed value, the live ones in brackets, and
    /// a bar after the value being typed.
    pub fn label(&self, live: Option<(f64, f64)>) -> String {
        let bar = |on: bool| if on { "|" } else { "" };
        let length = match (self.length.is_empty(), live) {
            (false, _) => self.length.clone(),
            (true, Some((l, _))) => format!("({l:.0})"),
            (true, None) => String::new(),
        };
        match live {
            None if self.is_empty() => String::new(),
            None => format!("{length}{}", bar(!self.on_angle)),
            Some((l, a)) if self.is_empty() => format!("{l:.0}  {a:.1}°"),
            Some((_, a)) => {
                let angle = if self.angle.is_empty() {
                    format!("({a:.1}°)")
                } else {
                    format!("{}°", self.angle)
                };
                format!(
                    "{length}{} < {angle}{}",
                    bar(!self.on_angle),
                    bar(self.on_angle)
                )
            }
        }
    }

    /// The typed length, if it is a length.
    pub fn length(&self) -> Option<f64> {
        number(&self.length).filter(|l| *l > 0.0)
    }

    /// Tab or `<`: typing goes to the angle, or back to the length. An
    /// angle needs a point to turn around.
    pub fn switch_field(&mut self, drawing: bool) {
        if drawing {
            self.on_angle = !self.on_angle;
        }
    }

    /// Forgets what was typed.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// A typed character: digits and a decimal separator build the length,
    /// or the angle after `<`, which may be negative.
    pub fn type_char(&mut self, c: char, drawing: bool) {
        match c {
            '<' => self.switch_field(drawing),
            '-' if self.on_angle && self.angle.is_empty() => self.angle.push(c),
            c if c.is_ascii_digit() || c == '.' || c == ',' => {
                if self.on_angle {
                    self.angle.push(c);
                } else {
                    self.length.push(c);
                }
            }
            _ => {}
        }
    }

    /// Backspace: erases the last typed character.
    pub fn erase(&mut self) {
        if !self.on_angle {
            self.length.pop();
        } else if self.angle.pop().is_none() {
            self.on_angle = false;
        }
    }

    /// The point placed with what was typed, or nothing if what was typed
    /// does not give one. From the last point `start`: the typed length
    /// along the typed angle, turned from `base`, the direction in degrees
    /// of the previous segment (0 with none), or towards the pointer; with
    /// an angle and no length, the pointer's distance along that angle.
    /// Before the first point: the typed length along the guide the
    /// pointer is on, from where the guide starts.
    pub fn point(&self, start: Option<Point>, base: f64, snap: &Snap) -> Option<Point> {
        let (length, angle) = (number(&self.length), number(&self.angle));
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

/// A typed number, a comma or a dot as the decimal separator.
fn number(s: &str) -> Option<f64> {
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
        assert_eq!(t.label(Some((500.0, 0.0))), "2000 < 90°|");
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
        // No angle without a point to turn around.
        t.switch_field(false);
        "90".chars().for_each(|c| t.type_char(c, false));
        assert_eq!(t.label(None), "150090|");
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
        assert_eq!(t.label(Some((7.0, 45.0))), "12| < (45.0°)");
        t.switch_field(true);
        "--3".chars().for_each(|c| t.type_char(c, true));
        assert_eq!(t.label(Some((7.0, 45.0))), "12 < -3°|");
        t.clear();
        assert_eq!(t.label(Some((7.0, 45.0))), "7  45.0°", "nothing typed");
        assert_eq!(t.label(None), "");
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
