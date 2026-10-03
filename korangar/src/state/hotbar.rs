use korangar_interface::element::StateElement;
use korangar_networking::NetworkingSystem;
use ragnarok_packets::handler::PacketCallback;
use ragnarok_packets::{HotbarSlot, HotbarTab, HotkeyData, HotkeyType, SkillId, SkillLevel};
use rust_state::RustState;

use crate::state::skills::LearnableSkill;

fn skill_hotkey_data(skill_id: SkillId, skill_level: SkillLevel) -> HotkeyData {
    HotkeyData {
        hotkey_type: HotkeyType::Skill,
        item_or_skill_id: skill_id.0 as u32,
        quantity_or_skill_level: skill_level.0,
    }
}

fn next_skill_level(current_level: SkillLevel, maximum_level: SkillLevel) -> SkillLevel {
    let current_level = current_level.0.clamp(1, maximum_level.0);

    match current_level >= maximum_level.0 {
        true => SkillLevel(1),
        false => SkillLevel(current_level + 1),
    }
}

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
        let hotkey_data = skill_hotkey_data(skill.skill_id, skill.maximum_level);
        let _ = networking_system.set_hotkey_data(HotbarTab(0), slot, hotkey_data);

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
                .map(|skill| skill_hotkey_data(skill.skill_id, skill.maximum_level))
                .unwrap_or(HotkeyData::UNBOUND);

            let second_data = second
                .as_ref()
                .map(|skill| skill_hotkey_data(skill.skill_id, skill.maximum_level))
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

    /// Cycle a selectable skill's cast level and persist the new value.
    pub fn cycle_skill_level<Callback>(
        &mut self,
        networking_system: &mut NetworkingSystem<Callback>,
        slot: HotbarSlot,
        learned_maximum_level: SkillLevel,
    ) where
        Callback: PacketCallback + Send,
    {
        let Some(skill) = self.skills[slot.0 as usize].as_mut() else {
            return;
        };

        if !skill.can_select_level || learned_maximum_level.0 <= 1 {
            return;
        }

        skill.maximum_level = next_skill_level(skill.maximum_level, learned_maximum_level);

        let hotkey_data = skill_hotkey_data(skill.skill_id, skill.maximum_level);
        let _ = networking_system.set_hotkey_data(HotbarTab(0), slot, hotkey_data);
    }

    pub fn get_skill_in_slot(&self, slot: HotbarSlot) -> &Option<LearnableSkill> {
        &self.skills[slot.0 as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_skill_level_cycles_from_maximum_to_one() {
        assert_eq!(next_skill_level(SkillLevel(5), SkillLevel(5)), SkillLevel(1));
        assert_eq!(next_skill_level(SkillLevel(1), SkillLevel(5)), SkillLevel(2));
    }

    #[test]
    fn selected_skill_level_is_encoded_in_hotkey_data() {
        let hotkey_data = skill_hotkey_data(SkillId(42), SkillLevel(3));

        assert_eq!(hotkey_data.hotkey_type, HotkeyType::Skill);
        assert_eq!(hotkey_data.item_or_skill_id, 42);
        assert_eq!(hotkey_data.quantity_or_skill_level, 3);
    }
}
