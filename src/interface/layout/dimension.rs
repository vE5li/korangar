use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum Dimension {
    Relative(f32),
    Absolute(f32),
    Remaining,
    Flexible,
    Super,
}

impl Dimension {
    fn resolve_width_inner(&self, available: f32, remaining: f32, parent: Option<f32>, scaling: f32) -> Result<f32, &'static str> {
        match *self {
            Dimension::Relative(precentage) => Ok(available / 100.0 * precentage),
            Dimension::Absolute(value) => Ok(value * scaling),
            Dimension::Remaining => Ok(remaining),
            Dimension::Super => parent.ok_or("trying to get parent size without a parent"),
            Dimension::Flexible => Err("the width may not be flexible"),
        }
    }

    pub fn resolve_width(&self, available: f32, remaining: f32, parent: Option<f32>, scaling: f32) -> f32 {
        self.resolve_width_inner(available, remaining, parent, scaling).unwrap()
    }

    pub fn try_resolve_width(&self, available: f32, remaining: f32, parent: Option<f32>, scaling: f32) -> Option<f32> {
        self.resolve_width_inner(available, remaining, parent, scaling).ok()
    }

    fn resolve_height_inner(
        &self,
        available: Option<f32>,
        remaining: Option<f32>,
        parent: Option<f32>,
        scaling: f32,
    ) -> Result<Option<f32>, &'static str> {
        match *self {
            Dimension::Relative(precentage) => Ok(Some(
                available.ok_or("trying to get a relative height from a flexible component")? / 100.0 * precentage,
            )),
            Dimension::Absolute(value) => Ok(Some(value * scaling)),
            Dimension::Remaining => Ok(Some(
                remaining.ok_or("trying to get remaining space from a flexible component")?,
            )),
            Dimension::Super => Ok(Some(parent.ok_or("trying to get parent size without a parent")?)),
            Dimension::Flexible => Ok(None),
        }
    }

    pub fn resolve_height(&self, available: Option<f32>, remaining: Option<f32>, parent: Option<f32>, scaling: f32) -> Option<f32> {
        self.resolve_height_inner(available, remaining, parent, scaling).unwrap()
    }

    pub fn try_resolve_height(&self, available: Option<f32>, remaining: Option<f32>, parent: Option<f32>, scaling: f32) -> Option<f32> {
        self.resolve_height_inner(available, remaining, parent, scaling).ok().flatten()
    }

    pub fn is_flexible(&self) -> bool {
        matches!(self, Dimension::Flexible)
    }

    pub fn is_remaining(&self) -> bool {
        matches!(self, Dimension::Remaining)
    }

    pub fn is_absolute(&self) -> bool {
        matches!(self, Dimension::Absolute(_))
    }
}
