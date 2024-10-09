mod cameras;
mod color;
#[cfg(feature = "debug")]
mod error;
mod features;
mod particles;
mod renderers;
mod settings;
mod smoothed;
mod vertices;

use cgmath::Matrix4;

pub use self::cameras::*;
pub use self::color::*;
#[cfg(feature = "debug")]
pub use self::error::error_handler;
pub use self::features::*;
pub use self::particles::*;
pub use self::renderers::*;
pub use self::settings::GraphicsSettings;
pub use self::smoothed::SmoothedValue;
pub use self::vertices::*;

pub struct GeometryInstruction {
    pub world_matrix: Matrix4<f32>,
    pub vertex_offset: u32,
    pub vertex_count: u32,
}
