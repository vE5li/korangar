mod attachment;
mod buffer;
mod deferred;
mod directional_shadow;
mod interface;
mod picker;
mod point_shadow;
mod sampler;
#[cfg(feature = "debug")]
mod settings;
mod surface;
mod texture;

use std::marker::PhantomData;
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, OnceLock};

use bytemuck::{Pod, Zeroable};
use cgmath::{Matrix4, Point3, Vector2};
use option_ext::OptionExt;
use ragnarok_packets::EntityId;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    BlendComponent, BlendFactor, BlendOperation, BlendState, BufferBindingType, BufferSize, BufferUsages, CommandBuffer, CommandEncoder,
    ComputePass, ComputePassDescriptor, Device, Extent3d, LoadOp, Operations, Queue, RenderPass, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, ShaderStages, StoreOp, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureView,
};

use self::attachment::{AttachmentImageType, AttachmentTextureFactory};
pub use self::buffer::Buffer;
pub use self::deferred::DeferredRenderer;
use self::deferred::DeferredSubRenderer;
pub use self::directional_shadow::{DirectionalShadowRenderer, ShadowDetail};
pub use self::interface::InterfaceRenderer;
use self::picker::PickerSubRenderer;
pub use self::picker::{PickerRenderer, PickerTarget};
pub use self::point_shadow::PointShadowRenderer;
#[cfg(feature = "debug")]
pub use self::settings::RenderSettings;
pub use self::surface::{PresentModeInfo, Surface};
pub use self::texture::{CubeTexture, Texture, TextureGroup};
use super::{Color, ModelVertex};
use crate::graphics::Camera;
use crate::interface::layout::{ScreenClip, ScreenPosition, ScreenSize};
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;

pub const LIGHT_ATTACHMENT_BLEND: BlendState = BlendState {
    color: BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::One,
        operation: BlendOperation::Add,
    },
    alpha: BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::One,
        operation: BlendOperation::Max,
    },
};

pub const WATER_ATTACHMENT_BLEND: BlendState = BlendState {
    color: BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::One,
        operation: BlendOperation::ReverseSubtract,
    },
    alpha: BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::One,
        operation: BlendOperation::Max,
    },
};

pub const INTERFACE_ATTACHMENT_BLEND: BlendState = BlendState {
    color: BlendComponent {
        src_factor: BlendFactor::SrcAlpha,
        dst_factor: BlendFactor::OneMinusSrcAlpha,
        operation: BlendOperation::Add,
    },
    alpha: BlendComponent {
        src_factor: BlendFactor::SrcAlpha,
        dst_factor: BlendFactor::DstAlpha,
        operation: BlendOperation::Max,
    },
};

pub const EFFECT_ATTACHMENT_BLEND: BlendState = BlendState {
    color: BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::One,
        operation: BlendOperation::Max,
    },
    alpha: BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::One,
        operation: BlendOperation::Max,
    },
};

pub const ALPHA_BLEND: BlendState = BlendState {
    color: BlendComponent {
        src_factor: BlendFactor::SrcAlpha,
        dst_factor: BlendFactor::OneMinusSrcAlpha,
        operation: BlendOperation::Add,
    },
    alpha: BlendComponent {
        src_factor: BlendFactor::SrcAlpha,
        dst_factor: BlendFactor::OneMinusSrcAlpha,
        operation: BlendOperation::Add,
    },
};

pub trait Renderer {
    type Target;
}

pub trait GeometryRenderer {
    fn render_geometry(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        vertex_buffer: &Buffer<ModelVertex>,
        textures: &TextureGroup,
        world_matrix: Matrix4<f32>,
        time: f32,
    ) where
        Self: Renderer;
}

pub trait EntityRenderer {
    fn render_entity(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        texture: &Texture,
        position: Point3<f32>,
        origin: Point3<f32>,
        scale: Vector2<f32>,
        cell_count: Vector2<usize>,
        cell_position: Vector2<usize>,
        mirror: bool,
        entity_id: EntityId,
    ) where
        Self: Renderer;
}

pub trait IndicatorRenderer {
    fn render_walk_indicator(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        color: Color,
        upper_left: Point3<f32>,
        upper_right: Point3<f32>,
        lower_left: Point3<f32>,
        lower_right: Point3<f32>,
    ) where
        Self: Renderer;
}

pub trait SpriteRenderer {
    fn render_sprite(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
        texture: &Texture,
        position: ScreenPosition,
        size: ScreenSize,
        screen_clip: ScreenClip,
        color: Color,
        smooth: bool,
    ) where
        Self: Renderer;
}

