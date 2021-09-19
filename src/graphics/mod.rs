mod color;
mod transform;
mod vertices;
mod camera;
mod renderer;

use std::sync::Arc;

use vulkano::command_buffer::{ AutoCommandBufferBuilder, PrimaryAutoCommandBuffer };
use vulkano::image::view::ImageView;
use vulkano::image::attachment::AttachmentImage;
use vulkano::image::ImmutableImage;
use vulkano::buffer::CpuAccessibleBuffer;
use vulkano::buffer::cpu_pool::CpuBufferPool;
use vulkano::pipeline::blend::AttachmentBlend;
use vulkano::pipeline::blend::BlendFactor;
use vulkano::pipeline::blend::BlendOp;

pub use self::color::Color;
pub use self::transform::Transform;
pub use self::vertices::{ ScreenVertex, Vertex };
pub use self::camera::Camera;
pub use self::renderer::Renderer;

pub type CommandBuilder = AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>;

pub type VertexBuffer = Arc<CpuAccessibleBuffer<[Vertex]>>;

pub type ScreenVertexBuffer = Arc<CpuAccessibleBuffer<[ScreenVertex]>>;

pub type Texture = Arc<ImageView<Arc<ImmutableImage>>>;

pub type ImageBuffer = Arc<ImageView<Arc<AttachmentImage>>>;

pub const LIGHT_ATTACHMENT_BLEND: AttachmentBlend = AttachmentBlend {
    enabled: true,
    color_op: BlendOp::Add,
    color_source: BlendFactor::One,
    color_destination: BlendFactor::One,
    alpha_op: BlendOp::Max,
    alpha_source: BlendFactor::One,
    alpha_destination: BlendFactor::One,
    mask_red: true,
    mask_green: true,
    mask_blue: true,
    mask_alpha: true,
};
