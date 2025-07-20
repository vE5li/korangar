use korangar_interface::event::ClickAction;
use korangar_interface::window::{CustomWindow, StateWindow, Window, WindowTrait};
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

    fn to_window<'a>(self) -> impl WindowTrait<ClientState> + 'a {
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
                            tooltip: "Increase base level by 1 [@blvl 1]",
                            event: UserEvent::SendMessage("@blvl 1".to_string()),
                        },
                        button! {
                            text: "+5",
                            tooltip: "Increase base level by 5 [@blvl 5]",
                            event: UserEvent::SendMessage("@blvl 5".to_string()),
                        },
                        button! {
                            text: "+10",
                            tooltip: "Increase base level by 10 [@blvl 10]",
                            event: UserEvent::SendMessage("@blvl 10".to_string()),
                        },
                        button! {
                            text: "MAX",
                            tooltip: "Set base level to the maximum [@blvl 9999]",
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
                            tooltip: "Increase job level by 1 [@blvl 1]",
                            event: UserEvent::SendMessage("@jlvl 1".to_string()),
                        },
                        button! {
                            text: "+5",
                            tooltip: "Increase job level by 5 [@blvl 5]",
                            event: UserEvent::SendMessage("@jlvl 5".to_string()),
                        },
                        button! {
                            text: "+10",
                            tooltip: "Increase job level by 10 [@blvl 10]",
                            event: UserEvent::SendMessage("@jlvl 10".to_string()),
                        },
                        button! {
                            text: "MAX",
                            tooltip: "Set base job to the maximum [@blvl 9999]",
                            event: UserEvent::SendMessage("@jlvl 9999".to_string()),
                        },
                    ),
                },
                text! {
                    text: "Stats",
                },
                button! {
                    text: "Set all to max",
                    tooltip: "Set all stats to the maximum [@allstats]",
                    event: UserEvent::SendMessage("@allstats".to_string()),
                },
                text! {
                    text: "Skills",
                },
                button! {
                    text: "Unlock all",
                    tooltip: "Unlock all learnable skills [@allskill]",
                    event: UserEvent::SendMessage("@allskill".to_string()),
                },
                text! {
                    text: "Resources",
                },
                button! {
                    text: "Give 10,000 Zeny",
                    tooltip: "Give the player 10,000 Zeny [@zeny 10000]",
                    event: UserEvent::SendMessage("@zeny 10000".to_string()),
                },
                text! {
                    text: "Player state",
                },
                button! {
                    text: "Mount",
                    tooltip: "Mount if possible [@mount]",
                    event: UserEvent::SendMessage("@mount".to_string()),
                },
                button! {
                    text: "Heal",
                    tooltip: "Heal the player [@heal]",
                    event: UserEvent::SendMessage("@heal".to_string()),
                },
                button! {
                    text: "Fill AP",
                    tooltip: "Fill the player AP [@healap]",
                    event: UserEvent::SendMessage("@healap".to_string()),
                },
                button! {
                    text: "Resurrect",
                    tooltip: "Resurrect the player [@alive]",
                    event: UserEvent::SendMessage("@alive".to_string()),
                },
            ),
        }
    }
}
