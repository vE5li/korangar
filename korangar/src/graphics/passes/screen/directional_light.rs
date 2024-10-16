use bytemuck::{Pod, Zeroable};
use wgpu::util::StagingBelt;
use wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingType, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites, CommandEncoder, Device, FragmentState, MultisampleState,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor,
    ShaderModuleDescriptor, ShaderStages, VertexState,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, RenderPassContext, ScreenRenderPassContext,
};
use crate::graphics::{GlobalContext, Prepare, RenderInstruction, LIGHT_ATTACHMENT_BLEND};
use crate::Buffer;

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/directional_light.wgsl");
const DRAWER_NAME: &str = "screen directional light";

#[derive(Copy, Clone, Default, Pod, Zeroable)]
#[repr(C)]
pub(crate) struct DirectionalLightUniforms {
    view_projection: [[f32; 4]; 4],
    color: [f32; 4],
    direction: [f32; 4],
}

pub(crate) struct ScreenDirectionalLightDrawer {
    buffer: Buffer<DirectionalLightUniforms>,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
    pipeline: RenderPipeline,
    light_uniforms: DirectionalLightUniforms,
}

impl Drawer<{ BindGroupCount::Two }, { ColorAttachmentCount::One }, { DepthAttachmentCount::None }> for ScreenDirectionalLightDrawer {
    type Context = ScreenRenderPassContext;
    type DrawData<'data> = Option<()>;

    fn new(device: &Device, _queue: &Queue, _global_context: &GlobalContext, render_pass_context: &Self::Context) -> Self {
        let shader_module = device.create_shader_module(SHADER);

        let buffer = Buffer::with_capacity(
            device,
            format!("{DRAWER_NAME} uniforms"),
            BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            size_of::<DirectionalLightUniforms>() as u64,
        );

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some(DRAWER_NAME),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: buffer.byte_capacity(),
                },
                count: None,
            }],
        });

        let bind_group = Self::create_bind_group(device, &bind_group_layout, &buffer);

        let bind_group_layouts = Self::Context::bind_group_layout(device);

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts: &[bind_group_layouts[0], bind_group_layouts[1], &bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(DRAWER_NAME),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: render_pass_context.color_attachment_formats()[0],
                    blend: Some(LIGHT_ATTACHMENT_BLEND),
                    write_mask: ColorWrites::default(),
                })],
            }),
            multiview: None,
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            cache: None,
        });

        Self {
            buffer,
            bind_group_layout,
            bind_group,
            pipeline,
            light_uniforms: DirectionalLightUniforms::default(),
        }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, _draw_data: Self::DrawData<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(2, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

impl Prepare for ScreenDirectionalLightDrawer {
    fn prepare(&mut self, _device: &Device, instructions: &RenderInstruction) {
        self.light_uniforms = DirectionalLightUniforms {
            view_projection: instructions.directional_light_with_shadow.view_projection_matrix.into(),
            color: instructions.directional_light_with_shadow.color.components_linear(),
            direction: instructions.directional_light_with_shadow.direction.extend(0.0).into(),
        };
    }

    fn upload(&mut self, device: &Device, staging_belt: &mut StagingBelt, command_encoder: &mut CommandEncoder) {
        let recreated = self.buffer.write(device, staging_belt, command_encoder, &[self.light_uniforms]);

        if recreated {
            self.bind_group = Self::create_bind_group(device, &self.bind_group_layout, &self.buffer);
        }
    }
}

impl ScreenDirectionalLightDrawer {
    fn create_bind_group(device: &Device, bind_group_layout: &BindGroupLayout, buffer: &Buffer<DirectionalLightUniforms>) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some(DRAWER_NAME),
            layout: bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        })
    }
}
