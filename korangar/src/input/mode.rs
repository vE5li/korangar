use std::sync::Arc;

use cgmath::Vector2;
use korangar_interface::MouseMode;
use korangar_interface::element::id::ElementId;
use korangar_networking::InventoryItem;

use crate::graphics::Texture;
use crate::interface::resource::{ItemSource, SkillSource};
use crate::inventory::Skill;
use crate::loaders::Sprite;
use crate::state::ClientState;
use crate::world::{Actions, ResourceMetadata, SpriteAnimationState};

#[derive(Debug)]
pub enum MouseInputMode {
    MoveItem(ItemSource, InventoryItem<ResourceMetadata>),
    MoveSkill(SkillSource, Skill),
    RotateCamera,
    Walk(Vector2<usize>),
}

impl From<MouseInputMode> for MouseMode<ClientState> {
    fn from(mode: MouseInputMode) -> Self {
        MouseMode::Custom { mode }
    }
}

pub enum Grabbed {
    Texture(Arc<Texture>),
    Action(Arc<Sprite>, Arc<Actions>, SpriteAnimationState),
}

pub trait GrabbedExt {
    fn is_rotating_camera(&self) -> bool;

    fn walk_position(&self) -> Option<Vector2<usize>>;

    fn grabbed(&self) -> Option<Grabbed>;
}

impl GrabbedExt for MouseMode<ClientState> {
    fn is_rotating_camera(&self) -> bool {
        matches!(self, MouseMode::Custom {
            mode: MouseInputMode::RotateCamera
        })
    }

    fn walk_position(&self) -> Option<Vector2<usize>> {
        match self {
            MouseMode::Custom {
                mode: MouseInputMode::Walk(position),
            } => Some(*position),
            _ => None,
        }
    }

    fn grabbed(&self) -> Option<Grabbed> {
        match self {
            MouseMode::Custom {
                mode: MouseInputMode::MoveItem(_, item),
            } => item.metadata.texture.as_ref().map(|texture| Grabbed::Texture(texture.clone())),
            MouseMode::Custom {
                mode: MouseInputMode::MoveSkill(_, skill),
            } => Some(Grabbed::Action(
                skill.sprite.clone(),
                skill.actions.clone(),
                skill.animation_state.clone(),
            )),
            _ => None,
        }
    }
}
