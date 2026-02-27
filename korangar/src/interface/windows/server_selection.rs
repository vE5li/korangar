use std::cmp::Ordering;

use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{DefaultLayoutInfo, Element};
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::{CharacterServerInformation, CharacterServerInformationPathExt};
use rust_state::{Path, State};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

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

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;
        use rust_state::{ManuallyAssertExt, VecIndexExt};

        struct ServerWrapper<P> {
            path: P,
            item_boxes: Vec<Box<dyn Element<ClientState, LayoutInfo = DefaultLayoutInfo<ClientState>>>>,
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

            fn correct_element_size(&mut self, state: &State<ClientState>) {
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
                                event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
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
            type LayoutInfo = Vec<DefaultLayoutInfo<ClientState>>;

            fn create_layout_info(
                &mut self,
                state: &State<ClientState>,
                mut store: ElementStoreMut,
                resolvers: &mut dyn Resolvers<ClientState>,
            ) -> Self::LayoutInfo {
                with_single_resolver(resolvers, |resolver| {
                    self.correct_element_size(state);

                    let (_area, layout_info) = resolver.with_derived(2.0, 4.0, |resolver| {
                        self.item_boxes
                            .iter_mut()
                            .enumerate()
                            .map(|(index, item_box)| item_box.create_layout_info(state, store.child_store(index as u64), resolver))
                            .collect()
                    });

                    layout_info
                })
            }

            fn lay_out<'a>(
                &'a self,
                state: &'a State<ClientState>,
                store: ElementStore<'a>,
                layout_info: &'a Self::LayoutInfo,
                layout: &mut WindowLayout<'a, ClientState>,
            ) {
                layout.with_layer(|layout| {
                    for (index, item_box) in self.item_boxes.iter().enumerate() {
                        item_box.lay_out(state, store.child_store(index as u64), &layout_info[index], layout);
                    }
                });
            }
        }

        window! {
            title: client_state().localization().server_selection_window_title(),
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
