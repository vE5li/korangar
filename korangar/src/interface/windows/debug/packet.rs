use korangar_interface::event::ClickAction;
use korangar_interface::window::{CustomWindow, StateWindow, Window, WindowTrait};
use rust_state::{Context, Path};

use crate::input::UserEvent;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::{WindowCache, WindowClass};
use crate::state::{ClientState, ClientThemeType};
use crate::{PacketState, PacketStatePathExt};

pub struct PacketWindow<P> {
    path: P,
}

impl<P> PacketWindow<P> {
    pub fn new(path: P) -> Self {
        Self { path }
    }
}

impl<P> CustomWindow<ClientState> for PacketWindow<P>
where
    P: Path<ClientState, PacketState>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Packets)
    }

    fn to_window<'a>(self) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Network Packets",
            class: Self::window_class(),
            theme: ClientThemeType::Game,
            closable: true,
            elements: (
                split! {
                    children: (
                        button! {
                            text: "Clear",
                            event: UserEvent::ClearPacketHistory,
                        },
                        state_button! {
                            text: "Show pings",
                            state: self.path.show_pings(),
                            event: Toggle(self.path.show_pings()),
                        },
                        state_button! {
                            text: "Update",
                            state: self.path.update(),
                            event: Toggle(self.path.update()),
                        },
                    ),
                },
            ),
        }
    }
}
