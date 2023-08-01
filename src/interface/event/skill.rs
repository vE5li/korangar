use crate::input::HotbarSlot;
use crate::inventory::Skill;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillSource {
    SkillTree,
    Hotbar { slot: HotbarSlot },
}

#[derive(Clone, Debug)]
pub struct SkillMove {
    pub source: SkillSource,
    pub destination: SkillSource,
    pub skill: Skill,
}
