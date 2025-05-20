use korangar_interface::event::ClickAction;
use korangar_interface::window::{CustomWindow, PrototypeWindow, Window, WindowTrait};
use rust_state::Context;

use crate::input::UserEvent;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::{WindowCache, WindowClass};
use crate::state::{ClientState, ClientThemeType};

pub struct CommandsWindow;

impl CustomWindow<ClientState> for CommandsWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Commands)
    }

    fn to_window<'a>(
        self,
        state: &Context<ClientState>,
        window_cache: &WindowCache,
        available_space: ScreenSize,
    ) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Commands",
            class: Self::window_class(),
            theme: ClientThemeType::Game,
            closable: true,
            elements: (
                text! {
                    text: "Base level",
                },
                split! {
                    children: (
                        button! {
                            text: "+1",
                            event: UserEvent::SendMessage("@blvl 1".to_string()),
                        },
                        button! {
                            text: "+5",
                            event: UserEvent::SendMessage("@blvl 5".to_string()),
                        },
                        button! {
                            text: "+10",
                            event: UserEvent::SendMessage("@blvl 10".to_string()),
                        },
                        button! {
                            text: "MAX",
                            event: UserEvent::SendMessage("@blvl 9999".to_string()),
                        },
                    ),
                },
                text! {
                    text: "Job level",
                },
                split! {
                    children: (
                        button! {
                            text: "+1",
                            event: UserEvent::SendMessage("@jlvl 1".to_string()),
                        },
                        button! {
                            text: "+5",
                            event: UserEvent::SendMessage("@jlvl 5".to_string()),
                        },
                        button! {
                            text: "+10",
                            event: UserEvent::SendMessage("@jlvl 10".to_string()),
                        },
                        button! {
                            text: "MAX",
                            event: UserEvent::SendMessage("@jlvl 9999".to_string()),
                        },
                    ),
                },
                text! {
                    text: "Stats",
                },
                button! {
                    text: "Set all stats to max",
                    event: UserEvent::SendMessage("@allstats".to_string()),
                },
                text! {
                    text: "Skills",
                },
                button! {
                    text: "Unlock all skills",
                    event: UserEvent::SendMessage("@allskill".to_string()),
                },
                text! {
                    text: "Resources",
                },
                button! {
                    text: "Give 10,000 Zeny",
                    event: UserEvent::SendMessage("@zeny 10000".to_string()),
                },
                text! {
                    text: "Player state",
                },
                button! {
                    text: "Mount",
                    event: UserEvent::SendMessage("@mount".to_string()),
                },
                button! {
                    text: "Heal",
                    event: UserEvent::SendMessage("@heal".to_string()),
                },
                button! {
                    text: "Fill AP",
                    event: UserEvent::SendMessage("@healap".to_string()),
                },
                button! {
                    text: "Resurrect",
                    event: UserEvent::SendMessage("@alive".to_string()),
                },
            ),
        }
    }
}
