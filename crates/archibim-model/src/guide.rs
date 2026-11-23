//! Guides the architect places by hand to draw against: lines and circles
//! kept with the project on one story. They are drawing aids, not building
//! elements: never shown in 3D, never counted, never printed.

use crate::{Id, Point};

/// A guide placed by hand on a story.
#[derive(Clone, Debug, PartialEq)]
pub struct Guide {
    /// Permanent identifier.
    pub id: Id,
    /// The story it is placed on.
    pub story: Id,
    /// Its shape in plan.
    pub shape: GuideShape,
}

/// The shape of a guide.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GuideShape {
    /// The infinite line through two distinct points.
    Line {
        /// A point of the line.
        a: Point,
        /// Another point of the line.
        b: Point,
    },
    /// A circle.
    Circle {
        /// Its centre.
        centre: Point,
        /// Its radius, more than zero.
        radius: f64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ModelError, Project, Story};

    fn pt(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn project() -> (Project, Id) {
        let mut p = Project::new();
        let story = p.new_id();
        let added = p.insert_story(Story {
            id: story,
            name: String::new(),
            finished_level: 0.0,
            floor_build_up: 0.0,
        });
        assert!(added.is_ok());
        (p, story)
    }

    fn guide(p: &mut Project, story: Id, shape: GuideShape) -> Guide {
        Guide {
            id: p.new_id(),
            story,
            shape,
        }
    }

    fn line(a: (f64, f64), b: (f64, f64)) -> GuideShape {
        GuideShape::Line {
            a: pt(a.0, a.1),
            b: pt(b.0, b.1),
        }
    }

    fn circle(radius: f64) -> GuideShape {
        GuideShape::Circle {
            centre: pt(0.0, 0.0),
            radius,
        }
    }

    #[test]
    fn valid_guides_are_added_found_and_removed() {
        let (mut p, story) = project();
        let l = guide(&mut p, story, line((0.0, 0.0), (1000.0, 0.0)));
        let c = guide(&mut p, story, circle(500.0));
        assert!(p.insert_guide(l.clone()).is_ok());
        assert!(p.insert_guide(c.clone()).is_ok());
        assert_eq!(p.guide(l.id).ok(), Some(&l));
        assert_eq!(p.remove_guide(l.id).ok(), Some(l.clone()));
        assert_eq!(p.guides, vec![c]);
        assert_eq!(p.remove_guide(l.id), Err(ModelError::NotFound(l.id)));
    }

    #[test]
    fn a_guide_needs_an_existing_story_and_a_new_identifier() {
        let (mut p, story) = project();
        let nowhere = p.new_id();
        let g = guide(&mut p, nowhere, circle(1.0));
        assert_eq!(p.insert_guide(g), Err(ModelError::NotFound(nowhere)));
        let g = guide(&mut p, story, circle(1.0));
        assert!(p.insert_guide(g.clone()).is_ok());
        assert!(p.insert_guide(g).is_err(), "same identifier twice");
        let same_as_story = Guide {
            id: story,
            story,
            shape: circle(1.0),
        };
        assert!(p.insert_guide(same_as_story).is_err(), "a story's id");
        assert_eq!(p.guides.len(), 1);
    }

    #[test]
    fn degenerate_lines_and_circles_are_refused() {
        let (mut p, story) = project();
        let nan = f64::NAN;
        let inf = f64::INFINITY;
        let bad = [
            line((5.0, 5.0), (5.0, 5.0)),
            line((0.0, 0.0), (1e-12, 0.0)),
            line((nan, 0.0), (1.0, 0.0)),
            line((0.0, 0.0), (1.0, nan)),
            line((0.0, 0.0), (inf, 0.0)),
            line((-inf, 0.0), (inf, 0.0)),
            circle(0.0),
            circle(-1.0),
            circle(nan),
            circle(inf),
            GuideShape::Circle {
                centre: pt(nan, 0.0),
                radius: 1.0,
            },
            GuideShape::Circle {
                centre: pt(0.0, inf),
                radius: 1.0,
            },
        ];
        for shape in bad {
            let g = guide(&mut p, story, shape);
            assert!(p.insert_guide(g).is_err(), "{shape:?}");
        }
        assert!(p.guides.is_empty());
    }

    #[test]
    fn extreme_but_valid_guides_are_accepted() {
        let (mut p, story) = project();
        for shape in [
            circle(1e-6),
            circle(1e12),
            line((0.0, 0.0), (1e-6, 0.0)),
            line((-1e200, 0.0), (1e200, 0.0)),
        ] {
            let g = guide(&mut p, story, shape);
            assert!(p.insert_guide(g).is_ok(), "{shape:?}");
        }
    }

    #[test]
    fn replacing_a_guide_checks_the_new_state_and_returns_the_old() {
        let (mut p, story) = project();
        let g = guide(&mut p, story, circle(500.0));
        assert!(p.insert_guide(g.clone()).is_ok());
        let moved = Guide {
            shape: circle(800.0),
            ..g.clone()
        };
        assert_eq!(p.replace_guide(moved.clone()).ok(), Some(g.clone()));
        assert_eq!(p.guide(g.id).ok(), Some(&moved));
        // A refused replacement leaves the guide as it was.
        for shape in [circle(0.0), line((1.0, 1.0), (1.0, 1.0))] {
            let bad = Guide { shape, ..g.clone() };
            assert!(p.replace_guide(bad).is_err());
        }
        let elsewhere = p.new_id();
        let bad = Guide {
            story: elsewhere,
            ..g.clone()
        };
        assert!(p.replace_guide(bad).is_err(), "story must exist");
        assert_eq!(p.guide(g.id).ok(), Some(&moved));
        let unknown = guide(&mut p, story, circle(1.0));
        assert_eq!(
            p.replace_guide(unknown.clone()),
            Err(ModelError::NotFound(unknown.id))
        );
    }

    #[test]
    fn a_story_with_guides_cannot_be_removed_until_they_are() {
        let (mut p, story) = project();
        let g = guide(&mut p, story, circle(500.0));
        assert!(p.insert_guide(g.clone()).is_ok());
        assert!(p.remove_story(story).is_err());
        assert_eq!(p.stories.len(), 1);
        assert!(p.remove_guide(g.id).is_ok());
        assert!(p.remove_story(story).is_ok());
    }

    #[test]
    fn guides_of_another_story_do_not_hold_a_story() {
        let (mut p, story) = project();
        let other = p.new_id();
        let added = p.insert_story(Story {
            id: other,
            name: String::new(),
            finished_level: 2800.0,
            floor_build_up: 0.0,
        });
        assert!(added.is_ok());
        let g = guide(&mut p, other, circle(500.0));
        assert!(p.insert_guide(g).is_ok());
        assert!(p.remove_story(story).is_ok());
    }
}
