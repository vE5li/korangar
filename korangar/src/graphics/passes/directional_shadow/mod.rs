mod entity;
mod indicator;
mod model;

use std::num::NonZeroU64;
use std::sync::OnceLock;

use bytemuck::{Pod, Zeroable};
use cgmath::{Matrix4, SquareMatrix};
pub(crate) use entity::DirectionalShadowEntityDrawer;
pub(crate) use indicator::DirectionalShadowIndicatorDrawer;
pub(crate) use model::DirectionalShadowModelDrawer;
use wgpu::util::StagingBelt;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    BufferBindingType, BufferUsages, CommandEncoder, Device, LoadOp, Operations, Queue, RenderPass, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, ShaderStages, StoreOp, TextureFormat,
};

use super::{BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, RenderPassContext};
use crate::graphics::{Buffer, GlobalContext, Prepare, RenderInstruction};
use crate::loaders::TextureLoader;

const PASS_NAME: &str = "directional shadow render pass";

#[derive(Copy, Clone, Default, Pod, Zeroable)]
#[repr(C)]
struct PassUniforms {
    view_projection: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
    inverse_view: [[f32; 4]; 4],
    animation_timer: f32,
    padding: [u32; 3],
}

pub(crate) struct DirectionalShadowRenderPassContext {
    uniforms_buffer: Buffer<PassUniforms>,
    bind_group: BindGroup,
    directional_shadow_texture_format: TextureFormat,
    uniforms_data: PassUniforms,
}

impl RenderPassContext<{ BindGroupCount::Two }, { ColorAttachmentCount::None }, { DepthAttachmentCount::One }>
    for DirectionalShadowRenderPassContext
{
    type PassData<'data> = Option<()>;

    fn new(device: &Device, _queue: &Queue, _texture_loader: &TextureLoader, global_context: &GlobalContext) -> Self {
        let uniforms_buffer = Buffer::with_capacity(
            device,
            format!("{PASS_NAME} uniforms"),
            BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            size_of::<PassUniforms>() as _,
        );

        let bind_group = Self::create_bind_group(device, &uniforms_buffer);

        let directional_shadow_texture_format = global_context.directional_shadow_map_texture.get_format();

        Self {
            uniforms_buffer,
            bind_group,
            directional_shadow_texture_format,
            uniforms_data: Default::default(),
        }
    }

    fn create_pass<'encoder>(
        &mut self,
        encoder: &'encoder mut CommandEncoder,
        global_context: &GlobalContext,
        _pass_data: Self::PassData<'_>,
    ) -> RenderPass<'encoder> {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(PASS_NAME),
            color_attachments: &[],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: global_context.directional_shadow_map_texture.get_texture_view(),
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(0.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_viewport(
            0.0,
            0.0,
            global_context.directional_shadow_size.width,
            global_context.directional_shadow_size.height,
            0.0,
            1.0,
        );
        pass.set_bind_group(0, &global_context.global_bind_group, &[]);
        pass.set_bind_group(1, &self.bind_group, &[]);

        pass
    }

    fn bind_group_layout(device: &Device) -> [&'static BindGroupLayout; 2] {
        static LAYOUT: OnceLock<BindGroupLayout> = OnceLock::new();

        let layout = LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(PASS_NAME),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(size_of::<PassUniforms>() as _),
                    },
                    count: None,
                }],
            })
        });

        [GlobalContext::global_bind_group_layout(device), layout]
    }

    fn color_attachment_formats(&self) -> [TextureFormat; 0] {
        []
    }

    fn depth_attachment_output_format(&self) -> [TextureFormat; 1] {
        [self.directional_shadow_texture_format]
    }
}

impl Prepare for DirectionalShadowRenderPassContext {
    fn prepare(&mut self, _device: &Device, instructions: &RenderInstruction) {
        self.uniforms_data = PassUniforms {
            view_projection: instructions.directional_light_with_shadow.view_projection_matrix.into(),
            view: instructions.directional_light_with_shadow.view_matrix.into(),
            inverse_view: instructions
                .directional_light_with_shadow
                .view_matrix
                .invert()
                .unwrap_or(Matrix4::identity())
                .into(),
            animation_timer: instructions.uniforms.animation_timer,
            padding: Default::default(),
        };
    }

    fn upload(&mut self, device: &Device, staging_belt: &mut StagingBelt, command_encoder: &mut CommandEncoder) {
        let recreated = self
            .uniforms_buffer
            .write(device, staging_belt, command_encoder, &[self.uniforms_data]);

        if recreated {
            self.bind_group = Self::create_bind_group(device, &self.uniforms_buffer);
        }
    }
}

impl DirectionalShadowRenderPassContext {
    fn create_bind_group(device: &Device, pass_uniforms_buffer: &Buffer<PassUniforms>) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some(PASS_NAME),
            layout: Self::bind_group_layout(device)[1],
            entries: &[BindGroupEntry {
                binding: 0,
                resource: pass_uniforms_buffer.as_entire_binding(),
            }],
        })
    }
}
