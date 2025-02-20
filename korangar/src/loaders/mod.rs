mod action;
mod animation;
mod archive;

mod r#async;
mod cache;
mod effect;
pub mod error;
mod font;
mod gamefile;
mod map;
mod model;
mod server;
mod smoothing;
mod sprite;
mod texture;

pub use self::action::*;
pub use self::animation::*;
pub use self::r#async::*;
pub use self::cache::{Cache, CachedTextureAtlas};
pub use self::effect::EffectLoader;
pub use self::font::{FontLoader, FontSize, GlyphInstruction, Scaling};
pub use self::gamefile::*;
pub use self::map::{MAP_TILE_SIZE, MapLoader};
pub use self::model::*;
pub use self::server::{ClientInfo, ServiceId, load_client_info};
pub use self::smoothing::{smooth_ground_normals, smooth_model_normals};
pub use self::sprite::*;
pub use self::texture::{ImageType, TextureAtlas, TextureAtlasEntry, TextureLoader, UncompressedTextureAtlas};

pub const FALLBACK_BMP_FILE: &str = "missing.bmp";
pub const FALLBACK_JPEG_FILE: &str = "missing.jpg";
pub const FALLBACK_PNG_FILE: &str = "missing.png";
pub const FALLBACK_TGA_FILE: &str = "missing.tga";
pub const FALLBACK_MODEL_FILE: &str = "missing.rsm";
pub const FALLBACK_SPRITE_FILE: &str = "npc\\missing.spr";
pub const FALLBACK_ACTIONS_FILE: &str = "npc\\missing.act";
/// The level of mip maps we optimize for (1 base + 3 mip map levels).
pub const MIP_LEVELS: u32 = 4;
