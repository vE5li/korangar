use korangar_interface::window::{CustomWindow, WindowTrait};
use rust_state::Path;

use crate::graphics::Color;
use crate::input::UserEvent;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

pub struct CharacterOverviewWindow<P, L, J> {
    player_name: P,
    base_level: L,
    job_level: J,
}

impl<P, L, J> CharacterOverviewWindow<P, L, J> {
    pub fn new(player_name: P, base_level: L, job_level: J) -> Self {
        Self {
            player_name,
            base_level,
            job_level,
        }
    }
}

impl<P, L, J> CustomWindow<ClientState> for CharacterOverviewWindow<P, L, J>
where
    P: Path<ClientState, String>,
    L: Path<ClientState, usize>,
    J: Path<ClientState, usize>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::CharacterOverview)
    }

    fn to_window<'a>(self) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let elements = (
            fragment! {
                gaps: 4.0,
                children: (
                    split! {
                        children: (
                            text! {
                                text: "Name",
                            },
                            text! {
                                text: self.player_name,
                                color: Color::rgb_u8(255, 144, 13),
                                horizontal_alignment: HorizontalAlignment::Right { offset: 0.0 },
                            },
                        ),
                    },
                    split! {
                        children: (
                            text! {
                                text: "Base level",
                            },
                            text! {
                                text: PartialEqDisplaySelector::new(self.base_level),
                                color: Color::rgb_u8(13, 231, 255),
                                horizontal_alignment: HorizontalAlignment::Right { offset: 0.0 },
                            },
                        ),
                    },
                    split! {
                        children: (
                            text! {
                                text: "Job level",
                            },
                            text! {
                                text: PartialEqDisplaySelector::new(self.job_level),
                                color: Color::rgb_u8(13, 231, 255),
                                horizontal_alignment: HorizontalAlignment::Right { offset: 0.0 },
                            },
                        ),
                    },
                ),
            },
            button! {
                text: "Inventory",
                event: UserEvent::OpenInventoryWindow,
            },
            button! {
                text: "Equipment",
                event: UserEvent::OpenEquipmentWindow,
            },
            button! {
                text: "Skill tree",
                event: UserEvent::OpenSkillTreeWindow,
            },
            button! {
                text: "Friend list",
                event: UserEvent::OpenFriendListWindow,
            },
            button! {
                text: "Menu",
                event: UserEvent::OpenMenuWindow,
            },
        );

        window! {
            title: "Character Overview",
            class: Self::window_class(),
            theme: InterfaceThemeType::Game,
            minimum_width: 300.0,
            maximum_width: 300.0,
            elements: elements,
        }
    }
}