#[cfg(feature = "debug")]
pub trait MarkerRenderer {
    fn render_marker(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        marker_identifier: MarkerIdentifier,
        position: Point3<f32>,
        hovered: bool,
    ) where
        Self: Renderer;
}

pub struct DeferredRenderTarget {
    diffuse_buffer: Texture,
    normal_buffer: Texture,
    water_buffer: Texture,
    depth_buffer: Texture,
    bound_sub_renderer: Option<DeferredSubRenderer>,
}

impl DeferredRenderTarget {
    pub fn new(device: &Device, dimensions: ScreenSize) -> Self {
        let image_factory = AttachmentTextureFactory::new("deferred render", device, dimensions, 4);

        let diffuse_buffer = image_factory.new_texture("diffuse", Self::output_diffuse_format(), AttachmentImageType::InputColor);
        let normal_buffer = image_factory.new_texture("normal", Self::output_normal_format(), AttachmentImageType::InputColor);
        let water_buffer = image_factory.new_texture("water", Self::output_water_format(), AttachmentImageType::InputColor);
        let depth_buffer = image_factory.new_texture("depth", Self::output_depth_format(), AttachmentImageType::InputDepth);

        let bound_sub_renderer = None;

        Self {
            diffuse_buffer,
            normal_buffer,
            water_buffer,
            depth_buffer,
            bound_sub_renderer,
        }
    }

    pub fn bound_sub_renderer(&mut self, sub_renderer: DeferredSubRenderer) -> bool {
        let already_bound = self.bound_sub_renderer.contains(&sub_renderer);
        self.bound_sub_renderer = Some(sub_renderer);
        !already_bound
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("start frame"))]
    pub fn start_geometry_pass<'encoder>(&mut self, encoder: &'encoder mut CommandEncoder) -> RenderPass<'encoder> {
        let clear_color = wgpu::Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        };

        let render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("deferred render geometry"),
            color_attachments: &[
                Some(RenderPassColorAttachment {
                    view: self.diffuse_buffer.get_texture_view(),
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(clear_color),
                        store: StoreOp::Store,
                    },
                }),
                Some(RenderPassColorAttachment {
                    view: self.normal_buffer.get_texture_view(),
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(clear_color),
                        store: StoreOp::Store,
                    },
                }),
                Some(RenderPassColorAttachment {
                    view: self.water_buffer.get_texture_view(),
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(clear_color),
                        store: StoreOp::Store,
                    },
                }),
            ],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: self.depth_buffer.get_texture_view(),
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(0.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        self.bound_sub_renderer = None;

        render_pass
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("start frame"))]
    pub fn start_screen_pass<'encoder>(&mut self, frame_view: &TextureView, encoder: &'encoder mut CommandEncoder) -> RenderPass<'encoder> {
        let clear_color = wgpu::Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        };

        let render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("deferred render screen"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: frame_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(clear_color),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass
    }

    #[must_use]
    #[cfg_attr(feature = "debug", korangar_debug::profile("finish screen image"))]
    pub fn finish(&mut self, deferred_encoder: CommandEncoder, screen_encoder: CommandEncoder) -> (CommandBuffer, CommandBuffer) {
        (deferred_encoder.finish(), screen_encoder.finish())
    }

    fn output_diffuse_format() -> TextureFormat {
        TextureFormat::Rgba8UnormSrgb
    }

    fn output_normal_format() -> TextureFormat {
        TextureFormat::Rgba16Float
    }

    fn output_water_format() -> TextureFormat {
        TextureFormat::Rgba8UnormSrgb
    }

    fn output_depth_format() -> TextureFormat {
        TextureFormat::Depth32Float
    }
}

pub struct PickerRenderTarget {
    pub texture: Texture,
    depth_texture: Texture,
    buffer: Buffer<u32>,
    bound_sub_renderer: Option<PickerSubRenderer>,
}

impl PickerRenderTarget {
    pub fn new(device: &Device, dimensions: ScreenSize) -> Self {
        let texture_factory = AttachmentTextureFactory::new("picker render", device, dimensions, 1);

        let texture = texture_factory.new_texture("color", Self::output_color_format(), AttachmentImageType::CopyColor);
        let depth_texture = texture_factory.new_texture("depth", Self::depth_texture_format(), AttachmentImageType::Depth);

        let buffer = Buffer::with_capacity(
            device,
            "picker value",
            BufferUsages::STORAGE | BufferUsages::MAP_READ,
            Self::picker_value_size() as _,
        );

        let bound_sub_renderer = None;

        Self {
            texture,
            depth_texture,
            buffer,
            bound_sub_renderer,
        }
    }

