use derive_new::new;
use korangar_interface::window::{CustomWindow, PrototypeWindow, Window, WindowTrait};
use rust_state::Context;

use crate::input::UserEvent;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;
use crate::state::{ClientState, ClientThemeType};

#[derive(new)]
pub struct CharacterOverviewWindow;

impl CharacterOverviewWindow {
    pub const WINDOW_CLASS: &'static str = "character_overview";
}

impl CustomWindow<ClientState> for CharacterOverviewWindow {
    fn window_class() -> Option<&'static str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window<'a>(
        self,
        state: &Context<ClientState>,
        window_cache: &WindowCache,
        available_space: ScreenSize,
    ) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let elements = (
            /*Text::default()
                .with_text(|| format!("base level: {}", player.get_base_level()))
                .wrap(),
            Text::default()
                .with_text(|| format!("job level: {}", player.get_job_level()))
                .wrap(),*/
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
                text: "Friends",
                event: UserEvent::OpenFriendsWindow,
            },
            button! {
                text: "Menu",
                event: UserEvent::OpenMenuWindow,
            },
        );

        window! {
            title: "Character Overview",
            theme: ClientThemeType::Game,
            window_id: 0,
            elements: elements,
        }
    }
}
