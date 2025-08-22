use korangar_interface::element::StateElement;
use korangar_networking::{CharPacketFactory, LoginPacketFactory, MapPacketFactory, NetworkingSystem};
use ragnarok_packets::handler::PacketCallback;
use ragnarok_packets::{HotbarSlot, HotbarTab, HotkeyData};
use rust_state::RustState;

use super::Skill;

#[derive(Default, RustState, StateElement)]
pub struct Hotbar {
    skills: [Option<Skill>; 10],
}

impl Hotbar {
    /// Set the slot without notifying the map server.
    pub fn set_slot(&mut self, slot: HotbarSlot, skill: Skill) {
        self.skills[slot.0 as usize] = Some(skill);
    }

    /// Update the slot and notify the map server.
    pub fn update_slot<Callback, L, C, M>(
        &mut self,
        networking_system: &mut NetworkingSystem<Callback, L, C, M>,
        slot: HotbarSlot,
        skill: Skill,
    ) where
        Callback: PacketCallback + Send,
        L: LoginPacketFactory,
        C: CharPacketFactory,
        M: MapPacketFactory,
    {
        let _ = networking_system.set_hotkey_data(HotbarTab(0), slot, HotkeyData {
            is_skill: true as u8,
            skill_id: skill.skill_id.0 as u32,
            quantity_or_skill_level: skill.skill_level,
        });

        self.skills[slot.0 as usize] = Some(skill);
    }

    /// Swap two slots in the hotbar and notify the map server.
    pub fn swap_slot<Callback, L, C, M>(
        &mut self,
        networking_system: &mut NetworkingSystem<Callback, L, C, M>,
        source_slot: HotbarSlot,
        destination_slot: HotbarSlot,
    ) where
        Callback: PacketCallback + Send,
        L: LoginPacketFactory,
        C: CharPacketFactory,
        M: MapPacketFactory,
    {
        if source_slot != destination_slot {
            let first = self.skills[source_slot.0 as usize].take();
            let second = self.skills[destination_slot.0 as usize].take();

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

            self.skills[source_slot.0 as usize] = second;
            self.skills[destination_slot.0 as usize] = first;
        }
    }

    /// Clear the slot without notifying the map server.
    pub fn unset_slot(&mut self, slot: HotbarSlot) {
        self.skills[slot.0 as usize] = None;
    }

    /// Clear the slot and notify the map server.
    pub fn clear_slot<Callback, L, C, M>(&mut self, networking_system: &mut NetworkingSystem<Callback, L, C, M>, slot: HotbarSlot)
    where
        Callback: PacketCallback + Send,
        L: LoginPacketFactory,
        C: CharPacketFactory,
        M: MapPacketFactory,
    {
        let _ = networking_system.set_hotkey_data(HotbarTab(0), slot, HotkeyData::UNBOUND);

        self.skills[slot.0 as usize] = None;
    }

    pub fn get_skill_in_slot(&self, slot: HotbarSlot) -> &Option<Skill> {
        &self.skills[slot.0 as usize]
    }
}