    /// Reads the picker value.
    #[cfg_attr(feature = "debug", korangar_debug::profile("queue read for picker value"))]
    pub fn queue_read_picker_value(&mut self, return_value: Arc<AtomicU32>) {
        self.buffer.queue_read_u32(return_value);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("start render pass"))]
    pub fn start_render_pass<'encoder>(&mut self, encoder: &'encoder mut CommandEncoder) -> RenderPass<'encoder> {
        let clear_color = wgpu::Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        };

        let render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("picker render"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: self.texture.get_texture_view(),
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(clear_color),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: self.depth_texture.get_texture_view(),
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(0.0),
                    store: StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        self.bound_sub_renderer = None;

        render_pass
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("start compute pass"))]
    pub fn start_compute_pass<'encoder>(&mut self, encoder: &'encoder mut CommandEncoder) -> ComputePass<'encoder> {
        let render_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("picker compute"),
            timestamp_writes: None,
        });

        self.bound_sub_renderer = None;

        render_pass
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn bound_sub_renderer(&mut self, sub_renderer: PickerSubRenderer) -> bool {
        let already_bound = self.bound_sub_renderer.contains(&sub_renderer);
        self.bound_sub_renderer = Some(sub_renderer);
        !already_bound
    }

    #[must_use]
    #[cfg_attr(feature = "debug", korangar_debug::profile("finish buffer"))]
    pub fn finish(&mut self, render_encoder: CommandEncoder, compute_encoder: CommandEncoder) -> (CommandBuffer, CommandBuffer) {
        (render_encoder.finish(), compute_encoder.finish())
    }

    pub fn picker_value_size() -> u32 {
        Self::output_color_format().target_pixel_byte_cost().unwrap()
    }

    pub const fn output_color_format() -> TextureFormat {
        // TODO: NHA We could use Rg32Uint for 64 bit range.
        TextureFormat::R32Uint
    }

    pub const fn depth_texture_format() -> TextureFormat {
        // TODO: NHA Re-use the depth texture between render passes.
        TextureFormat::Depth32Float
    }
}

pub trait IntoFormat {
    fn into_format() -> TextureFormat;
}

pub struct SingleRenderTarget<F: IntoFormat, S: PartialEq, C> {
    pub texture: Arc<Texture>,
    clear_value: C,
    bound_sub_renderer: Option<S>,
    name: &'static str,
    _phantom_data: PhantomData<F>,
}

impl<F: IntoFormat, S: PartialEq, C> SingleRenderTarget<F, S, C> {
    pub fn new(
        device: &Device,
        name: &'static str,
        dimensions: ScreenSize,
        sample_count: u32,
        texture_usage: TextureUsages,
        clear_value: C,
    ) -> Self {
        let texture = Arc::new(Texture::new(device, &TextureDescriptor {
            label: Some(name),
            size: Extent3d {
                width: dimensions.width as u32,
                height: dimensions.height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: TextureDimension::D2,
            format: F::into_format(),
            usage: texture_usage,
            view_formats: &[],
        }));

        let bound_sub_renderer = None;

        Self {
            texture,
            clear_value,
            bound_sub_renderer,
            name,
            _phantom_data: Default::default(),
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn bind_sub_renderer(&mut self, sub_renderer: S) -> bool {
        let already_bound = self.bound_sub_renderer.contains(&sub_renderer);
        self.bound_sub_renderer = Some(sub_renderer);
        !already_bound
    }

    pub fn output_texture_format() -> TextureFormat {
        F::into_format()
    }
}

impl<F: IntoFormat, S: PartialEq> SingleRenderTarget<F, S, wgpu::Color> {
    #[cfg_attr(feature = "debug", korangar_debug::profile("start frame"))]
    pub fn start<'encoder>(&mut self, encoder: &'encoder mut CommandEncoder, clear_interface: bool) -> RenderPass<'encoder> {
        let render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(self.name),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: self.texture.get_texture_view(),
                resolve_target: None,
                ops: Operations {
                    load: if clear_interface {
                        LoadOp::Clear(self.clear_value)
                    } else {
                        LoadOp::Load
                    },
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        self.bound_sub_renderer = None;

        render_pass
    }

    #[must_use]
    #[cfg_attr(feature = "debug", korangar_debug::profile("finalize buffer"))]
    pub fn finish(&mut self, encoder: CommandEncoder) -> CommandBuffer {
        encoder.finish()
    }
}

impl<F: IntoFormat, S: PartialEq> SingleRenderTarget<F, S, f32> {
    #[cfg_attr(feature = "debug", korangar_debug::profile("start frame"))]
    pub fn start<'encoder>(&mut self, encoder: &'encoder mut CommandEncoder) -> RenderPass<'encoder> {
        let render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(self.name),
            color_attachments: &[],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: self.texture.get_texture_view(),
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(self.clear_value),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        self.bound_sub_renderer = None;

        render_pass
    }

