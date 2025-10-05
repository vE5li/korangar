#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, Timer, print_debug};
use korangar_interface::components::drop_down::DropDownItem;
#[cfg(feature = "debug")]
use korangar_interface::element::Element;
use korangar_interface::element::StateElement;
use korangar_loaders::FileLoader;
#[cfg(feature = "debug")]
use ron::ser::PrettyConfig;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

#[cfg(feature = "debug")]
use super::ClientState;
#[cfg(feature = "debug")]
use crate::input::InputEvent;
use crate::loaders::GameFileLoader;

/// Supported languages.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, RustState, StateElement)]
pub enum Language {
    English,
    German,
}

impl Language {
    /// Convert from our supported languages to a locale code.
    fn to_locale_code(self) -> &'static str {
        match self {
            Self::English => "en-US",
            Self::German => "de-DE",
        }
    }
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

/// Controls for reloading and saving the localization from the state
/// inspector.
///
/// It implements [`StateElement`] so it shows up in the state inspector,
/// allowing editing and saving language files from the client.
#[cfg(feature = "debug")]
#[derive(Default)]
pub struct LocalizationControls;

#[cfg(feature = "debug")]
impl StateElement<ClientState> for LocalizationControls {
    type LayoutInfo = impl std::any::Any;
    type LayoutInfoMut = impl std::any::Any;
    type Return<P>
        = impl Element<ClientState, LayoutInfo = Self::LayoutInfo>
    where
        P: rust_state::Path<ClientState, Self>;
    type ReturnMut<P>
        = impl Element<ClientState, LayoutInfo = Self::LayoutInfoMut>
    where
        P: rust_state::Path<ClientState, Self>;

    fn to_element<P>(_: P, name: String) -> Self::Return<P>
    where
        P: rust_state::Path<ClientState, Self>,
    {
        use korangar_interface::prelude::*;

        collapsible! {
            text: name,
            children: split! {
                gaps: theme().window().gaps(),
                children: (
                    button! {
                        text: "Reload",
                        tooltip: "Reload the language from disk",
                        event: InputEvent::ReloadLanguage,
                    },
                    button! {
                        text: "Save",
                        tooltip: "Save the language to disk",
                        event: InputEvent::SaveLanguage,
                    },
                ),
            },
        }
    }

    fn to_element_mut<P>(_: P, name: String) -> Self::ReturnMut<P>
    where
        P: rust_state::Path<ClientState, Self>,
    {
        use korangar_interface::prelude::*;

        collapsible! {
            text: name,
            children: (
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Reload",
                            tooltip: "Reload the language from disk",
                            event: InputEvent::ReloadLanguage,
                        },
                        button! {
                            text: "Save",
                            tooltip: "Save the language to disk",
                            event: InputEvent::SaveLanguage,
                        },
                    ),
                },
            ),
        }
    }
}

/// Localization for the client in form of a string lookup.
#[derive(Serialize, Deserialize, RustState, StateElement)]
pub struct Localization {
    /// Controls for reloading and saving the localization from the state
    /// inspector.
    #[cfg(feature = "debug")]
    #[serde(skip_deserializing, skip_serializing)]
    controls: LocalizationControls,
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
    game_settings_button_text: String,
    interface_settings_button_text: String,
    audio_settings_button_text: String,
    log_out_button_text: String,
    exit_button_text: String,
    character_overview_window_title: String,
    name_text: String,
    base_level_text: String,
    job_level_text: String,
    inventory_button_text: String,
    equipment_button_text: String,
    stats_button_text: String,
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
    stats_window_title: String,
    game_settings_window_title: String,
    interface_settings_window_title: String,
    language_text: String,
    scaling_text: String,
    menu_theme_text: String,
    in_game_theme_text: String,
    world_theme_text: String,
    available_stat_points_text: String,
    strength_text: String,
    agility_text: String,
    vitality_text: String,
    intelligence_text: String,
    dexterity_text: String,
    luck_text: String,
    auto_attack_button_text: String,
    available_skill_points_text: String,
    reset_skill_points_button_text: String,
    cancel_skill_points_button_text: String,
    apply_skill_points_button_text: String,
    distribute_skill_points_button_text: String,
}

impl Localization {
    /// Save the localization to a file based on the provided language.
    // TODO: Currently this will just save to the file system but we might want to
    // save using the `GameFileLoader` instead.
    #[cfg(feature = "debug")]
    pub fn save_language(&self, language: Language) {
        #[cfg(feature = "debug")]
        let timer = Timer::new("Save language");

        let locale_code = language.to_locale_code();
        let file_name = format!("archive/data/languages/{locale_code}.ron");

        #[cfg(feature = "debug")]
        print_debug!("saving to file {}", file_name.magenta());

        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();

        if let Err(_error) = std::fs::write(&file_name, data) {
            #[cfg(feature = "debug")]
            print_debug!("failed to save language to {}: {:?}", file_name.magenta(), _error.red());
        }

        #[cfg(feature = "debug")]
        timer.stop();
    }

    /// Load the localization from a file based on the provided language.
    pub fn load_language(game_file_loader: &GameFileLoader, language: Language) -> Self {
        #[cfg(feature = "debug")]
        let timer = Timer::new("Load language");

        let locale_code = language.to_locale_code();
        let file_name = format!("data\\languages\\{locale_code}.ron");

        #[cfg(feature = "debug")]
        print_debug!("loading from file {}", file_name.magenta());

        let bytes = game_file_loader.get(&file_name).expect("language files should be present");
        let localization = ron::de::from_bytes(&bytes).expect("language files should be valid");

        #[cfg(feature = "debug")]
        timer.stop();

        localization
    }
}

#[cfg(test)]
mod languages {
    use crate::state::localization::{Language, Localization};

    #[test]
    fn language_files_are_valid() {
        // Please extend this when adding a language.
        let languages = [Language::English, Language::German];

        for language in languages {
            // Used match here so the test fails if somebody forgets to adjust the test
            // after adding a language.
            match language {
                Language::English | Language::German => {
                    let locale_code = language.to_locale_code();
                    let file_name = format!("archive/data/languages/{locale_code}.ron");
                    let file_content = std::fs::read_to_string(file_name).expect("language file should exist");
                    let _: Localization = ron::de::from_str(&file_content).expect("language file should be valid");
                }
            }
        }
    }
}
