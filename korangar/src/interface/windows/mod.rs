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

#[derive(Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowClass {
    AudioSettings,
    CharacterOverview,
    CharacterSelection,
    GraphicsSettings,
    Hotbar,
    Login,
    Menu,
    Respawn,
    SelectServer,
}
