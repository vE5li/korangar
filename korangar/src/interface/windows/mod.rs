mod account;
mod cache;
mod character;
#[cfg(feature = "debug")]
mod debug;
// mod friends;
mod generic;
mod settings;
// mod shop;

use serde::{Deserialize, Serialize};

pub use self::account::*;
pub use self::cache::WindowCache;
pub use self::character::*;
#[cfg(feature = "debug")]
pub use self::debug::*;
// pub use self::friends::*;
pub use self::generic::*;
pub use self::settings::*;
// pub use self::shop::*;

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
    Login,
    Menu,
    Respawn,
    SelectServer,
    #[cfg(feature = "debug")]
    Time,
    #[cfg(feature = "debug")]
    Maps,
    #[cfg(feature = "debug")]
    Packets,
    #[cfg(feature = "debug")]
    RenderSettings,
    #[cfg(feature = "debug")]
    Commands,
    #[cfg(feature = "debug")]
    Profiler,
}
