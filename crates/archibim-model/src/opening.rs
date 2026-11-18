use crate::{Id, ModelError};

/// The side the hinges are on, seen from the side the opening is drawn from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hand {
    /// Hinges on the left.
    Left,
    /// Hinges on the right.
    Right,
}

/// How one part of an opening opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    /// Does not open.
    Fixed,
    /// Hinged on one side.
    SideHung(Hand),
    /// Hinged on one side, and tilts from the bottom.
    TiltAndTurn(Hand),
    /// Hinged at the top.
    TopHung,
    /// Turns around a central axis.
    Pivot,
    /// Slides along the wall.
    Sliding,
}

/// One part of an opening: a leaf, a sash or a fixed light, side by side with
/// the other parts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Part {
    /// Clear width of the part.
    pub width: f64,
    /// How it opens.
    pub operation: Operation,
}

/// Performance targets of an exterior opening.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Performance {
    /// Thermal transmittance, in W/m²K.
    pub thermal_uw: Option<f64>,
    /// Airtightness class.
    pub airtightness_class: Option<u8>,
    /// Acoustic insulation, in dB.
    pub acoustic_db: Option<f64>,
}

/// Interior and exterior openings do not have the same performance targets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OpeningKind {
    /// A room door, an interior glazed screen: no performance targets.
    Interior,
    /// A window, an entrance door.
    Exterior(Performance),
}

/// Which width stays fixed when the frame or the composition changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NominalSize {
    /// The clear width of the parts stays fixed; the hole in the wall follows.
    ClearOpening,
    /// The hole in the wall stays fixed; the parts follow.
    WallOpening,
}

/// A door or a window, composed of parts side by side.
#[derive(Clone, Debug, PartialEq)]
pub struct Opening {
    /// Permanent identifier.
    pub id: Id,
    /// The host wall.
    pub wall: Id,
    /// Interior or exterior.
    pub kind: OpeningKind,
    /// Distance along the wall, from its start to the left side of the hole.
    pub position: f64,
    /// Sill height above the finished level of the wall's story.
    pub sill: f64,
    /// Height of the hole.
    pub height: f64,
    /// Width of the frame, and of the mullions between parts.
    pub frame: f64,
    /// The parts, from left to right.
    pub parts: Vec<Part>,
    /// Which width stays fixed.
    pub nominal: NominalSize,
}

impl Opening {
    /// Sum of the clear widths of the parts.
    pub fn clear_width(&self) -> f64 {
        self.parts.iter().map(|p| p.width).sum()
    }

    /// Width of the hole in the wall: the parts, the mullions between them
    /// and the frame on both sides.
    pub fn wall_opening_width(&self) -> f64 {
        self.clear_width() + self.frame * (self.parts.len() + 1) as f64
    }

    /// Adds a part on the left or on the right.
    ///
    /// With a fixed clear opening, the hole grows. With a fixed wall opening,
    /// the neighbouring part shrinks: its index is returned so that the user
    /// can be told.
    pub fn add_part(&mut self, part: Part, at_left: bool) -> Result<Option<usize>, ModelError> {
        let neighbour = match self.nominal {
            NominalSize::ClearOpening => None,
            NominalSize::WallOpening => {
                let index = if at_left {
                    0
                } else {
                    self.parts.len().saturating_sub(1)
                };
                let taken = part.width + self.frame;
                let shrunk = self
                    .parts
                    .get_mut(index)
                    .ok_or(ModelError::Invalid("no part to take the width from"))?;
                if shrunk.width <= taken {
                    return Err(ModelError::Invalid("the neighbouring part is too narrow"));
                }
                shrunk.width -= taken;
                Some(if at_left { index + 1 } else { index })
            }
        };
        if at_left {
            self.parts.insert(0, part);
        } else {
            self.parts.push(part);
        }
        Ok(neighbour)
    }

    /// Changes the frame width, keeping the nominal size.
    pub fn set_frame(&mut self, frame: f64) -> Result<(), ModelError> {
        if self.nominal == NominalSize::WallOpening && !self.parts.is_empty() {
            let n = self.parts.len() as f64;
            let per_part = (frame - self.frame) * (n + 1.0) / n;
            if self.parts.iter().any(|p| p.width <= per_part) {
                return Err(ModelError::Invalid(
                    "the frame leaves no room for the parts",
                ));
            }
            for p in &mut self.parts {
                p.width -= per_part;
            }
        }
        self.frame = frame;
        Ok(())
    }
}
