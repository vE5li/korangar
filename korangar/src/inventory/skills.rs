use std::sync::Arc;

use ragnarok_packets::{ClientTick, SkillId, SkillInformation, SkillLevel, SkillType};

use crate::loaders::{ActionLoader, Sprite, SpriteLoader};
use crate::world::{Actions, SpriteAnimationState};

#[derive(Clone, Debug)]
pub struct Skill {
    pub skill_id: SkillId,
    pub skill_level: SkillLevel,
    pub skill_type: SkillType,
    pub skill_name: String,
    pub sprite: Arc<Sprite>,
    pub actions: Arc<Actions>,
    pub animation_state: SpriteAnimationState,
}

#[derive(Default)]
pub struct SkillTree {
    skills: Vec<Skill>,
}

impl SkillTree {
    pub fn fill(
        &mut self,
        sprite_loader: &SpriteLoader,
        action_loader: &ActionLoader,
        skill_data: Vec<SkillInformation>,
        client_tick: ClientTick,
    ) {
        self.skills = skill_data
            .into_iter()
            .map(|skill_data| {
                let file_path = format!("아이템\\{}", skill_data.skill_name);
                let sprite = sprite_loader.get_or_load(&format!("{file_path}.spr")).unwrap();
                let actions = action_loader.get_or_load(&format!("{file_path}.act")).unwrap();

                Skill {
                    skill_id: skill_data.skill_id,
                    skill_level: skill_data.skill_level,
                    skill_type: skill_data.skill_type,
                    skill_name: skill_data.skill_name,
                    sprite,
                    actions,
                    animation_state: SpriteAnimationState::new(client_tick),
                }
            })
            .collect();
    }

    // pub fn get_skills(&self) -> &[Skill] {
    //     &self.skills
    // }

    pub fn find_skill(&self, skill_id: SkillId) -> Option<Skill> {
        self.skills.iter().find(|skill| skill.skill_id == skill_id).cloned()
    }
}
