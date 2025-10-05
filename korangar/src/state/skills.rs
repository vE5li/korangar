use std::sync::Arc;

use hashbrown::HashMap;
use korangar_interface::element::StateElement;
use ragnarok_packets::{AttackRange, JobId, SkillId, SkillInformation, SkillLevel, SkillType};
use rust_state::{Path, PathExt, RustState, Selector};

use crate::loaders::Sprite;
use crate::state::ClientState;
use crate::world::{Actions, Library, SkillListKey, SkillListRequirements, SpriteAnimationState};

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

        let skills = self.skills_path.follow_safe(state);

        skills.iter().find(|skill| skill.skill_id == learnable_skill_id)
    }

    fn follow_mut<'a>(&self, state: &'a mut ClientState) -> Option<&'a mut LearnedSkill> {
        let learnable_skill = self.learnable_skill_path.follow(state)?;
        let learnable_skill_id = learnable_skill.skill_id;

        let skills = self.skills_path.follow_mut_safe(state);

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, RustState, StateElement)]
pub enum SkillAcquisition {
    Job,
    Quest,
    SoulLink,
}

#[derive(Clone, Debug, RustState, StateElement)]
pub struct LearnableSkill {
    pub skill_id: SkillId,
    pub maximum_level: SkillLevel,
    pub file_name: String,
    pub skill_name: String,
    pub can_select_level: bool,
    pub acquisition: SkillAcquisition,
    // TODO: Unhide this
    #[hidden_element]
    pub required_skills: HashMap<SkillId, SkillLevel>,
    #[hidden_element]
    pub required_for_skills: HashMap<SkillId, SkillLevel>,
    // TODO: Unhide this
    #[hidden_element]
    pub sprite: Option<Arc<Sprite>>,
    // TODO: Unhide this
    #[hidden_element]
    pub actions: Option<Arc<Actions>>,
    pub animation_state: SpriteAnimationState,
}

#[derive(Clone, Debug, RustState, StateElement)]
pub struct LearnedSkill {
    pub skill_id: SkillId,
    pub skill_level: SkillLevel,
    pub skill_type: SkillType,
    pub spell_point_cost: u16,
    pub attack_range: AttackRange,
    pub skill_name: String,
    pub upgradable: bool,
}

impl LearnedSkill {
    pub fn new(
        SkillInformation {
            skill_id,
            skill_type,
            skill_level,
            spell_point_cost,
            attack_range,
            skill_name,
            upgradable,
        }: SkillInformation,
    ) -> Self {
        LearnedSkill {
            skill_id,
            skill_level,
            skill_type,
            spell_point_cost,
            attack_range,
            skill_name,
            upgradable: upgradable != 0,
        }
    }
}

#[derive(Debug, Clone, RustState, StateElement)]
pub struct SkillTabLayout {
    pub name: String,
    #[hidden_element]
    pub skills: HashMap<usize, LearnableSkill>,
}

#[derive(Debug, Clone, Default, RustState, StateElement)]
pub struct SkillTreeLayout {
    pub tabs: Vec<SkillTabLayout>,
}

#[derive(Default, RustState, StateElement)]
pub struct SkillTree {
    layout: SkillTreeLayout,
    skills: Vec<LearnedSkill>,
}

impl SkillTree {
    pub fn update_skill(
        &mut self,
        skill_id: SkillId,
        skill_level: SkillLevel,
        spell_point_cost: u16,
        attack_range: AttackRange,
        upgradable: bool,
    ) {
        if let Some(skill) = self.skills.iter_mut().find(|skill| skill.skill_id == skill_id) {
            skill.skill_level = skill_level;
            skill.spell_point_cost = spell_point_cost;
            skill.attack_range = attack_range;
            skill.upgradable = upgradable;
        }
    }

    pub fn remove_skill(&mut self, skill_id: SkillId) {
        self.skills.retain(|skill| skill.skill_id != skill_id);
    }
}

pub fn bring_skill_to_level(
    collected_skill_points: &mut Vec<SkillId>,
    library: &Library,
    learned_skills: &[LearnedSkill],
    job_id: JobId,
    skill_id: SkillId,
    target_skill_level: SkillLevel,
    mut available_skill_points: usize,
) -> usize {
    let skill_requirements = library.get::<SkillListRequirements>(SkillListKey::with_job(job_id, skill_id));

    let current_skill_level = learned_skills
        .iter()
        .find(|skill| skill.skill_id == skill_id)
        .map(|skill| skill.skill_level.0)
        .unwrap_or_default()
        + collected_skill_points
            .iter()
            .filter(|pending_skill_level| **pending_skill_level == skill_id)
            .count() as u16;

    // Early return for met requirements.
    if current_skill_level >= target_skill_level.0 {
        return available_skill_points;
    }

    for (required_skill_id, required_skill_level) in &skill_requirements.required_skills {
        available_skill_points = bring_skill_to_level(
            collected_skill_points,
            library,
            learned_skills,
            job_id,
            *required_skill_id,
            *required_skill_level,
            available_skill_points,
        );

        // Early return when running out of skill points.
        if available_skill_points == 0 {
            return 0;
        }
    }

    let total_points_required = (target_skill_level.0 - current_skill_level) as usize;

    // Attempt to bring the skill to the required level.
    collected_skill_points.extend(std::iter::repeat_n(skill_id, total_points_required.min(available_skill_points)));

    available_skill_points.saturating_sub(total_points_required)
}
