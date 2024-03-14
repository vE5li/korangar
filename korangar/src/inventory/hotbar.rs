use std::cell::Ref;

use korangar_interface::state::{PlainRemote, PlainTrackedState, TrackedState, TrackedStateExt};

use super::Skill;
use crate::input::HotbarSlot;

#[derive(Default)]
pub struct Hotbar {
    skills: PlainTrackedState<[Option<Skill>; 10]>,
}

impl Hotbar {
    pub fn set_slot(&mut self, skill: Skill, slot: HotbarSlot) {
        self.skills.mutate(|skills| {
            skills[slot.0] = Some(skill);
        });
    }

    pub fn swap_slot(&mut self, source_slot: HotbarSlot, destination_slot: HotbarSlot) {
        if source_slot != destination_slot {
            self.skills.mutate(|skills| {
                let first = skills[source_slot.0].take();
                let second = skills[destination_slot.0].take();

                skills[source_slot.0] = second;
                skills[destination_slot.0] = first;
            });
        }
    }

    /*pub fn clear_slot(&mut self, slot: HotbarSlot) {
        self.skills.with_mut(|skills, changed| {
            skills[slot.0] = None;
            changed();
        });
    }*/

    pub fn get_skill_in_slot(&self, slot: HotbarSlot) -> Ref<Option<Skill>> {
        Ref::map(self.skills.get(), |skills| &skills[slot.0])
    }

    pub fn get_skills(&self) -> PlainRemote<[Option<Skill>; 10]> {
        self.skills.new_remote()
    }
}
