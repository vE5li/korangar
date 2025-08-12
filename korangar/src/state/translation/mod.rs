#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, Timer, print_debug};
use korangar_interface::components::drop_down::DropDownItem;
use korangar_interface::element::StateElement;
use korangar_util::FileLoader;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

use crate::loaders::GameFileLoader;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, RustState, StateElement)]
pub enum Language {
    English,
    German,
}

impl DropDownItem<Language> for Language {
    fn text(&self) -> &str {
        match self {
            Self::English => "English",
            Self::German => "Deutsch",
        }
    }

    fn value(&self) -> Language {
        *self
    }
}

#[derive(Serialize, Deserialize, RustState, StateElement)]
pub struct Translation {
    log_in_window_title: String,
    select_service_text: String,
    account_data_text: String,
    username_text: String,
    password_text: String,
    remember_username_text: String,
    remember_password_text: String,
    log_in_button_text: String,
    log_in_button_tooltip: String,
    menu_window_title: String,
    graphics_settings_button_text: String,
    audio_settings_button_text: String,
    log_out_button_text: String,
    exit_button_text: String,
    character_overview_window_title: String,
    name_text: String,
    base_level_text: String,
    job_level_text: String,
    inventory_button_text: String,
    equipment_button_text: String,
    skill_tree_button_text: String,
    friend_list_button_text: String,
    menu_button_text: String,
    chat_window_title: String,
    chat_text_box_message: String,
    audio_settings_window_title: String,
    mute_audio_on_focus_loss_button_text: String,
    create_character_window_title: String,
    character_name_text: String,
    create_character_button_text: String,
    create_character_button_tooltip: String,
    dialog_window_title: String,
    next_button_text: String,
    close_button_text: String,
    error_window_title: String,
    friend_list_window_title: String,
    friend_list_text_box_message: String,
    remove_button_text: String,
    hotbar_window_title: String,
    inventory_window_title: String,
    respawn_window_title: String,
    respawn_button_text: String,
    disconnect_button_text: String,
    server_selection_window_title: String,
    skill_tree_window_title: String,
}

impl Translation {
    pub fn load_language(game_file_loader: &GameFileLoader, language: Language) -> Self {
        #[cfg(feature = "debug")]
        let timer = Timer::new("Load language");

        let locale_code = match language {
            Language::English => "en-US",
            Language::German => "de-DE",
        };

        let file_name = format!("data\\languages\\{locale_code}.ron");

        #[cfg(feature = "debug")]
        print_debug!("loading from file {}", file_name.magenta());

        let bytes = game_file_loader.get(&file_name).expect("language files should be present");
        let translation = ron::de::from_bytes(&bytes).expect("language files should be valid");

        #[cfg(feature = "debug")]
        timer.stop();

        translation
    }
}
