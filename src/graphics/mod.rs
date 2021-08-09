mod transform;
mod vertex;
mod object;
mod camera;
mod renderer;

pub use self::transform::Transform;
pub use self::vertex::Vertex;
pub use self::object::Object;
pub use self::camera::Camera;
pub use self::renderer::Renderer;

use std::sync::Arc;

use vulkano::image::ImmutableImage;
use vulkano::image::view::ImageView;
use vulkano::buffer::CpuAccessibleBuffer;
use vulkano::buffer::cpu_pool::CpuBufferPool;

use vertex_shader::ty::Matrices;

pub use vertex_shader::Shader as VertexShader;

pub use fragment_shader::Shader as FragmentShader;

pub type VertexBuffer = Arc<CpuAccessibleBuffer<[Vertex]>>;

pub type Texture = Arc<ImageView<Arc<ImmutableImage>>>;

pub type MatrixBuffer = CpuBufferPool::<Matrices>;
