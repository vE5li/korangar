use derive_new::new;
use procedural::*;

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

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Window {
        let elements = vec![
            /*Text::default()
                .with_dynamic_text(|| format!("base level: {}", player.get_base_level()))
                .wrap(),
            Text::default()
                .with_dynamic_text(|| format!("job level: {}", player.get_job_level()))
                .wrap(),*/
            Button::default()
                .with_static_text("inventory")
                .with_event(UserEvent::OpenInventoryWindow)
                .wrap(),
            Button::default()
                .with_static_text("equipment")
                .with_event(UserEvent::OpenEquipmentWindow)
                .wrap(),
            Button::default()
                .with_static_text("menu")
                .with_event(UserEvent::OpenMenuWindow)
                .wrap(),
        ];

        Window::new(
            window_cache,
            interface_settings,
            avalible_space,
            "Character Overview".to_string(),
            Self::WINDOW_CLASS.to_string().into(),
            elements,
            constraint!(200 > 300 < 400, ?),
            false,
        )
    }
}
