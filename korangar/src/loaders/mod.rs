mod action;
mod animation;
mod archive;

mod effect;
pub mod error;
mod font;
mod gamefile;
mod map;
mod model;
mod script;
mod server;
mod sprite;
mod texture;

pub use self::action::*;
pub use self::animation::*;
pub use self::effect::EffectLoader;
pub use self::font::{FontLoader, FontSize, GlyphInstruction, Scaling, TextLayout};
pub use self::gamefile::*;
pub use self::map::{MapLoader, MAP_TILE_SIZE};
pub use self::model::*;
pub use self::script::{ResourceMetadata, ScriptLoader};
pub use self::server::{load_client_info, ClientInfo, ServiceId};
pub use self::sprite::*;
pub use self::texture::{TextureAtlasFactory, TextureLoader};

pub const FALLBACK_BMP_FILE: &str = "missing.bmp";
pub const FALLBACK_JPEG_FILE: &str = "missing.jpg";
pub const FALLBACK_PNG_FILE: &str = "missing.png";
pub const FALLBACK_TGA_FILE: &str = "missing.tga";
pub const FALLBACK_MODEL_FILE: &str = "missing.rsm";
pub const FALLBACK_SPRITE_FILE: &str = "npc\\missing.spr";
pub const FALLBACK_ACTIONS_FILE: &str = "npc\\missing.act";
/// The level of mip maps we optimize for (1 base + 3 mip map levels).
pub const MIP_LEVELS: u32 = 4;
