use std::sync::Arc;

use cgmath::Vector2;
use korangar_interface::MouseMode;
use korangar_networking::InventoryItem;

use crate::graphics::Texture;
use crate::interface::resource::{ItemSource, SkillSource};
use crate::inventory::Skill;
use crate::loaders::Sprite;
use crate::state::ClientState;
use crate::world::{Actions, ResourceMetadata, SpriteAnimationState};

#[derive(Debug, Clone)]
pub enum MouseInputMode {
    RotateCamera,
    Walk {
        destination: Vector2<usize>,
    },
    MoveItem {
        source: ItemSource,
        item: InventoryItem<ResourceMetadata>,
    },
    MoveSkill {
        source: SkillSource,
        skill: Skill,
    },
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

pub trait MouseModeExt {
    fn is_rotating_camera(&self) -> bool;

    fn walk_destination(&self) -> Option<Vector2<usize>>;

    fn grabbed(&self) -> Option<Grabbed>;
}

impl MouseModeExt for MouseMode<ClientState> {
    fn is_rotating_camera(&self) -> bool {
        matches!(self, MouseMode::Custom {
            mode: MouseInputMode::RotateCamera
        })
    }

    fn walk_destination(&self) -> Option<Vector2<usize>> {
        match self {
            MouseMode::Custom {
                mode: MouseInputMode::Walk { destination },
            } => Some(*destination),
            _ => None,
        }
    }

    fn grabbed(&self) -> Option<Grabbed> {
        match self {
            MouseMode::Custom {
                mode: MouseInputMode::MoveItem { item, .. },
            } => item.metadata.texture.as_ref().map(|texture| Grabbed::Texture(texture.clone())),
            MouseMode::Custom {
                mode: MouseInputMode::MoveSkill { skill, .. },
            } => Some(Grabbed::Action(
                skill.sprite.clone(),
                skill.actions.clone(),
                skill.animation_state.clone(),
            )),
            _ => None,
        }
    }
}
