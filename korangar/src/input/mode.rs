use std::sync::Arc;

use cgmath::Vector2;
use korangar_interface::application::MouseInputModeTrait;
use korangar_interface::elements::{Element, ElementCell};
use korangar_networking::InventoryItem;

use crate::graphics::Texture;
use crate::interface::application::InterfaceSettings;
use crate::interface::resource::{ItemSource, SkillSource};
use crate::inventory::Skill;
use crate::loaders::{ResourceMetadata, Sprite};
use crate::world::{Actions, SpriteAnimationState};

#[derive(Default)]
pub enum MouseInputMode {
    MoveItem(ItemSource, InventoryItem<ResourceMetadata>),
    MoveSkill(SkillSource, Skill),
    MoveInterface(usize),
    ResizeInterface(usize),
    DragElement((ElementCell<InterfaceSettings>, usize)),
    ClickInterface,
    RotateCamera,
    Walk(Vector2<usize>),
    #[default]
    None,
}

pub enum Grabbed {
    Texture(Arc<Texture>),
    Action(Arc<Sprite>, Arc<Actions>, SpriteAnimationState),
}

impl MouseInputMode {
    pub fn is_none(&self) -> bool {
        matches!(self, MouseInputMode::None)
    }

    pub fn is_walk(&self) -> bool {
        matches!(self, MouseInputMode::Walk(..))
    }

    pub fn grabbed(&self) -> Option<Grabbed> {
        match self {
            MouseInputMode::MoveItem(_, item) => Some(Grabbed::Texture(item.metadata.texture.clone())),
            MouseInputMode::MoveSkill(_, skill) => Some(Grabbed::Action(
                skill.sprite.clone(),
                skill.actions.clone(),
                skill.animation_state.clone(),
            )),
            _ => None,
        }
    }
}

impl MouseInputModeTrait<InterfaceSettings> for MouseInputMode {
    fn is_none(&self) -> bool {
        matches!(self, MouseInputMode::None)
    }

    fn is_self_dragged(&self, element: &dyn Element<InterfaceSettings>) -> bool {
        matches!(self, Self::DragElement(dragged_element) if std::ptr::eq((&*dragged_element.0.borrow()) as *const _ as *const (), element as *const _ as *const ()))
    }

    fn is_moving_window(&self, window_index: usize) -> bool {
        matches!(self, Self::MoveInterface(index) if *index == window_index)
    }
}
