use korangar_interface::element::StateElement;
use korangar_networking::NetworkingSystem;
use ragnarok_packets::handler::PacketCallback;
use ragnarok_packets::{HotbarSlot, HotbarTab, HotkeyData, HotkeyType};
use rust_state::RustState;

use crate::state::skills::LearnableSkill;

#[derive(Default, RustState, StateElement)]
pub struct Hotbar {
    skills: [Option<LearnableSkill>; 10],
}

impl Hotbar {
    /// Set the slot without notifying the map server.
    pub fn set_slot(&mut self, slot: HotbarSlot, skill: LearnableSkill) {
        self.skills[slot.0 as usize] = Some(skill);
    }

    /// Update the slot and notify the map server.
    pub fn update_slot<Callback>(&mut self, networking_system: &mut NetworkingSystem<Callback>, slot: HotbarSlot, skill: LearnableSkill)
    where
        Callback: PacketCallback + Send,
    {
        let _ = networking_system.set_hotkey_data(HotbarTab(0), slot, HotkeyData {
            hotkey_type: HotkeyType::Skill,
            item_or_skill_id: skill.skill_id.0 as u32,
            quantity_or_skill_level: skill.maximum_level.0,
        });

        self.skills[slot.0 as usize] = Some(skill);
    }

    /// Swap two slots in the hotbar and notify the map server.
    pub fn swap_slot<Callback>(
        &mut self,
        networking_system: &mut NetworkingSystem<Callback>,
        source_slot: HotbarSlot,
        destination_slot: HotbarSlot,
    ) where
        Callback: PacketCallback + Send,
    {
        if source_slot != destination_slot {
            let first = self.skills[source_slot.0 as usize].take();
            let second = self.skills[destination_slot.0 as usize].take();

            let first_data = first
                .as_ref()
                .map(|skill| HotkeyData {
                    hotkey_type: HotkeyType::Skill,
                    item_or_skill_id: skill.skill_id.0 as u32,
                    quantity_or_skill_level: skill.maximum_level.0,
                })
                .unwrap_or(HotkeyData::UNBOUND);

            let second_data = second
                .as_ref()
                .map(|skill| HotkeyData {
                    hotkey_type: HotkeyType::Skill,
                    item_or_skill_id: skill.skill_id.0 as u32,
                    quantity_or_skill_level: skill.maximum_level.0,
                })
                .unwrap_or(HotkeyData::UNBOUND);

            let _ = networking_system.set_hotkey_data(HotbarTab(0), destination_slot, first_data);
            let _ = networking_system.set_hotkey_data(HotbarTab(0), source_slot, second_data);

            self.skills[source_slot.0 as usize] = second;
            self.skills[destination_slot.0 as usize] = first;
        }
    }

    /// Clear the slot without notifying the map server.
    pub fn unset_slot(&mut self, slot: HotbarSlot) {
        self.skills[slot.0 as usize] = None;
    }

    /// Clear the slot and notify the map server.
    pub fn clear_slot<Callback>(&mut self, networking_system: &mut NetworkingSystem<Callback>, slot: HotbarSlot)
    where
        Callback: PacketCallback + Send,
    {
        let _ = networking_system.set_hotkey_data(HotbarTab(0), slot, HotkeyData::UNBOUND);

        self.skills[slot.0 as usize] = None;
    }

    pub fn get_skill_in_slot(&self, slot: HotbarSlot) -> &Option<LearnableSkill> {
        &self.skills[slot.0 as usize]
    }
}
