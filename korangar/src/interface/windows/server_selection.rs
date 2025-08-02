use std::cmp::Ordering;

use korangar_interface::element::id::ElementIdGenerator;
use korangar_interface::element::store::ElementStore;
use korangar_interface::element::{DefaultLayoutInfo, Element};
use korangar_interface::event::EventQueue;
use korangar_interface::layout::{Layout, Resolver};
use korangar_interface::window::{CustomWindow, WindowTrait};
use ragnarok_packets::{CharacterServerInformation, CharacterServerInformationPathExt};
use rust_state::{Context, Path};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

pub struct ServerSelectionWindow<P> {
    path: P,
}

impl<P> ServerSelectionWindow<P> {
    pub fn new(path: P) -> Self {
        Self { path }
    }
}

impl<P> CustomWindow<ClientState> for ServerSelectionWindow<P>
where
    P: Path<ClientState, Vec<CharacterServerInformation>>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::SelectServer)
    }

    fn to_window<'a>(self) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;
        use rust_state::{ManuallyAssertExt, VecIndexExt};

        struct ServerWrapper<P> {
            path: P,
            item_boxes: Vec<Box<dyn Element<ClientState, LayoutInfo = DefaultLayoutInfo>>>,
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
                                    queue.queue(InputEvent::SelectServer { character_server_information });
                                },
                            }));
                        }
                    }
                    Ordering::Equal => {}
                }
            }
        }

        impl<P> Element<ClientState> for ServerWrapper<P>
        where
            P: Path<ClientState, Vec<CharacterServerInformation>>,
        {
            type LayoutInfo = Vec<DefaultLayoutInfo>;

            fn create_layout_info(
                &mut self,
                state: &Context<ClientState>,
                store: &mut ElementStore,
                generator: &mut ElementIdGenerator,
                resolver: &mut Resolver,
            ) -> Self::LayoutInfo {
                self.correct_element_size(state);

                let (_area, layout_info) = resolver.with_derived(2.0, 4.0, |resolver| {
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
            title: "Select Server",
            class: Self::window_class(),
            theme: InterfaceThemeType::Menu,
            minimum_width: 450.0,
            maximum_width: 450.0,
            elements: (
                ServerWrapper::new(self.path),
            ),
        }
    }
}
