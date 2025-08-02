use std::sync::Arc;

use korangar_interface::element::StateElement;
use ragnarok_packets::{ClientTick, SkillId, SkillInformation, SkillLevel, SkillType};
use rust_state::RustState;

use crate::loaders::{ActionLoader, Sprite, SpriteLoader};
use crate::world::{Actions, SpriteAnimationState};

#[derive(Clone, Debug, RustState, StateElement)]
pub struct Skill {
    pub skill_id: SkillId,
    pub skill_level: SkillLevel,
    pub skill_type: SkillType,
    pub skill_name: String,
    // TODO: Unhide this
    #[hidden_element]
    pub sprite: Arc<Sprite>,
    // TODO: Unhide this
    #[hidden_element]
    pub actions: Arc<Actions>,
    pub animation_state: SpriteAnimationState,
}

#[derive(Default, RustState, StateElement)]
pub struct SkillTree {
    skills: Vec<Skill>,
}

impl SkillTree {
    pub fn fill(
        &mut self,
        sprite_loader: &SpriteLoader,
        action_loader: &ActionLoader,
        skill_information: Vec<SkillInformation>,
        client_tick: ClientTick,
    ) {
        self.skills = skill_information
            .into_iter()
            .map(|skill_information| {
                let file_path = format!("아이템\\{}", skill_information.skill_name);
                let sprite = sprite_loader.get_or_load(&format!("{file_path}.spr")).unwrap();
                let actions = action_loader.get_or_load(&format!("{file_path}.act")).unwrap();

                Skill {
                    skill_id: skill_information.skill_id,
                    skill_level: skill_information.skill_level,
                    skill_type: skill_information.skill_type,
                    skill_name: skill_information.skill_name,
                    sprite,
                    actions,
                    animation_state: SpriteAnimationState::new(client_tick),
                }
            })
            .collect();
    }

    pub fn find_skill(&self, skill_id: SkillId) -> Option<Skill> {
        self.skills.iter().find(|skill| skill.skill_id == skill_id).cloned()
    }
}
