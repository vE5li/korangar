use std::cell::UnsafeCell;

use korangar_components::character_slot_preview;
use korangar_interface::element::id::ElementIdGenerator;
use korangar_interface::element::store::ElementStore;
use korangar_interface::element::{Element, ElementSet, ResolverSet};
use korangar_interface::layout::Layout;
use korangar_interface::window::{CustomWindow, WindowTrait};
use rust_state::{Context, Path};

use crate::character_slots::{CharacterSlots, CharacterSlotsExt};
use crate::interface::components::character_slot_preview::CharacterSlotPreviewHandler;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::{WindowCache, WindowClass};
use crate::state::{ClientState, ClientThemeType};

pub struct CharacterSelectionWindow<C, M> {
    character_slots: C,
    switch_request: M,
}

impl<C, M> CharacterSelectionWindow<C, M> {
    pub fn new(characters: C, switch_request: M) -> Self {
        Self {
            character_slots: characters,
            switch_request,
        }
    }
}

impl<C, M> CustomWindow<ClientState> for CharacterSelectionWindow<C, M>
where
    C: Path<ClientState, CharacterSlots>,
    M: Path<ClientState, Option<usize>>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::CharacterSelection)
    }

    fn to_window<'a>(
        self,
        state: &Context<ClientState>,
        window_cache: &WindowCache,
        available_space: ScreenSize,
    ) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        struct CharacterWrapper<C, M> {
            character_slots: C,
            switch_request: M,
            item_boxes: UnsafeCell<Vec<Box<dyn Element<ClientState>>>>,
        }

        impl<C, M> CharacterWrapper<C, M>
        where
            C: Path<ClientState, CharacterSlots>,
            M: Path<ClientState, Option<usize>>,
        {
            fn new(character_slots: C, switch_request: M) -> Self {
                Self {
                    character_slots,
                    switch_request,
                    item_boxes: UnsafeCell::new(Vec::new()),
                }
            }

            fn correct_element_size(&self, state: &Context<ClientState>) {
                let character_slots = state.get(&self.character_slots);
                let item_boxes = unsafe { &mut *self.item_boxes.get() };
                let slot_count = character_slots.get_slot_count();

                // FIX: Very broken check
                if item_boxes.len() != slot_count / 5 {
                    item_boxes.clear();

                    for row in 0..slot_count / 5 {
                        let slot = row * 5;
                        let path = self.character_slots;

                        item_boxes.push(Box::new(split! {
                            children: (
                                character_slot_preview! {
                                    character_information: path.in_slot(slot),
                                    switch_request: self.switch_request,
                                    click_handler: CharacterSlotPreviewHandler::new(self.switch_request, path.in_slot(slot), slot),
                                    slot: slot,
                                },
                                character_slot_preview! {
                                    character_information: path.in_slot(slot + 1),
                                    switch_request: self.switch_request,
                                    click_handler: CharacterSlotPreviewHandler::new(self.switch_request, path.in_slot(slot + 1), slot + 1),
                                    slot: slot + 1,
                                },
                                character_slot_preview! {
                                    character_information: path.in_slot(slot + 2),
                                    switch_request: self.switch_request,
                                    click_handler: CharacterSlotPreviewHandler::new(self.switch_request, path.in_slot(slot + 2), slot + 2),
                                    slot: slot + 2,
                                },
                                character_slot_preview! {
                                    character_information: path.in_slot(slot + 3),
                                    switch_request: self.switch_request,
                                    click_handler: CharacterSlotPreviewHandler::new(self.switch_request, path.in_slot(slot + 3), slot + 3),
                                    slot: slot + 3,
                                },
                                character_slot_preview! {
                                    character_information: path.in_slot(slot + 4),
                                    switch_request: self.switch_request,
                                    click_handler: CharacterSlotPreviewHandler::new(self.switch_request, path.in_slot(slot + 4), slot + 4),
                                    slot: slot + 4,
                                },
                            )
                        }));
                    }
                }
            }
        }

        impl<C, M> ElementSet<ClientState> for CharacterWrapper<C, M>
        where
            C: Path<ClientState, CharacterSlots>,
            M: Path<ClientState, Option<usize>>,
        {
            fn get_element_count(&self) -> usize {
                unimplemented!()
            }

            fn get_height(
                &self,
                state: &Context<ClientState>,
                store: &ElementStore,
                generator: &mut ElementIdGenerator,
                mut resolver_set: impl ResolverSet,
            ) {
                self.correct_element_size(state);
                let item_boxes = unsafe { &mut *self.item_boxes.get() };

                // FIX: Make this right. Maybe with_derived should expect a resolver set as well
                resolver_set.with_index(0, |resolver| {
                    resolver.with_derived(2.0, 4.0, |resolver| {
                        for (index, item_box) in item_boxes.iter().enumerate() {
                            item_box.get_height(state, store.child_store(index as u64, generator), generator, resolver);
                        }
                    });
                });
            }

            fn create_layout<'a>(
                &'a self,
                state: &'a Context<ClientState>,
                store: &'a ElementStore,
                generator: &mut ElementIdGenerator,
                mut resolver_set: impl ResolverSet,
                layout: &mut Layout<'a, ClientState>,
            ) {
                self.correct_element_size(state);
                let item_boxes = unsafe { &mut *self.item_boxes.get() };

                // FIX: Make this right. Maybe with_derived should expect a resolver set as well
                resolver_set.with_index(0, |resolver| {
                    resolver.with_derived(2.0, 4.0, |resolver| {
                        // TODO: Very much temp
                        layout.push_layer();

                        for (index, item_box) in item_boxes.iter().enumerate() {
                            item_box.create_layout(state, store.child_store(index as u64, generator), generator, resolver, layout);
                        }

                        // TODO: Very much temp
                        layout.pop_layer();
                    });
                });
            }
        }

        window! {
            title: "Select Character",
            class: Self::window_class(),
            theme: ClientThemeType::Menu,
            elements: CharacterWrapper::new(self.character_slots, self.switch_request),
        }
    }
}
