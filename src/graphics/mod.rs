mod color;
mod smoothed;
mod transform;
mod vertex;
//mod object;
mod camera;
mod lighting;
mod renderer;

pub use self::color::Color;
pub use self::smoothed::SmoothedValue;
pub use self::transform::Transform;
pub use self::vertex::Vertex;
//pub use self::object::Object;
pub use self::camera::Camera;
pub use self::lighting::*;
pub use self::renderer::Renderer;

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

mod deferred_vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/deferred_vertex_shader.glsl"
    }
}

mod deferred_fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/deferred_fragment_shader.glsl"
    }
}

pub use self::deferred_vertex_shader::ty::Matrices;

pub use self::deferred_vertex_shader::Shader as DeferredVertexShader;

pub use self::deferred_fragment_shader::Shader as DeferredFragmentShader;

pub type CommandBuilder = AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>;

pub type VertexBuffer = Arc<CpuAccessibleBuffer<[Vertex]>>;

pub type ScreenVertexBuffer = Arc<CpuAccessibleBuffer<[ScreenVertex]>>;

pub type Texture = Arc<ImageView<Arc<ImmutableImage>>>;

pub type ImageBuffer = Arc<ImageView<Arc<AttachmentImage>>>;

pub type MatrixBuffer = CpuBufferPool::<Matrices>;

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
