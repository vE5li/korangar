mod audio_settings;
mod cache;
mod character_creation;
mod character_overview;
mod character_selection;
#[cfg(feature = "debug")]
mod commands;
mod equipment;
mod error;
#[cfg(feature = "debug")]
mod frame_inspector;
mod friend_list;
mod friend_request;
mod graphics_settings;
mod hotbar;
mod inventory;
mod login;
#[cfg(feature = "debug")]
mod maps;
mod menu;
#[cfg(feature = "debug")]
mod packet_inspector;
#[cfg(feature = "debug")]
mod profiler;
#[cfg(feature = "debug")]
mod render_options;
mod respawn;
mod select_server;
mod skill_tree;
#[cfg(feature = "debug")]
mod time;

use serde::{Deserialize, Serialize};

pub use self::audio_settings::AudioSettingsWindow;
pub use self::cache::WindowCache;
pub use self::character_creation::CharacterCreationWindow;
pub use self::character_overview::CharacterOverviewWindow;
pub use self::character_selection::CharacterSelectionWindow;
#[cfg(feature = "debug")]
pub use self::commands::CommandsWindow;
pub use self::equipment::EquipmentWindow;
pub use self::error::ErrorWindow;
#[cfg(feature = "debug")]
pub use self::frame_inspector::FrameInspectorWindow;
pub use self::friend_list::FriendListWindow;
pub use self::friend_request::FriendRequestWindow;
pub use self::graphics_settings::GraphicsSettingsWindow;
pub use self::hotbar::HotbarWindow;
pub use self::inventory::InventoryWindow;
// pub use self::shop::*;
pub use self::login::LoginWindow;
#[cfg(feature = "debug")]
pub use self::maps::MapsWindow;
pub use self::menu::MenuWindow;
#[cfg(feature = "debug")]
pub use self::packet_inspector::PacketInspector;
#[cfg(feature = "debug")]
pub use self::profiler::ProfilerWindow;
#[cfg(feature = "debug")]
pub use self::render_options::RenderOptionsWindow;
pub use self::respawn::RespawnWindow;
pub use self::select_server::SelectServerWindow;
pub use self::skill_tree::SkillTreeWindow;
#[cfg(feature = "debug")]
pub use self::time::TimeWindow;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowClass {
    AudioSettings,
    CharacterCreation,
    CharacterOverview,
    CharacterSelection,
    GraphicsSettings,
    Hotbar,
    Inventory,
    Equipment,
    SkillTree,
    FriendList,
    FriendRequest,
    Login,
    Menu,
    Respawn,
    SelectServer,
    #[cfg(feature = "debug")]
    Time,
    #[cfg(feature = "debug")]
    Maps,
    #[cfg(feature = "debug")]
    ClientStateInspector,
    #[cfg(feature = "debug")]
    PacketInspector,
    #[cfg(feature = "debug")]
    RenderOptions,
    #[cfg(feature = "debug")]
    Commands,
    #[cfg(feature = "debug")]
    Profiler,
}
