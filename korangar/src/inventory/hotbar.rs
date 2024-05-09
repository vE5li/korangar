use std::cell::Ref;

use korangar_interface::state::{PlainRemote, PlainTrackedState, TrackedState, TrackedStateExt};
use korangar_networking::NetworkingSystem;
use ragnarok_packets::handler::PacketCallback;
use ragnarok_packets::{HotbarSlot, HotbarTab, HotkeyData};

use super::Skill;

#[derive(Default)]
pub struct Hotbar {
    skills: PlainTrackedState<[Option<Skill>; 10]>,
}

impl Hotbar {
    /// Set the slot without notifying the map server.
    pub fn set_slot(&mut self, slot: HotbarSlot, skill: Skill) {
        self.skills.mutate(|skills| {
            skills[slot.0 as usize] = Some(skill);
        });
    }

    /// Update the slot and notify the map server.
    pub fn update_slot<Callback>(&mut self, networking_system: &mut NetworkingSystem<Callback>, slot: HotbarSlot, skill: Skill)
    where
        Callback: PacketCallback,
    {
        let _ = networking_system.set_hotkey_data(HotbarTab(0), slot, HotkeyData {
            is_skill: true as u8,
            skill_id: skill.skill_id.0 as u32,
            quantity_or_skill_level: skill.skill_level,
        });

        self.skills.mutate(|skills| {
            skills[slot.0 as usize] = Some(skill);
        });
    }

    /// Swap two slots in the hotbar and notify the map server.
    pub fn swap_slot<Callback>(
        &mut self,
        networking_system: &mut NetworkingSystem<Callback>,
        source_slot: HotbarSlot,
        destination_slot: HotbarSlot,
    ) where
        Callback: PacketCallback,
    {
        if source_slot != destination_slot {
            self.skills.mutate(|skills| {
                let first = skills[source_slot.0 as usize].take();
                let second = skills[destination_slot.0 as usize].take();

                let first_data = first
                    .as_ref()
                    .map(|skill| HotkeyData {
                        is_skill: true as u8,
                        skill_id: skill.skill_id.0 as u32,
                        quantity_or_skill_level: skill.skill_level,
                    })
                    .unwrap_or(HotkeyData::UNBOUND);

                let second_data = second
                    .as_ref()
                    .map(|skill| HotkeyData {
                        is_skill: true as u8,
                        skill_id: skill.skill_id.0 as u32,
                        quantity_or_skill_level: skill.skill_level,
                    })
                    .unwrap_or(HotkeyData::UNBOUND);

                let _ = networking_system.set_hotkey_data(HotbarTab(0), destination_slot, first_data);
                let _ = networking_system.set_hotkey_data(HotbarTab(0), source_slot, second_data);

                skills[source_slot.0 as usize] = second;
                skills[destination_slot.0 as usize] = first;
            })
        }
    }

    /// Clear the slot without notifying the map server.
    pub fn clear_slot(&mut self, slot: HotbarSlot) {
        self.skills.mutate(|skills| {
            skills[slot.0 as usize] = None;
        });
    }

    pub fn get_skill_in_slot(&self, slot: HotbarSlot) -> Ref<Option<Skill>> {
        Ref::map(self.skills.get(), |skills| &skills[slot.0 as usize])
    }

    pub fn get_skills(&self) -> PlainRemote<[Option<Skill>; 10]> {
        self.skills.new_remote()
    }
}
