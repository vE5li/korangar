use derive_new::new;

use crate::types::maths::*;

pub type Size = Vector2<f32>;
pub type Position = Vector2<f32>;

#[derive(Copy, Clone, new)]
pub struct PartialSize {
    pub x: f32,
    pub y: Option<f32>,
}

impl PartialSize {

    pub fn finalize(self) -> Vector2<f32> {
        let x = self.x;
        let y = self.y.expect("element cannot have flexible height");
        Vector2::new(x, y)
    }

    pub fn finalize_or(self, y: f32) -> Vector2<f32> {
        let x = self.x;
        let y = self.y.unwrap_or(y);
        Vector2::new(x, y)
    }
}

impl From<Size> for PartialSize {
   
    fn from(size: Size) -> Self { 
        Self { x: size.x, y: Some(size.y) }
    }
}
