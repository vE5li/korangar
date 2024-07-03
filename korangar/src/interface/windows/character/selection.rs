use derive_new::new;
use korangar_interface::elements::ElementWrap;
use korangar_interface::size_bound;
use korangar_interface::state::PlainRemote;
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use ragnarok_packets::CharacterInformation;
use rust_state::Context;

use crate::interface::elements::CharacterPreview;
use crate::interface::layout::ScreenSize;
use crate::interface::theme::InterfaceThemeKind;
use crate::interface::windows::WindowCache;
use crate::GameState;

#[derive(new)]
pub struct CharacterSelectionWindow {
    slot_count: usize,
}

impl CharacterSelectionWindow {
    pub const WINDOW_CLASS: &'static str = "character_selection";
}

impl PrototypeWindow<GameState> for CharacterSelectionWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, application: &Context<GameState>, available_space: ScreenSize) -> Window<GameState> {
        let elements = (0..self.slot_count).map(|slot| CharacterPreview::new(slot).wrap()).collect();

        WindowBuilder::new()
            .with_title("Character Selection".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(400 > 700 < 1000, ?))
            .with_elements(elements)
            .with_theme_kind(InterfaceThemeKind::Menu)
            .build(window_cache, application, available_space)
    }
}
