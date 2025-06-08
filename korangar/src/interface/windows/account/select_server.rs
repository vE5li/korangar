use std::any::Any;
use std::cell::UnsafeCell;
use std::cmp::Ordering;
use std::collections::HashMap;

use derive_new::new;
use korangar_interface::element::id::ElementIdGenerator;
use korangar_interface::element::store::ElementStore;
use korangar_interface::element::{DefaultLayouted, Element, ElementSet, ResolverSet};
use korangar_interface::event::EventQueue;
use korangar_interface::layout::{Layout, Resolver};
use korangar_interface::window::{CustomWindow, PrototypeWindow, Window, WindowTrait};
use ragnarok_packets::{CharacterServerInformation, CharacterServerInformationPathExt};
use rust_state::{Context, Path};

use crate::input::UserEvent;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::{WindowCache, WindowClass};
use crate::loaders::ServiceId;
use crate::state::{ClientState, ClientThemeType};

pub struct SelectServerWindow<P> {
    path: P,
}

impl<P> SelectServerWindow<P> {
    pub fn new(path: P) -> Self {
        Self { path }
    }
}

impl<P> CustomWindow<ClientState> for SelectServerWindow<P>
where
    P: Path<ClientState, Vec<CharacterServerInformation>>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::SelectServer)
    }

    fn to_window<'a>(
        self,
        state: &Context<ClientState>,
        window_cache: &WindowCache,
        available_space: ScreenSize,
    ) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;
        use rust_state::{ManuallyAssertExt, VecIndexExt};

        struct ServerWrapper<P> {
            path: P,
            item_boxes: Vec<Box<dyn Element<ClientState, Layouted = DefaultLayouted>>>,
        }

        impl<P> ServerWrapper<P>
        where
            P: Path<ClientState, Vec<CharacterServerInformation>>,
        {
            fn new(path: P) -> Self {
                Self {
                    path,
                    item_boxes: Vec::new(),
                }
            }

            fn correct_element_size(&mut self, state: &Context<ClientState>) {
                let character_server_information = state.get(&self.path);

                match self.item_boxes.len().cmp(&character_server_information.len()) {
                    Ordering::Greater => {
                        // Delete excess elements.
                        self.item_boxes.truncate(character_server_information.len());
                    }
                    Ordering::Less => {
                        // Add new elements.
                        for index in self.item_boxes.len()..character_server_information.len() {
                            let path = self.path.index(index).manually_asserted();
                            self.item_boxes.push(Box::new(button! {
                                text: self.path.index(index).manually_asserted().server_name(),
                                event: move |state: &Context<ClientState>, queue: &mut EventQueue<ClientState>| {
                                    let character_server_information = state.get(&path).clone();
                                    queue.queue(UserEvent::SelectServer { character_server_information });
                                },
                            }));
                        }
                    }
                    Ordering::Equal => {}
                }
            }
        }

        impl<P> ElementSet<ClientState> for ServerWrapper<P>
        where
            P: Path<ClientState, Vec<CharacterServerInformation>>,
        {
            type Layouted = Vec<DefaultLayouted>;

            fn get_element_count(&self) -> usize {
                unimplemented!()
            }

            fn make_layout(
                &mut self,
                state: &Context<ClientState>,
                store: &mut ElementStore,
                generator: &mut ElementIdGenerator,
                mut resolver_set: impl ResolverSet,
            ) -> Self::Layouted {
                self.correct_element_size(state);

                // FIX: Make this right. Maybe with_derived should expect a resolver set as well
                resolver_set.with_index(0, |resolver| {
                    let (area, layouted) = resolver.with_derived(2.0, 4.0, |resolver| {
                        self.item_boxes
                            .iter_mut()
                            .enumerate()
                            .map(|(index, item_box)| {
                                item_box.make_layout(
                                    state,
                                    store.get_or_create_child_store(index as u64, generator),
                                    generator,
                                    resolver,
                                )
                            })
                            .collect()
                    });

                    layouted
                })
            }

            fn create_layout<'a>(
                &'a self,
                state: &'a Context<ClientState>,
                store: &'a ElementStore,
                layouted: &'a Self::Layouted,
                layout: &mut Layout<'a, ClientState>,
            ) {
                layout.push_layer();

                for (index, item_box) in self.item_boxes.iter().enumerate() {
                    item_box.create_layout(state, store.child_store(index as u64), &layouted[index], layout);
                }

                // TODO: Very much temp
                layout.pop_layer();
            }
        }

        window! {
            title: "Select Server",
            class: Self::window_class(),
            theme: ClientThemeType::Menu,
            minimum_width: 450.0,
            maximum_width: 450.0,
            elements: ServerWrapper::new(self.path),
        }
    }
}
