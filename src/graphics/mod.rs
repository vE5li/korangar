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

use vulkano::buffer::cpu_pool::CpuBufferPool;
use vulkano::buffer::CpuAccessibleBuffer;
use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};
use vulkano::image::attachment::AttachmentImage;
use vulkano::image::view::ImageView;
use vulkano::image::ImmutableImage;

pub use self::cameras::*;
pub use self::color::*;
pub use self::memory::MemoryAllocator;
pub use self::particles::*;
pub use self::renderers::*;
pub use self::settings::GraphicsSettings;
pub use self::smoothed::SmoothedValue;
pub use self::transform::Transform;
pub use self::vertices::*;

pub type CommandBuilder = AutoCommandBufferBuilder<PrimaryAutoCommandBuffer, MemoryAllocator>;

pub type ModelVertexBuffer = Arc<CpuAccessibleBuffer<[ModelVertex]>>;

pub type WaterVertexBuffer = Arc<CpuAccessibleBuffer<[WaterVertex]>>;

pub type TileVertexBuffer = Arc<CpuAccessibleBuffer<[TileVertex]>>;

pub type Texture = Arc<ImageView<Arc<ImmutableImage>>>;

pub type ImageBuffer = Arc<ImageView<Arc<AttachmentImage>>>;
