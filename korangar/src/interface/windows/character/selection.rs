use std::cell::UnsafeCell;
use std::cmp::Ordering;

use korangar_interface::element::id::ElementIdGenerator;
use korangar_interface::element::store::ElementStore;
use korangar_interface::element::{Element, ElementSet};
use korangar_interface::event::EventQueue;
use korangar_interface::layout::{Layout, Resolver};
use korangar_interface::window::{CustomWindow, PrototypeWindow, Window, WindowTrait};
use ragnarok_packets::{CharacterInformation, CharacterServerInformation};
use rust_state::{Context, Path, Selector};

use crate::input::UserEvent;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::{WindowCache, WindowClass};
use crate::state::{ClientState, ClientThemeType};

pub struct CharacterSelectionWindow<C, M, S> {
    characters: C,
    move_request: M,
    slot_count: S,
}

impl<C, M, S> CharacterSelectionWindow<C, M, S> {
    pub fn new(characters: C, move_request: M, slot_count: S) -> Self {
        Self {
            characters,
            move_request,
            slot_count,
        }
    }
}

impl<C, M, S> CustomWindow<ClientState> for CharacterSelectionWindow<C, M, S>
where
    C: Path<ClientState, Vec<CharacterInformation>>,
    M: Path<ClientState, Option<usize>>,
    S: Path<ClientState, usize>,
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
        use rust_state::{ManuallyAssertExt, VecIndexExt};

        // TODO: Remove, just temporary
        struct ButtonText<P> {
            path: P,
            slot: usize,
            final_string: UnsafeCell<String>,
        }

        impl<P> Selector<ClientState, String> for ButtonText<P>
        where
            P: Path<ClientState, Vec<CharacterInformation>>,
        {
            fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a String> {
                let name = self
                    .path
                    .follow(state)
                    .unwrap()
                    .iter()
                    .find(|character| character.character_number as usize == self.slot)
                    .map(|character| character.name.clone())
                    .unwrap_or("<Free>".to_owned());
                let slot = self.slot;

                unsafe {
                    *self.final_string.get() = format!("#{slot}: {name}");
                }

                unsafe { Some(self.final_string.as_ref_unchecked()) }
            }
        }

        struct CharacterWrapper<C, M, S> {
            characters: C,
            move_request: M,
            slot_count: S,
            item_boxes: UnsafeCell<Vec<Box<dyn Element<ClientState>>>>,
        }

        impl<C, M, S> CharacterWrapper<C, M, S>
        where
            C: Path<ClientState, Vec<CharacterInformation>>,
            M: Path<ClientState, Option<usize>>,
            S: Path<ClientState, usize>,
        {
            fn new(characters: C, move_request: M, slot_count: S) -> Self {
                Self {
                    characters,
                    move_request,
                    slot_count,
                    item_boxes: UnsafeCell::new(Vec::new()),
                }
            }

            fn correct_element_size(&self, state: &Context<ClientState>) {
                let character_information = state.get(&self.characters);
                let item_boxes = unsafe { &mut *self.item_boxes.get() };
                let slot_count = *state.get(&self.slot_count);

                match item_boxes.len().cmp(&slot_count) {
                    Ordering::Greater => {
                        // Delete excess elements.
                        item_boxes.truncate(character_information.len());
                    }
                    Ordering::Less => {
                        // Add new elements.
                        for slot in item_boxes.len()..slot_count {
                            let path = self.characters;

                            item_boxes.push(Box::new(button! {
                                text: ButtonText {
                                    path,
                                    slot,
                                    final_string: UnsafeCell::new(String::new()),
                                },
                                event: move |state: &Context<ClientState>, queue: &mut EventQueue<ClientState>| {
                                    let character_information = state.get(&path).clone();
                                    queue.queue(UserEvent::SelectCharacter { slot });
                                },
                            }));
                        }
                    }
                    Ordering::Equal => {}
                }
            }
        }

        impl<C, M, S> ElementSet<ClientState> for CharacterWrapper<C, M, S>
        where
            C: Path<ClientState, Vec<CharacterInformation>>,
            M: Path<ClientState, Option<usize>>,
            S: Path<ClientState, usize>,
        {
            fn get_element_count(&self) -> usize {
                unimplemented!()
            }

            fn get_height(
                &self,
                state: &Context<ClientState>,
                store: &ElementStore,
                generator: &mut ElementIdGenerator,
                resolver: &mut Resolver,
            ) {
                self.correct_element_size(state);
                let item_boxes = unsafe { &mut *self.item_boxes.get() };

                resolver.with_derived(2.0, 4.0, |resolver| {
                    for (index, item_box) in item_boxes.iter().enumerate() {
                        item_box.get_height(state, store.child_store(index as u64, generator), generator, resolver);
                    }
                });
            }

            fn create_layout<'a>(
                &'a self,
                state: &'a Context<ClientState>,
                store: &'a ElementStore,
                generator: &mut ElementIdGenerator,
                resolver: &mut Resolver,
                layout: &mut Layout<'a, ClientState>,
            ) {
                self.correct_element_size(state);
                let item_boxes = unsafe { &mut *self.item_boxes.get() };

                resolver.with_derived(2.0, 4.0, |resolver| {
                    // TODO: Very much temp
                    layout.push_layer();

                    for (index, item_box) in item_boxes.iter().enumerate() {
                        item_box.create_layout(state, store.child_store(index as u64, generator), generator, resolver, layout);
                    }

                    // TODO: Very much temp
                    layout.pop_layer();
                });
            }
        }

        window! {
            title: "Select Character",
            class: Some(WindowClass::CharacterSelection),
            theme: ClientThemeType::Menu,
            elements: CharacterWrapper::new(self.characters, self.move_request, self.slot_count),
        }
    }
}
