mod entity;
mod indicator;
mod model;

use std::sync::OnceLock;

use bytemuck::{Pod, Zeroable};
use cgmath::{Matrix4, SquareMatrix};
pub(crate) use entity::{
    DirectionalShadowEntityDrawData, DirectionalShadowEntityDrawer, EntityPassMode as DirectionalShadowEntityPassMode,
};
pub(crate) use indicator::DirectionalShadowIndicatorDrawer;
pub(crate) use model::DirectionalShadowModelDrawer;
use wgpu::util::StagingBelt;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, Color,
    CommandEncoder, Device, LoadOp, Operations, Queue, RenderPass, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, ShaderStages, StoreOp, TextureFormat,
};

use super::{BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, RenderPassContext};
use crate::graphics::buffer::DynamicUniformBuffer;
use crate::graphics::{GlobalContext, Prepare, RenderInstruction};
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
    uniforms_buffer: DynamicUniformBuffer<PassUniforms>,
    bind_group: BindGroup,
    directional_shadow_texture_format: TextureFormat,
    directional_shadow_translucence_format: TextureFormat,
}

impl RenderPassContext<{ BindGroupCount::Two }, { ColorAttachmentCount::One }, { DepthAttachmentCount::One }>
    for DirectionalShadowRenderPassContext
{
    type PassData<'data> = usize;

    fn new(device: &Device, _queue: &Queue, _texture_loader: &TextureLoader, global_context: &GlobalContext) -> Self {
        let uniforms_buffer = DynamicUniformBuffer::new(device, &format!("{PASS_NAME} uniforms"));

        let bind_group = Self::create_bind_group(device, &uniforms_buffer);

        let directional_shadow_texture_format = global_context.directional_shadow_map_texture.get_format();
        let directional_shadow_translucence_format = global_context.directional_shadow_translucence_texture.get_format();

        Self {
            uniforms_buffer,
            bind_group,
            directional_shadow_texture_format,
            directional_shadow_translucence_format,
        }
    }

    fn create_pass<'encoder>(
        &mut self,
        encoder: &'encoder mut CommandEncoder,
        global_context: &GlobalContext,
        pass_data: Self::PassData<'_>,
    ) -> RenderPass<'encoder> {
        let dynamic_offset = self.uniforms_buffer.dynamic_offset(pass_data);

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(PASS_NAME),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: global_context
                    .directional_shadow_translucence_texture
                    .get_array_texture_view(pass_data),
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::WHITE),
                    store: StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: global_context.directional_shadow_map_texture.get_array_texture_view(pass_data),
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
        pass.set_bind_group(1, &self.bind_group, &[dynamic_offset]);

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
                    ty: DynamicUniformBuffer::<PassUniforms>::get_binding_type(),
                    count: None,
                }],
            })
        });

        [GlobalContext::global_bind_group_layout(device), layout]
    }

    fn color_attachment_formats(&self) -> [TextureFormat; 1] {
        [self.directional_shadow_translucence_format]
    }

    fn depth_attachment_output_format(&self) -> [TextureFormat; 1] {
        [self.directional_shadow_texture_format]
    }
}

impl Prepare for DirectionalShadowRenderPassContext {
    fn prepare(&mut self, _device: &Device, instructions: &RenderInstruction) {
        let uniforms = instructions.directional_light_partitions.iter().map(|caster| PassUniforms {
            view_projection: caster.view_projection_matrix.into(),
            view: caster.view_matrix.into(),
            inverse_view: caster.view_matrix.invert().unwrap_or(Matrix4::identity()).into(),
            animation_timer: instructions.uniforms.animation_timer_ms / 1000.0,
            padding: Default::default(),
        });
        self.uniforms_buffer.write_data(uniforms);
    }

    fn upload(&mut self, device: &Device, staging_belt: &mut StagingBelt, command_encoder: &mut CommandEncoder) {
        let recreated = self.uniforms_buffer.upload(device, staging_belt, command_encoder);

        if recreated {
            self.bind_group = Self::create_bind_group(device, &self.uniforms_buffer);
        }
    }
}

impl DirectionalShadowRenderPassContext {
    fn create_bind_group(device: &Device, uniforms_buffer: &DynamicUniformBuffer<PassUniforms>) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some(PASS_NAME),
            layout: Self::bind_group_layout(device)[1],
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniforms_buffer.get_binding_resource(),
            }],
        })
    }
}
