mod account;
mod cache;
mod character;
#[cfg(feature = "debug")]
mod debug;
mod friend_list;
mod friend_request;
mod generic;
mod settings;
// mod shop;

use serde::{Deserialize, Serialize};

pub use self::account::*;
pub use self::cache::WindowCache;
pub use self::character::*;
#[cfg(feature = "debug")]
pub use self::debug::*;
pub use self::friend_list::FriendListWindow;
pub use self::friend_request::FriendRequestWindow;
pub use self::generic::*;
pub use self::settings::*;
// pub use self::shop::*;

// TODO: Small issue with excluding window classes based on the debug build is
// that deserialization of the window cache will fail when going from a debug
// build to a release build. Not sure what the correct approach is yet.
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
    ClientState,
    #[cfg(feature = "debug")]
    Packets,
    #[cfg(feature = "debug")]
    RenderOptions,
    #[cfg(feature = "debug")]
    Commands,
    #[cfg(feature = "debug")]
    Profiler,
}
