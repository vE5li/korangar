mod entity;
mod indicator;
mod model;

use std::sync::OnceLock;

use bytemuck::{Pod, Zeroable};
pub(crate) use entity::PointShadowEntityDrawer;
pub(crate) use indicator::PointShadowIndicatorDrawer;
pub(crate) use model::PointShadowModelDrawer;
use wgpu::util::StagingBelt;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, CommandEncoder,
    Device, LoadOp, Operations, Queue, RenderPass, RenderPassDepthStencilAttachment, RenderPassDescriptor, ShaderStages, StoreOp,
    TextureFormat,
};

use super::{BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, RenderPassContext};
use crate::graphics::buffer::DynamicUniformBuffer;
use crate::graphics::{EntityInstruction, GlobalContext, ModelInstruction, PointShadowCasterInstruction, Prepare, RenderInstruction};
use crate::loaders::TextureLoader;

const PASS_NAME: &str = "point shadow render pass";
const NUMBER_FACES: usize = 6;

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct PassUniforms {
    view_projection: [[f32; 4]; 4],
    light_position: [f32; 4],
    animation_timer: f32,
    padding: [u32; 3],
}

#[derive(Copy, Clone)]
pub(crate) struct PointShadowData {
    pub(crate) shadow_caster_index: usize,
    pub(crate) face_index: usize,
}

pub(crate) struct PointShadowEntityBatchData<'a> {
    pub(crate) pass_data: PointShadowData,
    pub(crate) caster: &'a [PointShadowCasterInstruction],
    pub(crate) instructions: &'a [EntityInstruction],
}

pub(crate) struct PointShadowModelBatchData<'a> {
    pub(crate) pass_data: PointShadowData,
    pub(crate) caster: &'a [PointShadowCasterInstruction],
    pub(crate) instructions: &'a [ModelInstruction],
}

pub(crate) struct PointShadowRenderPassContext {
    point_shadow_texture_format: TextureFormat,
    uniforms_buffer: DynamicUniformBuffer<PassUniforms>,
    bind_group: BindGroup,
}

impl RenderPassContext<{ BindGroupCount::Two }, { ColorAttachmentCount::None }, { DepthAttachmentCount::One }>
    for PointShadowRenderPassContext
{
    type PassData<'data> = PointShadowData;

    fn new(device: &Device, _queue: &Queue, _texture_loader: &TextureLoader, global_context: &GlobalContext) -> Self {
        let point_shadow_texture_format = global_context.point_shadow_map_textures.get_texture_format();

        let uniforms_buffer = DynamicUniformBuffer::new(device, &format!("{PASS_NAME} pass uniforms"));

        let bind_group = Self::create_bind_group(device, &uniforms_buffer);

        Self {
            point_shadow_texture_format,
            uniforms_buffer,
            bind_group,
        }
    }

    fn create_pass<'encoder>(
        &mut self,
        encoder: &'encoder mut CommandEncoder,
        global_context: &GlobalContext,
        pass_data: PointShadowData,
    ) -> RenderPass<'encoder> {
        let dynamic_offset = self
            .uniforms_buffer
            .dynamic_offset(pass_data.shadow_caster_index * NUMBER_FACES + pass_data.face_index);

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(PASS_NAME),
            color_attachments: &[],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: global_context
                    .point_shadow_map_textures
                    .get_texture_face_view(pass_data.shadow_caster_index, pass_data.face_index),
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
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
            global_context.point_shadow_size.width,
            global_context.point_shadow_size.height,
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

    fn color_attachment_formats(&self) -> [TextureFormat; 0] {
        []
    }

    fn depth_attachment_output_format(&self) -> [TextureFormat; 1] {
        [self.point_shadow_texture_format]
    }
}

impl Prepare for PointShadowRenderPassContext {
    fn prepare(&mut self, _device: &Device, instructions: &RenderInstruction) {
        let uniforms = instructions.point_light_shadow_caster.iter().flat_map(|caster| {
            (0..NUMBER_FACES).map(|face_index| PassUniforms {
                view_projection: caster.view_projection_matrices[face_index].into(),
                light_position: caster.position.to_homogeneous().into(),
                animation_timer: instructions.uniforms.animation_timer,
                padding: Default::default(),
            })
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

impl PointShadowRenderPassContext {
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
