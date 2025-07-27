use std::any::Any;
use std::cell::UnsafeCell;

use korangar_components::character_slot_preview;
use korangar_interface::element::id::ElementIdGenerator;
use korangar_interface::element::store::ElementStore;
use korangar_interface::element::{DefaultLayoutInfo, DefaultLayoutInfoSet, Element, ElementSet, ResolverSet};
use korangar_interface::layout::{Layout, Resolver};
use korangar_interface::window::{CustomWindow, WindowTrait};
use rust_state::{Context, Path};

use crate::character_slots::{CharacterSlots, CharacterSlotsExt};
use crate::interface::components::character_slot_preview::CharacterSlotPreviewHandler;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::{WindowCache, WindowClass};
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

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

    fn to_window<'a>(self) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        type RowLayoutInfo = (
            DefaultLayoutInfo,
            DefaultLayoutInfo,
            DefaultLayoutInfo,
            DefaultLayoutInfo,
            DefaultLayoutInfo,
        );

        struct CharacterWrapper<C, M> {
            character_slots: C,
            switch_request: M,
            item_boxes: Vec<Box<dyn Element<ClientState, LayoutInfo = RowLayoutInfo>>>,
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
                    item_boxes: Vec::new(),
                }
            }

            fn correct_element_size(&mut self, state: &Context<ClientState>) {
                let character_slots = state.get(&self.character_slots);
                let slot_count = character_slots.get_slot_count();

                // FIX: Very broken check
                if self.item_boxes.len() != slot_count / 5 {
                    self.item_boxes.clear();

                    for row in 0..slot_count / 5 {
                        let slot = row * 5;
                        let path = self.character_slots;

                        self.item_boxes.push(Box::new(split! {
                            children: (
                                character_slot_preview! {
                                    character_information: path.in_slot(slot),
                                    switch_request: self.switch_request,
                                    click_handler: CharacterSlotPreviewHandler::new(self.switch_request, slot),
                                    overlay_handler: crate::interface::components::character_slot_preview::OverlayHandler::new(slot, self.switch_request, path.in_slot(slot)),
                                    slot: slot,
                                },
                                character_slot_preview! {
                                    character_information: path.in_slot(slot + 1),
                                    switch_request: self.switch_request,
                                    click_handler: CharacterSlotPreviewHandler::new(self.switch_request, slot + 1),
                                    overlay_handler: crate::interface::components::character_slot_preview::OverlayHandler::new(slot + 1, self.switch_request, path.in_slot(slot + 1)),
                                    slot: slot + 1,
                                },
                                character_slot_preview! {
                                    character_information: path.in_slot(slot + 2),
                                    switch_request: self.switch_request,
                                    click_handler: CharacterSlotPreviewHandler::new(self.switch_request, slot + 2),
                                    overlay_handler: crate::interface::components::character_slot_preview::OverlayHandler::new(slot + 2, self.switch_request, path.in_slot(slot + 2)),
                                    slot: slot + 2,
                                },
                                character_slot_preview! {
                                    character_information: path.in_slot(slot + 3),
                                    switch_request: self.switch_request,
                                    click_handler: CharacterSlotPreviewHandler::new(self.switch_request, slot + 3),
                                    overlay_handler: crate::interface::components::character_slot_preview::OverlayHandler::new(slot + 3, self.switch_request, path.in_slot(slot + 3)),
                                    slot: slot + 3,
                                },
                                character_slot_preview! {
                                    character_information: path.in_slot(slot + 4),
                                    switch_request: self.switch_request,
                                    click_handler: CharacterSlotPreviewHandler::new(self.switch_request, slot + 4),
                                    overlay_handler: crate::interface::components::character_slot_preview::OverlayHandler::new(slot + 4, self.switch_request, path.in_slot(slot + 4)),
                                    slot: slot + 4,
                                },
                            )
                        }));
                    }
                }
            }
        }

        impl<C, M> Element<ClientState> for CharacterWrapper<C, M>
        where
            C: Path<ClientState, CharacterSlots>,
            M: Path<ClientState, Option<usize>>,
        {
            type LayoutInfo = Vec<RowLayoutInfo>;

            fn create_layout_info(
                &mut self,
                state: &Context<ClientState>,
                store: &mut ElementStore,
                generator: &mut ElementIdGenerator,
                resolver: &mut Resolver,
            ) -> Self::LayoutInfo {
                self.correct_element_size(state);
                let (area, layout_info) = resolver.with_derived(10.0, 0.0, |resolver| {
                    self.item_boxes
                        .iter_mut()
                        .enumerate()
                        .map(|(index, item_box)| {
                            item_box.create_layout_info(
                                state,
                                store.get_or_create_child_store(index as u64, generator),
                                generator,
                                resolver,
                            )
                        })
                        .collect()
                });

                layout_info
            }

            fn layout_element<'a>(
                &'a self,
                state: &'a Context<ClientState>,
                store: &'a ElementStore,
                layout_info: &'a Self::LayoutInfo,
                layout: &mut Layout<'a, ClientState>,
            ) {
                layout.with_layer(|layout| {
                    for (index, item_box) in self.item_boxes.iter().enumerate() {
                        item_box.layout_element(state, store.child_store(index as u64), &layout_info[index], layout);
                    }
                });
            }
        }

        window! {
            title: "Select Character",
            class: Self::window_class(),
            theme: InterfaceThemeType::Menu,
            minimum_width: 900.0,
            maximum_width: 900.0,
            elements: (
                CharacterWrapper::new(self.character_slots, self.switch_request),
            ),
        }
    }
}
