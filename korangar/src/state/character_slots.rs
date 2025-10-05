use korangar_interface::element::StateElement;
use ragnarok_packets::{CharacterId, CharacterInformation};
use rust_state::{Path, PathExt, RustState, Selector};

use crate::state::ClientState;

#[derive(Default, RustState, StateElement)]
pub struct CharacterSlots {
    slots: Vec<Option<CharacterInformation>>,
}

impl CharacterSlots {
    pub fn set_slot_count(&mut self, slot_count: usize) {
        self.slots.resize(slot_count, None);
    }

    pub fn get_slot_count(&self) -> usize {
        self.slots.len()
    }

    pub fn add_character(&mut self, character_information: CharacterInformation) {
        let Some(slot) = self.slots.get_mut(character_information.character_number as usize) else {
            panic!("attempted to add character to a slot that doesn't exist");
        };

        assert!(slot.is_none(), "attempted to add a character to an occupied slot");

        *slot = Some(character_information);
    }

    pub fn remove_with_id(&mut self, character_id: CharacterId) {
        self.slots.iter_mut().for_each(|slot| {
            if slot
                .as_ref()
                .is_some_and(|character_information| character_information.character_id == character_id)
            {
                *slot = None;
            }
        })
    }

    pub fn with_id(&self, character_id: CharacterId) -> Option<&CharacterInformation> {
        self.slots
            .iter()
            .find(|slot| {
                slot.as_ref()
                    .is_some_and(|character_information| character_information.character_id == character_id)
            })
            .and_then(|slot| slot.as_ref())
    }

    pub fn set_characters(&mut self, characters: Vec<CharacterInformation>) {
        // Clear the character list.
        self.slots.iter_mut().for_each(|slot| *slot = None);

        characters
            .into_iter()
            .for_each(|character_information| self.add_character(character_information));
    }
}

#[derive(Clone, Copy)]
struct SlotPath<P>
where
    P: Copy,
{
    path: P,
    slot: usize,
}

impl<P> Path<ClientState, CharacterInformation, false> for SlotPath<P>
where
    P: Path<ClientState, CharacterSlots>,
{
    fn follow<'a>(&self, state: &'a ClientState) -> Option<&'a CharacterInformation> {
        self.path.follow_safe(state).slots.get(self.slot).and_then(|slot| slot.as_ref())
    }

    fn follow_mut<'a>(&self, state: &'a mut ClientState) -> Option<&'a mut CharacterInformation> {
        self.path
            .follow_mut_safe(state)
            .slots
            .get_mut(self.slot)
            .and_then(|slot| slot.as_mut())
    }
}

impl<P> Selector<ClientState, CharacterInformation, false> for SlotPath<P>
where
    P: Path<ClientState, CharacterSlots>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a CharacterInformation> {
        self.follow(state)
    }
}

pub trait CharacterSlotsExt {
    fn in_slot(self, slot: usize) -> impl Path<ClientState, CharacterInformation, false>;
}

impl<P> CharacterSlotsExt for P
where
    P: Path<ClientState, CharacterSlots>,
{
    fn in_slot(self, slot: usize) -> impl Path<ClientState, CharacterInformation, false> {
        SlotPath { path: self, slot }
    }
}
