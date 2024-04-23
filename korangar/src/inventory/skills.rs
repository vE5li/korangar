use std::sync::Arc;

use korangar_interface::state::{PlainRemote, PlainTrackedState, TrackedState};
use ragnarok_packets::{ClientTick, SkillId, SkillInformation, SkillLevel, SkillType};

use crate::loaders::{ActionLoader, Actions, AnimationState, GameFileLoader, Sprite, SpriteLoader};

#[derive(Clone, Debug)]
pub struct Skill {
    pub skill_id: SkillId,
    pub skill_level: SkillLevel,
    pub skill_type: SkillType,
    pub skill_name: String,
    pub sprite: Arc<Sprite>,
    pub actions: Arc<Actions>,
    pub animation_state: AnimationState,
}

#[derive(Default)]
pub struct SkillTree {
    skills: PlainTrackedState<Vec<Skill>>,
}

impl SkillTree {
    pub fn fill(
        &mut self,
        game_file_loader: &mut GameFileLoader,
        sprite_loader: &mut SpriteLoader,
        action_loader: &mut ActionLoader,
        skill_data: Vec<SkillInformation>,
    ) {
        let skills = skill_data
            .into_iter()
            .map(|skill_data| {
                let file_path = format!("¾ÆÀÌÅÛ\\{}", skill_data.skill_name);
                let sprite = sprite_loader.get(&format!("{file_path}.spr"), game_file_loader).unwrap();
                let actions = action_loader.get(&format!("{file_path}.act"), game_file_loader).unwrap();

                Skill {
                    skill_id: skill_data.skill_id,
                    skill_level: skill_data.skill_level,
                    skill_type: skill_data.skill_type,
                    skill_name: skill_data.skill_name,
                    sprite,
                    actions,
                    // FIX: give correct client tick
                    animation_state: AnimationState::new(ClientTick(0)),
                }
            })
            .collect();

        self.skills.set(skills);
    }

    pub fn get_skills(&self) -> PlainRemote<Vec<Skill>> {
        self.skills.new_remote()
    }
}
