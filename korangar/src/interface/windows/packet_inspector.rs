use korangar_interface::element::Element;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Context, Path};

use crate::interface::windows::WindowClass;
use crate::networking::{PacketHistory, PacketHistoryPathExt};
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

pub struct PacketInspectorWindow<A> {
    packet_history_path: A,
}

impl<A> PacketInspectorWindow<A> {
    pub fn new(packet_history_path: A) -> Self {
        Self { packet_history_path }
    }
}

impl<A> CustomWindow<ClientState> for PacketInspectorWindow<A>
where
    A: Path<ClientState, PacketHistory>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::PacketInspector)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        struct BufferWrapper<A> {
            packet_history_path: A,
        }

        impl<A> Element<ClientState> for BufferWrapper<A>
        where
            A: Path<ClientState, PacketHistory>,
        {
            type LayoutInfo = ();

            fn create_layout_info(
                &mut self,
                state: &Context<ClientState>,
                mut store: ElementStoreMut,
                resolvers: &mut dyn Resolvers<ClientState>,
            ) {
                with_single_resolver(resolvers, |resolver| {
                    let packet_history = state.get(&self.packet_history_path);

                    packet_history.get_entries().iter().for_each(|entry| {
                        if ((entry.is_incoming() && packet_history.show_incoming) || (entry.is_outgoing() && packet_history.show_outgoing))
                            && (!entry.is_ping() || packet_history.show_pings)
                        {
                            let element = unsafe { &mut *entry.element.get() };
                            let store = store.child_store(entry.unique_id);
                            element.create_layout_info(state, store, resolver);
                        }
                    });
                })
            }

            fn lay_out<'a>(
                &'a self,
                state: &'a Context<ClientState>,
                store: ElementStore<'a>,
                _: &'a Self::LayoutInfo,
                layout: &mut WindowLayout<'a, ClientState>,
            ) {
                let packet_history = state.get(&self.packet_history_path);

                packet_history.get_entries().iter().for_each(|entry| {
                    if ((entry.is_incoming() && packet_history.show_incoming) || (entry.is_outgoing() && packet_history.show_outgoing))
                        && (!entry.is_ping() || packet_history.show_pings)
                    {
                        let element = unsafe { &*entry.element.get() };
                        let store = store.child_store(entry.unique_id);
                        element.lay_out(state, store, &(), layout)
                    }
                });
            }
        }

        window! {
            title: "Packet Inspector",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            minimum_height: 200.0,
            closable: true,
            resizable: true,
            elements: (
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Clear",
                            event: move |state: &Context<ClientState>, _: &mut EventQueue<ClientState>| {
                                state.update_value_with(self.packet_history_path.entries(), |buffer| buffer.clear());
                            }
                        },
                        state_button! {
                            text: "Incoming",
                            state: self.packet_history_path.show_incoming(),
                            event: Toggle(self.packet_history_path.show_incoming()),
                        },
                        state_button! {
                            text: "Outgoing",
                            state: self.packet_history_path.show_outgoing(),
                            event: Toggle(self.packet_history_path.show_outgoing()),
                        },
                        state_button! {
                            text: "Pings",
                            state: self.packet_history_path.show_pings(),
                            event: Toggle(self.packet_history_path.show_pings()),
                        },
                    ),
                },
                scroll_view! {
                    follow: true,
                    children: BufferWrapper {
                        packet_history_path: self.packet_history_path,
                    },
                },
            ),
        }
    }
}
