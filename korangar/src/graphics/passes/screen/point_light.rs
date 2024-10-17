use bytemuck::{Pod, Zeroable};
use wgpu::{
    include_wgsl, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    BufferBindingType, BufferUsages, ColorTargetState, ColorWrites, Device, FragmentState, MultisampleState, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor,
    ShaderStages, VertexState,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, RenderPassContext, ScreenRenderPassContext,
};
use crate::graphics::*;

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/point_light.wgsl");
const DRAWER_NAME: &str = "screen point light";

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct InstanceData {
    position: [f32; 4],
    color: [f32; 4],
    screen_position: [f32; 2],
    screen_size: [f32; 2],
    range: f32,
    texture_index: i32,
    padding: [u32; 2],
}

pub(crate) struct ScreenPointLightDrawer {
    instance_data_buffer: Buffer<InstanceData>,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
    pipeline: RenderPipeline,
    draw_count: usize,
    instance_data: Vec<InstanceData>,
}

impl Drawer<{ BindGroupCount::Two }, { ColorAttachmentCount::One }, { DepthAttachmentCount::None }> for ScreenPointLightDrawer {
    type Context = ScreenRenderPassContext;
    type DrawData<'data> = Option<()>;

    fn new(device: &Device, _queue: &Queue, _global_context: &GlobalContext, render_pass_context: &Self::Context) -> Self {
        let shader_module = device.create_shader_module(SHADER);

        let instance_data_buffer = Buffer::with_capacity(
            device,
            format!("{DRAWER_NAME} instance data"),
            BufferUsages::COPY_DST | BufferUsages::STORAGE,
            (size_of::<InstanceData>() * NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS) as _,
        );

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some(DRAWER_NAME),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(size_of::<InstanceData>() as _),
                },
                count: None,
            }],
        });

        let bind_group = Self::create_bind_group(device, &bind_group_layout, &instance_data_buffer);

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
            multiview: None,
            primitive: PrimitiveState::default(),
            multisample: MultisampleState::default(),
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
            depth_stencil: None,
            cache: None,
        });

        Self {
            instance_data_buffer,
            bind_group_layout,
            bind_group,
            pipeline,
            draw_count: 0,
            instance_data: Vec::default(),
        }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, _draw_data: Self::DrawData<'_>) {
        if self.draw_count == 0 {
            return;
        }

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(2, &self.bind_group, &[]);
        pass.draw(0..6, 0..self.draw_count as u32);
    }
}

impl Prepare for ScreenPointLightDrawer {
    fn prepare(&mut self, _device: &Device, instructions: &RenderInstruction) {
        self.draw_count = instructions.point_light_shadow_caster.len() + instructions.point_light.len();

        if self.draw_count == 0 {
            return;
        }

        self.instance_data.clear();

        for (instance_index, instruction) in instructions.point_light_shadow_caster.iter().enumerate() {
            self.instance_data.push(InstanceData {
                position: instruction.position.to_homogeneous().into(),
                color: instruction.color.components_linear(),
                screen_position: instruction.screen_position.into(),
                screen_size: instruction.screen_size.into(),
                range: instruction.range,
                texture_index: (instance_index + 1) as i32,
                padding: Default::default(),
            });
        }

        for instruction in instructions.point_light.iter() {
            self.instance_data.push(InstanceData {
                position: instruction.position.to_homogeneous().into(),
                color: instruction.color.components_linear(),
                screen_position: instruction.screen_position.into(),
                screen_size: instruction.screen_size.into(),
                range: instruction.range,
                texture_index: 0,
                padding: Default::default(),
            });
        }
    }

    fn upload(&mut self, device: &Device, staging_belt: &mut StagingBelt, command_encoder: &mut CommandEncoder) {
        let recreated = self
            .instance_data_buffer
            .write(device, staging_belt, command_encoder, &self.instance_data);

        if recreated {
            self.bind_group = Self::create_bind_group(device, &self.bind_group_layout, &self.instance_data_buffer)
        }
    }
}

impl ScreenPointLightDrawer {
    fn create_bind_group(device: &Device, bind_group_layout: &BindGroupLayout, instance_data_buffer: &Buffer<InstanceData>) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some(DRAWER_NAME),
            layout: bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: instance_data_buffer.as_entire_binding(),
            }],
        })
    }
}
