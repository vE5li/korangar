use std::sync::Arc;

use hashbrown::HashMap;
use korangar_interface::element::StateElement;
use ragnarok_packets::{ClientTick, SkillId, SkillInformation, SkillLevel, SkillType};
use rust_state::{Path, RustState, Selector};

use crate::loaders::{ActionLoader, Sprite, SpriteLoader};
use crate::state::ClientState;
use crate::world::{Actions, SpriteAnimationState};

pub struct LearnedSkillPath<A, B> {
    learnable_skill_path: A,
    skills_path: B,
}

impl<A, B> LearnedSkillPath<A, B> {
    pub fn new(learnable_skill_path: A, skills_path: B) -> Self {
        Self {
            learnable_skill_path,
            skills_path,
        }
    }
}

impl<A, B> Clone for LearnedSkillPath<A, B>
where
    A: Clone,
    B: Clone,
{
    fn clone(&self) -> Self {
        Self {
            learnable_skill_path: self.learnable_skill_path.clone(),
            skills_path: self.skills_path.clone(),
        }
    }
}

impl<A, B> Copy for LearnedSkillPath<A, B>
where
    A: Copy,
    B: Copy,
{
}

impl<A, B> Path<ClientState, LearnedSkill, false> for LearnedSkillPath<A, B>
where
    A: Path<ClientState, LearnableSkill, false>,
    B: Path<ClientState, Vec<LearnedSkill>>,
{
    fn follow<'a>(&self, state: &'a ClientState) -> Option<&'a LearnedSkill> {
        let learnable_skill = self.learnable_skill_path.follow(state)?;
        let learnable_skill_id = learnable_skill.skill_id;

        // SAFETY:
        // Unwrap is safe here because of the bounds.
        let skills = self.skills_path.follow(state).unwrap();

        skills.iter().find(|skill| skill.skill_id == learnable_skill_id)
    }

    fn follow_mut<'a>(&self, state: &'a mut ClientState) -> Option<&'a mut LearnedSkill> {
        let learnable_skill = self.learnable_skill_path.follow(state)?;
        let learnable_skill_id = learnable_skill.skill_id;

        // SAFETY:
        // Unwrap is safe here because of the bounds.
        let skills = self.skills_path.follow_mut(state).unwrap();

        skills.iter_mut().find(|skill| skill.skill_id == learnable_skill_id)
    }
}

impl<A, B> Selector<ClientState, LearnedSkill, false> for LearnedSkillPath<A, B>
where
    A: Path<ClientState, LearnableSkill, false>,
    B: Path<ClientState, Vec<LearnedSkill>>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a LearnedSkill> {
        self.follow(state)
    }
}

#[derive(Clone, Debug, RustState, StateElement)]
pub struct LearnableSkill {
    pub skill_id: SkillId,
    pub maximum_level: SkillLevel,
    pub file_name: String,
    pub skill_name: String,
    // TODO: Unhide this
    #[hidden_element]
    pub sprite: Arc<Sprite>,
    // TODO: Unhide this
    #[hidden_element]
    pub actions: Arc<Actions>,
    pub animation_state: SpriteAnimationState,
}

impl LearnableSkill {
    pub fn load(
        sprite_loader: &SpriteLoader,
        action_loader: &ActionLoader,
        skill_id: SkillId,
        maximum_level: SkillLevel,
        file_name: String,
        skill_name: String,
        client_tick: ClientTick,
    ) -> Self {
        let file_path = format!("아이템\\{}", file_name);
        let sprite = sprite_loader.get_or_load(&format!("{file_path}.spr")).unwrap();
        let actions = action_loader.get_or_load(&format!("{file_path}.act")).unwrap();

        LearnableSkill {
            skill_id,
            maximum_level,
            file_name,
            skill_name,
            sprite,
            actions,
            animation_state: SpriteAnimationState::new(client_tick),
        }
    }
}

#[derive(Clone, Debug, RustState, StateElement)]
pub struct LearnedSkill {
    pub skill_id: SkillId,
    pub skill_level: SkillLevel,
    pub skill_type: SkillType,
    pub skill_name: String,
}

impl LearnedSkill {
    pub fn new(
        SkillInformation {
            skill_id,
            skill_type,
            skill_level,
            skill_name,
            ..
        }: SkillInformation,
    ) -> Self {
        LearnedSkill {
            skill_id,
            skill_level,
            skill_type,
            skill_name,
        }
    }
}

#[derive(Default, RustState, StateElement)]
pub struct SkillTree {
    #[hidden_element]
    layout: HashMap<usize, LearnableSkill>,
    skills: Vec<LearnedSkill>,
}
