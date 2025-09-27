mod audio_settings;
mod buy;
mod buy_cart;
mod buy_or_sell;
mod cache;
mod character_creation;
mod character_overview;
mod character_selection;
mod chat;
#[cfg(feature = "debug")]
mod commands;
mod dialog;
mod equipment;
mod error;
#[cfg(feature = "debug")]
mod frame_inspector;
mod friend_list;
mod friend_request;
mod graphics_settings;
mod hotbar;
mod interface_settings;
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
mod sell;
mod sell_cart;
mod server_selection;
mod skill_tree;
mod stats;
#[cfg(feature = "debug")]
mod theme_inspector;
#[cfg(feature = "debug")]
mod time;

use serde::{Deserialize, Serialize};

pub use self::audio_settings::AudioSettingsWindow;
pub use self::buy::BuyWindow;
pub use self::buy_cart::BuyCartWindow;
pub use self::buy_or_sell::BuyOrSellWindow;
pub use self::cache::WindowCache;
pub use self::character_creation::CharacterCreationWindow;
pub use self::character_overview::CharacterOverviewWindow;
pub use self::character_selection::CharacterSelectionWindow;
pub use self::chat::{ChatTextBox, ChatWindow, ChatWindowState};
#[cfg(feature = "debug")]
pub use self::commands::CommandsWindow;
pub use self::dialog::{DialogWindow, DialogWindowState};
pub use self::equipment::EquipmentWindow;
pub use self::error::ErrorWindow;
#[cfg(feature = "debug")]
pub use self::frame_inspector::FrameInspectorWindow;
pub use self::friend_list::{FriendListWindow, FriendListWindowState};
pub use self::friend_request::FriendRequestWindow;
pub use self::graphics_settings::GraphicsSettingsWindow;
pub use self::hotbar::HotbarWindow;
pub use self::interface_settings::InterfaceSettingsWindow;
pub use self::inventory::InventoryWindow;
pub use self::login::{LoginWindow, LoginWindowState};
#[cfg(feature = "debug")]
pub use self::maps::MapsWindow;
pub use self::menu::MenuWindow;
#[cfg(feature = "debug")]
pub use self::packet_inspector::PacketInspectorWindow;
#[cfg(feature = "debug")]
pub use self::profiler::{ProfilerWindow, ProfilerWindowState};
#[cfg(feature = "debug")]
pub use self::render_options::RenderOptionsWindow;
pub use self::respawn::RespawnWindow;
pub use self::sell::SellWindow;
pub use self::sell_cart::SellCartWindow;
pub use self::server_selection::ServerSelectionWindow;
pub use self::skill_tree::SkillTreeWindow;
pub use self::stats::StatsWindow;
#[cfg(feature = "debug")]
pub use self::theme_inspector::{ThemeInspectorWindow, ThemeInspectorWindowState};
#[cfg(feature = "debug")]
pub use self::time::TimeWindow;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowClass {
    AudioSettings,
    Buy,
    BuyCart,
    BuyOrSell,
    Chat,
    CharacterCreation,
    CharacterOverview,
    CharacterSelection,
    Dialog,
    InterfaceSettings,
    GraphicsSettings,
    Hotbar,
    Inventory,
    Equipment,
    SkillTree,
    Stats,
    FriendList,
    FriendRequest,
    Login,
    Menu,
    Respawn,
    SelectServer,
    Sell,
    SellCart,
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
    ThemeInspector,
    #[cfg(feature = "debug")]
    Profiler,
    #[cfg(feature = "debug")]
    CacheStatistics,
}
