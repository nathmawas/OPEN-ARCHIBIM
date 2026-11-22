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
    /// In degrees anticlockwise from the x axis.
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
    /// along the typed angle or towards the pointer; with an angle and no
    /// length, the pointer's distance along that angle. Before the first
    /// point: the typed length along the guide the pointer is on, from
    /// where the guide starts.
    pub fn point(&self, start: Option<Point>, snap: &Snap) -> Option<Point> {
        let number = |s: &str| {
            s.replace(',', ".")
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
        };
        let (length, angle) = (number(&self.length), number(&self.angle));
        let Some(start) = start else {
            let (origin, u) = snap.along?;
            let l = length.filter(|l| *l > 0.0)?;
            return Some(Point::new(origin.x + u.x * l, origin.y + u.y * l));
        };
        let toward = snap.point;
        let (dx, dy) = match angle {
            Some(degrees) => {
                let a = degrees.to_radians();
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
        let p = t.point(Some(Point::new(3000.0, 0.0)), &at(3000.0, 10.0));
        assert!(p.is_some_and(|p| close(p, 3000.0, 4000.0)), "{p:?}");
        assert!(t.point(Some(Point::new(0.0, 0.0)), &at(0.0, 0.0)).is_none());
    }

    #[test]
    fn a_typed_angle_sets_the_direction_with_or_without_a_length() {
        let start = Some(Point::new(0.0, 0.0));
        let mut t = Typed::default();
        "2000<90".chars().for_each(|c| t.type_char(c, true));
        assert_eq!(t.label(Some((500.0, 0.0))), "2000 < 90°|");
        let p = t.point(start, &at(500.0, 0.0));
        assert!(p.is_some_and(|p| close(p, 0.0, 2000.0)), "{p:?}");
        // An angle alone: the pointer gives the length along it.
        t.clear();
        t.switch_field(true);
        "-45".chars().for_each(|c| t.type_char(c, true));
        let p = t.point(start, &at(1000.0, 0.0));
        assert!(p.is_some_and(|p| close(p, 500.0, -500.0)), "{p:?}");
        // Backspace empties the angle, then goes back to the length.
        (0..4).for_each(|_| t.erase());
        assert!(t.is_empty());
    }

    #[test]
    fn before_the_first_point_a_length_runs_along_the_guide() {
        let mut t = Typed::default();
        "1500".chars().for_each(|c| t.type_char(c, false));
        let on_guide = Snap {
            along: Some((Point::new(1000.0, 0.0), Point::new(0.0, 1.0))),
            ..at(1000.0, 800.0)
        };
        let p = t.point(None, &on_guide);
        assert!(p.is_some_and(|p| close(p, 1000.0, 1500.0)), "{p:?}");
        assert!(t.point(None, &at(5.0, 5.0)).is_none(), "off any guide");
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
            assert!(typed(keys, "").point(start, &toward).is_none(), "{keys}");
        }
        // A lone minus or dot in the angle is ignored: the pointer's way is used.
        for angle in ["-", "."] {
            let p = typed("500", angle).point(start, &toward);
            assert!(p.is_some_and(|p| close(p, 500.0, 0.0)), "{p:?}");
        }
        // A comma is a decimal separator.
        let p = typed("1,5", "").point(start, &toward);
        assert!(p.is_some_and(|p| close(p, 1.5, 0.0)), "{p:?}");
        // An angle alone with the pointer behind it gives no point.
        assert!(typed("", "180").point(start, &toward).is_none());
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
        assert!(t.point(None, &on_guide).is_none(), "nothing typed");
        ".".chars().for_each(|c| t.type_char(c, false));
        assert!(t.point(None, &on_guide).is_none(), "garbage");
    }
}
