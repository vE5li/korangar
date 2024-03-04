use derive_new::new;

use crate::input::UserEvent;
use crate::interface::*;

#[derive(new)]
pub struct CharacterOverviewWindow {}

impl CharacterOverviewWindow {
    pub const WINDOW_CLASS: &'static str = "character_overview";
}

impl PrototypeWindow for CharacterOverviewWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: ScreenSize) -> Window {
        let elements = vec![
            /*Text::default()
                .with_text(|| format!("base level: {}", player.get_base_level()))
                .wrap(),
            Text::default()
                .with_text(|| format!("job level: {}", player.get_job_level()))
                .wrap(),*/
            ButtonBuilder::new()
                .with_text("Inventory")
                .with_event(UserEvent::OpenInventoryWindow)
                .build()
                .wrap(),
            ButtonBuilder::new()
                .with_text("Equipment")
                .with_event(UserEvent::OpenEquipmentWindow)
                .build()
                .wrap(),
            ButtonBuilder::new()
                .with_text("Skill tree")
                .with_event(UserEvent::OpenSkillTreeWindow)
                .build()
                .wrap(),
            ButtonBuilder::new()
                .with_text("Friends")
                .with_event(UserEvent::OpenFriendsWindow)
                .build()
                .wrap(),
            ButtonBuilder::new()
                .with_text("Menu")
                .with_event(UserEvent::OpenMenuWindow)
                .build()
                .wrap(),
        ];

        WindowBuilder::new()
            .with_title("Character Overview".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(SizeBound::DEFAULT_UNBOUNDED)
            .with_elements(elements)
            .build(window_cache, interface_settings, available_space)
    }
}
