mod cameras;
mod color;
mod memory;
mod particles;
mod renderers;
mod settings;
mod smoothed;
mod transform;
mod vertices;

use std::sync::Arc;

use vulkano::buffer::Subbuffer;
use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};
use vulkano::image::view::ImageView;

pub use self::cameras::*;
pub use self::color::*;
use self::memory::{allocate_descriptor_set, MatrixAllocator};
pub use self::memory::{BufferAllocator, MemoryAllocator};
pub use self::particles::*;
pub use self::renderers::*;
pub use self::settings::GraphicsSettings;
pub use self::smoothed::SmoothedValue;
pub use self::transform::Transform;
pub use self::vertices::*;

pub type CommandBuilder = AutoCommandBufferBuilder<PrimaryAutoCommandBuffer<MemoryAllocator>, MemoryAllocator>;