    #[must_use]
    #[cfg_attr(feature = "debug", korangar_debug::profile("finalize buffer"))]
    pub fn finish(&mut self, encoder: CommandEncoder) -> CommandBuffer {
        encoder.finish()
    }
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct ViewProjectionMatrix([[f32; 4]; 4]);

impl ViewProjectionMatrix {
    const fn memory_size() -> u64 {
        std::mem::size_of::<ViewProjectionMatrix>() as u64
    }
}

pub struct CubeFaceBuffer {
    buffer: Buffer<ViewProjectionMatrix>,
    bind_group: BindGroup,
}

impl CubeFaceBuffer {
    pub fn bind_group_layout(device: &Device) -> &BindGroupLayout {
        static LAYOUT: OnceLock<BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("view projection matrix"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(ViewProjectionMatrix::memory_size()),
                    },
                    count: None,
                }],
            })
        })
    }

    pub fn new(device: &Device, name: &'static str) -> Self {
        let buffer = Buffer::with_capacity(
            device,
            name,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            ViewProjectionMatrix::memory_size(),
        );

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("geometry"),
            layout: Self::bind_group_layout(device),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self { buffer, bind_group }
    }
}

pub struct CubeRenderTarget<S: PartialEq> {
    pub texture: Arc<CubeTexture>,
    faces: [CubeFaceBuffer; 6],
    bound_sub_renderer: Option<S>,
    name: &'static str,
    index: usize,
}

impl<S: PartialEq> CubeRenderTarget<S> {
    pub fn new(device: &Device, name: &'static str, dimensions: [u32; 2], texture_usage: TextureUsages) -> Self {
        let texture = Arc::new(CubeTexture::new(device, &TextureDescriptor {
            label: Some(name),
            size: Extent3d {
                width: dimensions[0],
                height: dimensions[1],
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: texture_usage,
            view_formats: &[],
        }));

        let matrix_buffers = [
            CubeFaceBuffer::new(device, "cube face 0"),
            CubeFaceBuffer::new(device, "cube face 1"),
            CubeFaceBuffer::new(device, "cube face 2"),
            CubeFaceBuffer::new(device, "cube face 3"),
            CubeFaceBuffer::new(device, "cube face 4"),
            CubeFaceBuffer::new(device, "cube face 5"),
        ];

        Self {
            texture,
            faces: matrix_buffers,
            bound_sub_renderer: None,
            name,
            index: 0,
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("start frame"))]
    pub fn start<'encoder>(
        &mut self,
        queue: &Queue,
        encoder: &'encoder mut CommandEncoder,
        index: u32,
        matrix: Matrix4<f32>,
    ) -> RenderPass<'encoder> {
        let render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(self.name),
            color_attachments: &[],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: self.texture.get_texture_face_view(index as usize),
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        self.bound_sub_renderer = None;

        self.faces[index as usize]
            .buffer
            .write_exact(queue, &[ViewProjectionMatrix(matrix.into())]);
        self.index = index as usize;

        render_pass
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn bind_sub_renderer(&mut self, sub_renderer: S) -> bool {
        let already_bound = self.bound_sub_renderer.contains(&sub_renderer);
        self.bound_sub_renderer = Some(sub_renderer);
        !already_bound
    }

    #[must_use]
    #[cfg_attr(feature = "debug", korangar_debug::profile("finalize buffer"))]
    pub fn finish(&mut self, encoder: CommandEncoder) -> CommandBuffer {
        encoder.finish()
    }

    pub fn output_texture_format() -> TextureFormat {
        TextureFormat::Depth32Float
    }

    pub fn face_bind_group(&self) -> &BindGroup {
        &self.faces[self.index].bind_group
    }
}
