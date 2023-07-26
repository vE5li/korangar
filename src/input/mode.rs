use cgmath::Vector2;

use crate::graphics::Texture;
use crate::interface::{ElementCell, ItemSource};
use crate::inventory::Item;

#[derive(Default)]
pub enum MouseInputMode {
    MoveItem(ItemSource, Item),
    MoveSkill(usize),
    MoveInterface(usize),
    ResizeInterface(usize),
    DragElement((ElementCell, usize)),
    ClickInterface,
    RotateCamera,
    Walk(Vector2<usize>),
    #[default]
    None,
}

impl MouseInputMode {
    pub fn is_none(&self) -> bool {
        matches!(self, MouseInputMode::None)
    }

    pub fn is_walk(&self) -> bool {
        matches!(self, MouseInputMode::Walk(..))
    }

    pub fn grabbed_texture(&self) -> Option<Texture> {
        match self {
            MouseInputMode::MoveItem(_, item) => Some(item.texture.clone()),
            _ => None,
        }
    }
}
