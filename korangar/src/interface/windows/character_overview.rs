use korangar_interface::window::{CustomWindow, Window};
use rust_state::Path;

use crate::graphics::Color;
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

pub struct CharacterOverviewWindow<A, B, C> {
    player_name_path: A,
    base_level_path: B,
    job_level_path: C,
}

impl<A, B, C> CharacterOverviewWindow<A, B, C> {
    pub fn new(player_name_path: A, base_level_path: B, job_level_path: C) -> Self {
        Self {
            player_name_path,
            base_level_path,
            job_level_path,
        }
    }
}

impl<A, B, C> CustomWindow<ClientState> for CharacterOverviewWindow<A, B, C>
where
    A: Path<ClientState, String>,
    B: Path<ClientState, usize>,
    C: Path<ClientState, usize>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::CharacterOverview)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Character Overview",
            class: Self::window_class(),
            theme: InterfaceThemeType::Game,
            minimum_width: 300.0,
            maximum_width: 300.0,
            elements: (
                fragment! {
                    gaps: 4.0,
                    children: (
                        split! {
                            children: (
                                text! {
                                    text: "Name",
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                                text! {
                                    text: self.player_name_path,
                                    color: Color::rgb_u8(255, 144, 13),
                                    horizontal_alignment: HorizontalAlignment::Right { offset: 0.0, border: 3.0 },
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                            ),
                        },
                        split! {
                            children: (
                                text! {
                                    text: "Base level",
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                                text! {
                                    text: PartialEqDisplaySelector::new(self.base_level_path),
                                    color: Color::rgb_u8(13, 231, 255),
                                    horizontal_alignment: HorizontalAlignment::Right { offset: 0.0, border: 3.0 },
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                            ),
                        },
                        split! {
                            children: (
                                text! {
                                    text: "Job level",
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                                text! {
                                    text: PartialEqDisplaySelector::new(self.job_level_path),
                                    color: Color::rgb_u8(13, 231, 255),
                                    horizontal_alignment: HorizontalAlignment::Right { offset: 0.0, border: 3.0 },
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                            ),
                        },
                    ),
                },
                button! {
                    text: "Inventory",
                    event: InputEvent::OpenInventoryWindow,
                },
                button! {
                    text: "Equipment",
                    event: InputEvent::OpenEquipmentWindow,
                },
                button! {
                    text: "Skill tree",
                    event: InputEvent::OpenSkillTreeWindow,
                },
                button! {
                    text: "Friend list",
                    event: InputEvent::OpenFriendListWindow,
                },
                button! {
                    text: "Menu",
                    event: InputEvent::OpenMenuWindow,
                },
            ),
        }
    }
}
