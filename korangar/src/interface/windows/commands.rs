use korangar_interface::window::{CustomWindow, WindowTrait};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

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
            theme: InterfaceThemeType::Game,
            closable: true,
            elements: (
                text! {
                    text: "Base level",
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "+1",
                            tooltip: "Increase base level by 1 [^50C3FF@blvl 1^000000]",
                            event: InputEvent::SendMessage { text: "@blvl 1".to_string() },
                        },
                        button! {
                            text: "+5",
                            tooltip: "Increase base level by 5 [^50C3FF@blvl 5^000000]",
                            event: InputEvent::SendMessage { text: "@blvl 5".to_string() },
                        },
                        button! {
                            text: "+10",
                            tooltip: "Increase base level by 10 [^50C3FF@blvl 10^000000]",
                            event: InputEvent::SendMessage { text: "@blvl 10".to_string() },
                        },
                        button! {
                            text: "MAX",
                            tooltip: "Set base level to the maximum [^50C3FF@blvl 9999^000000]",
                            event: InputEvent::SendMessage { text: "@blvl 9999".to_string() },
                        },
                    ),
                },
                text! {
                    text: "Job level",
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "+1",
                            tooltip: "Increase job level by 1 [^50C3FF@blvl 1^000000]",
                            event: InputEvent::SendMessage { text: "@jlvl 1".to_string() },
                        },
                        button! {
                            text: "+5",
                            tooltip: "Increase job level by 5 [^50C3FF@blvl 5^000000]",
                            event: InputEvent::SendMessage { text: "@jlvl 5".to_string() },
                        },
                        button! {
                            text: "+10",
                            tooltip: "Increase job level by 10 [^50C3FF@blvl 10^000000]",
                            event: InputEvent::SendMessage { text: "@jlvl 10".to_string() },
                        },
                        button! {
                            text: "MAX",
                            tooltip: "Set base job to the maximum [^50C3FF@blvl 9999^000000]",
                            event: InputEvent::SendMessage { text: "@jlvl 9999".to_string() },
                        },
                    ),
                },
                text! {
                    text: "Stats",
                },
                button! {
                    text: "Set all to max",
                    tooltip: "Set all stats to the maximum [^50C3FF@allstats^000000]",
                    event: InputEvent::SendMessage { text: "@allstats".to_string() },
                },
                text! {
                    text: "Skills",
                },
                button! {
                    text: "Unlock all",
                    tooltip: "Unlock all learnable skills [^50C3FF@allskill^000000]",
                    event: InputEvent::SendMessage { text: "@allskill".to_string() },
                },
                text! {
                    text: "Resources",
                },
                button! {
                    text: "Give 10,000 Zeny",
                    tooltip: "Give the player 10,000 Zeny [^50C3FF@zeny 10000^000000]",
                    event: InputEvent::SendMessage { text: "@zeny 10000".to_string() },
                },
                text! {
                    text: "Player state",
                },
                button! {
                    text: "Mount",
                    tooltip: "Mount if possible [^50C3FF@mount^000000]",
                    event: InputEvent::SendMessage { text: "@mount".to_string() },
                },
                button! {
                    text: "Heal",
                    tooltip: "Heal the player [^50C3FF@heal^000000]",
                    event: InputEvent::SendMessage { text: "@heal".to_string() },
                },
                button! {
                    text: "Fill AP",
                    tooltip: "Fill the player AP [^50C3FF@healap^000000]",
                    event: InputEvent::SendMessage { text: "@healap".to_string() },
                },
                button! {
                    text: "Resurrect",
                    tooltip: "Resurrect the player [^50C3FF@alive^000000]",
                    event: InputEvent::SendMessage { text: "@alive".to_string() },
                },
            ),
        }
    }
}
