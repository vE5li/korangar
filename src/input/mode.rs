use std::sync::Arc;

use cgmath::Vector2;
use vulkano::image::view::ImageView;

use crate::interface::{ElementCell, ItemSource, SkillSource};
use crate::inventory::{Item, Skill};
use crate::loaders::{Actions, AnimationState, Sprite};

#[derive(Default)]
pub enum MouseInputMode {
    MoveItem(ItemSource, Item),
    MoveSkill(SkillSource, Skill),
    MoveInterface(usize),
    ResizeInterface(usize),
    DragElement((ElementCell, usize)),
    ClickInterface,
    RotateCamera,
    Walk(Vector2<usize>),
    #[default]
    None,
}

pub enum Grabbed {
    Texture(Arc<ImageView>),
    Action(Arc<Sprite>, Arc<Actions>, AnimationState),
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
            MouseInputMode::MoveItem(_, item) => Some(Grabbed::Texture(item.texture.clone())),
            MouseInputMode::MoveSkill(_, skill) => Some(Grabbed::Action(
                skill.sprite.clone(),
                skill.actions.clone(),
                skill.animation_state.clone(),
            )),
            _ => None,
        }
    }
}
