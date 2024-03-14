use korangar_interface::elements::{ButtonBuilder, Container, ElementCell, ElementWrap, Headline, PrototypeElement};
use korangar_procedural::{dimension_bound, size_bound, PrototypeElement};

use crate::input::UserEvent;
use crate::interface::application::{InterfaceSettings, InternalThemeKind};

/// Debug actions for a single theme.
#[derive(Default)]
struct Actions<const KIND: InternalThemeKind>;

impl<const KIND: InternalThemeKind> PrototypeElement<InterfaceSettings> for Actions<KIND> {
    fn to_element(&self, display: String) -> ElementCell<InterfaceSettings> {
        let elements = vec![
            Headline::new(display, size_bound!(33%, 12)).wrap(),
            ButtonBuilder::new()
                .with_text("Save")
                .with_event(UserEvent::SaveTheme { theme_kind: KIND })
                .with_width_bound(dimension_bound!(33%))
                .build()
                .wrap(),
            ButtonBuilder::new()
                .with_text("Reload")
                .with_event(UserEvent::ReloadTheme { theme_kind: KIND })
                .with_width_bound(dimension_bound!(!))
                .build()
                .wrap(),
        ];

        Container::new(elements).wrap()
    }
}

/// Debug actions for all themes.
#[derive(Default, PrototypeElement)]
pub(super) struct ThemeActions {
    #[name("Main theme")]
    main_theme_actions: Actions<{ InternalThemeKind::Main }>,
    #[name("Menu theme")]
    menu_theme_actions: Actions<{ InternalThemeKind::Menu }>,
    #[name("Game theme")]
    game_theme_actions: Actions<{ InternalThemeKind::Game }>,
}
