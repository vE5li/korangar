#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, Timer, print_debug};
use korangar_interface::components::drop_down::DropDownItem;
#[cfg(feature = "debug")]
use korangar_interface::element::Element;
use korangar_interface::element::StateElement;
use korangar_loaders::FileLoader;
use ragnarok_packets::{ItemId, SkillId, SkillUseFailureCode};
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

/// Localized messages and format templates for rejected skill uses.
#[derive(Serialize, Deserialize, RustState, StateElement)]
pub(crate) struct SkillUseFailureLocalization {
    generic: String,
    not_enough_sp: String,
    not_enough_hp: String,
    missing_items: String,
    cooldown: String,
    not_enough_money: String,
    wrong_weapon: String,
    need_red_gemstone: String,
    need_blue_gemstone: String,
    overweight: String,
    invalid_target: String,
    need_holy_water: String,
    need_other_skill: String,
    invalid_direction: String,
    invalid_position: String,
    need_item: String,
    need_equipment: String,
    need_equipment_generic: String,
    need_combo_skill: String,
    need_spirits: String,
    need_spirits_generic: String,
    need_ammunition: String,
    need_coins: String,
    need_coins_generic: String,
    basic_emotions: String,
    basic_sit: String,
    basic_chat: String,
    basic_party: String,
    basic_shout: String,
    basic_pvp: String,
    basic_alignment: String,
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
    skill_use_failure: SkillUseFailureLocalization,
}

impl Localization {
    /// Formats a localized map-server skill rejection.
    pub fn skill_use_failure_message(
        &self,
        skill_id: SkillId,
        detail: i32,
        item_id: ItemId,
        item_name: Option<&str>,
        cause: SkillUseFailureCode,
    ) -> String {
        const BASIC_SKILL_ID: SkillId = SkillId(1);

        let messages = &self.skill_use_failure;
        match cause {
            SkillUseFailureCode::GENERIC if skill_id == BASIC_SKILL_ID => match detail {
                1 => messages.basic_emotions.clone(),
                2 => messages.basic_sit.clone(),
                3 => messages.basic_chat.clone(),
                4 => messages.basic_party.clone(),
                5 => messages.basic_shout.clone(),
                6 => messages.basic_pvp.clone(),
                7 => messages.basic_alignment.clone(),
                _ => messages.generic.clone(),
            },
            SkillUseFailureCode::NOT_ENOUGH_SP => messages.not_enough_sp.clone(),
            SkillUseFailureCode::NOT_ENOUGH_HP => messages.not_enough_hp.clone(),
            SkillUseFailureCode::MISSING_ITEMS => messages.missing_items.clone(),
            SkillUseFailureCode::COOLDOWN => messages.cooldown.clone(),
            SkillUseFailureCode::NOT_ENOUGH_MONEY => messages.not_enough_money.clone(),
            SkillUseFailureCode::WRONG_WEAPON => messages.wrong_weapon.clone(),
            SkillUseFailureCode::NEED_RED_GEMSTONE => messages.need_red_gemstone.clone(),
            SkillUseFailureCode::NEED_BLUE_GEMSTONE => messages.need_blue_gemstone.clone(),
            SkillUseFailureCode::OVERWEIGHT => messages.overweight.clone(),
            SkillUseFailureCode::INVALID_TARGET => messages.invalid_target.clone(),
            SkillUseFailureCode::NEED_HOLY_WATER => messages.need_holy_water.clone(),
            SkillUseFailureCode::NEED_OTHER_SKILL => messages.need_other_skill.clone(),
            SkillUseFailureCode::INVALID_DIRECTION => messages.invalid_direction.clone(),
            SkillUseFailureCode::INVALID_POSITION => messages.invalid_position.clone(),
            SkillUseFailureCode::NEED_ITEM if detail > 0 && item_id.0 != 0 => {
                let fallback_item_name = format!("#{}", item_id.0);
                messages
                    .need_item
                    .replace("{count}", &detail.to_string())
                    .replace("{item}", item_name.unwrap_or(&fallback_item_name))
            }
            SkillUseFailureCode::NEED_EQUIPMENT if item_id.0 != 0 => {
                let fallback_item_name = format!("#{}", item_id.0);
                messages.need_equipment.replace("{item}", item_name.unwrap_or(&fallback_item_name))
            }
            SkillUseFailureCode::NEED_EQUIPMENT => messages.need_equipment_generic.clone(),
            SkillUseFailureCode::NEED_COMBO_SKILL if detail > 0 => messages.need_combo_skill.replace("{skill_id}", &detail.to_string()),
            SkillUseFailureCode::NEED_COMBO_SKILL => messages.need_other_skill.clone(),
            SkillUseFailureCode::NEED_SPIRITS if detail > 0 => messages.need_spirits.replace("{count}", &detail.to_string()),
            SkillUseFailureCode::NEED_SPIRITS => messages.need_spirits_generic.clone(),
            SkillUseFailureCode::NEED_AMMUNITION => messages.need_ammunition.clone(),
            SkillUseFailureCode::NEED_COINS if detail > 0 => messages.need_coins.replace("{count}", &detail.to_string()),
            SkillUseFailureCode::NEED_COINS => messages.need_coins_generic.clone(),
            _ => messages.generic.clone(),
        }
    }

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
    use ragnarok_packets::{ItemId, SkillId, SkillUseFailureCode};

    use crate::state::localization::{Language, Localization};

    fn load_language_file(language: Language) -> Localization {
        let locale_code = language.to_locale_code();
        let file_name = format!("archive/data/languages/{locale_code}.ron");
        let file_content = std::fs::read_to_string(file_name).expect("language file should exist");
        ron::de::from_str(&file_content).expect("language file should be valid")
    }

    #[test]
    fn language_files_are_valid() {
        // Please extend this when adding a language.
        let languages = [Language::English, Language::German];

        for language in languages {
            // Used match here so the test fails if somebody forgets to adjust the test
            // after adding a language.
            match language {
                Language::English | Language::German => {
                    load_language_file(language);
                }
            }
        }
    }

    #[test]
    fn skill_use_failure_messages_are_localized_and_unknown_tolerant() {
        let english = load_language_file(Language::English);
        let german = load_language_file(Language::German);
        let no_item = ItemId(0);

        assert_eq!(
            english.skill_use_failure_message(SkillId(100), 0, no_item, None, SkillUseFailureCode::NOT_ENOUGH_SP,),
            "Not enough SP."
        );
        assert_eq!(
            german.skill_use_failure_message(SkillId(100), 0, no_item, None, SkillUseFailureCode::NOT_ENOUGH_SP,),
            "Nicht genügend SP."
        );
        assert_eq!(
            english.skill_use_failure_message(SkillId(100), 0, no_item, None, SkillUseFailureCode::GENERIC),
            "Skill failed."
        );
        assert_eq!(
            english.skill_use_failure_message(SkillId(100), 0, no_item, None, SkillUseFailureCode(255)),
            "Skill failed."
        );
        assert_eq!(
            english.skill_use_failure_message(SkillId(1), 2, no_item, None, SkillUseFailureCode::GENERIC),
            "Sitting requires a higher Basic Skill level."
        );
        assert_eq!(
            english.skill_use_failure_message(
                SkillId(100),
                2,
                ItemId(717),
                Some("Blue Gemstone"),
                SkillUseFailureCode::NEED_ITEM,
            ),
            "Requires 2 × Blue Gemstone."
        );
    }
}
