mod types;
mod vertices;
mod renderer;
pub mod camera;

use std::sync::Arc;

use vulkano::command_buffer::{ AutoCommandBufferBuilder, PrimaryAutoCommandBuffer };
use vulkano::image::view::ImageView;
use vulkano::image::attachment::AttachmentImage;
use vulkano::image::ImmutableImage;
use vulkano::buffer::CpuAccessibleBuffer;
use vulkano::buffer::cpu_pool::CpuBufferPool;
use vulkano::pipeline::graphics::color_blend::AttachmentBlend;
use vulkano::pipeline::graphics::color_blend::BlendFactor;
use vulkano::pipeline::graphics::color_blend::BlendOp;

pub use self::types::*;
pub use self::vertices::*;
pub use self::renderer::{ Renderer, RenderSettings };
pub use self::camera::*;

pub type CommandBuilder = AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>;

pub type ModelVertexBuffer = Arc<CpuAccessibleBuffer<[ModelVertex]>>;

pub type WaterVertexBuffer = Arc<CpuAccessibleBuffer<[WaterVertex]>>;

pub type ScreenVertexBuffer = Arc<CpuAccessibleBuffer<[ScreenVertex]>>;

pub type TileVertexBuffer = Arc<CpuAccessibleBuffer<[TileVertex]>>;

pub type Texture = Arc<ImageView<Arc<ImmutableImage>>>;

pub type ImageBuffer = Arc<ImageView<Arc<AttachmentImage>>>;

pub const LIGHT_ATTACHMENT_BLEND: AttachmentBlend = AttachmentBlend {
    color_op: BlendOp::Add,
    color_source: BlendFactor::One,
    color_destination: BlendFactor::One,
    alpha_op: BlendOp::Max,
    alpha_source: BlendFactor::One,
    alpha_destination: BlendFactor::One,
};

pub const WATER_ATTACHMENT_BLEND: AttachmentBlend = AttachmentBlend {
    color_op: BlendOp::ReverseSubtract,
    color_source: BlendFactor::One,
    color_destination: BlendFactor::One,
    alpha_op: BlendOp::Max,
    alpha_source: BlendFactor::One,
    alpha_destination: BlendFactor::One,
};

pub const INTERFACE_ATTACHMENT_BLEND: AttachmentBlend = AttachmentBlend {
    color_op: BlendOp::Add,
    color_source: BlendFactor::SrcAlpha,
    color_destination: BlendFactor::OneMinusSrcAlpha,
    alpha_op: BlendOp::Add,
    alpha_source: BlendFactor::SrcAlpha,
    alpha_destination: BlendFactor::DstAlpha,
};
