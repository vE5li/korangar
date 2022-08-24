use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum Dimension {
    Relative(f32),
    Absolute(f32),
    Remaining,
    Flexible,
}

impl Dimension {

    pub fn resolve_width(&self, avalible: f32, remaining: f32, scaling: f32) -> f32 {
        match *self {
            Dimension::Relative(precentage) => avalible / 100.0 * precentage,
            Dimension::Absolute(value) => value * scaling,
            Dimension::Remaining => remaining,
            Dimension::Flexible => panic!("the width may not be flexible"),
        }
    }

    pub fn resolve_height(&self, avalible: Option<f32>, remaining: Option<f32>, scaling: f32) -> Option<f32> {
        match *self {

            Dimension::Relative(precentage) => {
                Some(avalible.expect("trying to get a relative height from a flexible component") / 100.0 * precentage)
            }

            Dimension::Absolute(value) => Some(value * scaling),

            Dimension::Remaining => Some(remaining.expect("trying to get remaining space from a flexible component")),

            Dimension::Flexible => None,
        }
    }

    pub fn is_flexible(&self) -> bool {
        matches!(self, Dimension::Flexible)
    }

    pub fn is_remaining(&self) -> bool {
        matches!(self, Dimension::Remaining)
    }
}
