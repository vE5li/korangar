mod color;
mod transform;
mod smoothed;
mod vertices;
mod renderers;
mod cameras;
mod particles;

use std::sync::Arc;

use vulkano::command_buffer::{ AutoCommandBufferBuilder, PrimaryAutoCommandBuffer };
use vulkano::image::view::ImageView;
use vulkano::image::attachment::AttachmentImage;
use vulkano::image::ImmutableImage;
use vulkano::buffer::CpuAccessibleBuffer;
use vulkano::buffer::cpu_pool::CpuBufferPool;

pub use self::color::*;
pub use self::transform::Transform;
pub use self::smoothed::SmoothedValue;
pub use self::vertices::*;
pub use self::renderers::*;
pub use self::cameras::*;
pub use self::particles::*;

pub type CommandBuilder = AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>;

pub type ModelVertexBuffer = Arc<CpuAccessibleBuffer<[ModelVertex]>>;

pub type WaterVertexBuffer = Arc<CpuAccessibleBuffer<[WaterVertex]>>;

pub type ScreenVertexBuffer = Arc<CpuAccessibleBuffer<[ScreenVertex]>>;

pub type TileVertexBuffer = Arc<CpuAccessibleBuffer<[TileVertex]>>;

pub type Texture = Arc<ImageView<Arc<ImmutableImage>>>;

pub type ImageBuffer = Arc<ImageView<Arc<AttachmentImage>>>;
