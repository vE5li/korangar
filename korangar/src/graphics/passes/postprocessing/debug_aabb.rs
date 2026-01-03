use std::num::NonZeroU64;

use bytemuck::{Pod, Zeroable};
use cgmath::Point3;
use wgpu::util::StagingBelt;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    BufferBindingType, BufferUsages, ColorTargetState, ColorWrites, CommandEncoder, Device, FragmentState, IndexFormat, MultisampleState,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, Queue, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, ShaderStages, VertexState,
};

use crate::Buffer;
use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, PostProcessingRenderPassContext, RenderPassContext,
};
use crate::graphics::shader_compiler::ShaderCompiler;
use crate::graphics::{Capabilities, GlobalContext, Prepare, RenderInstruction, SimpleVertex};

const DRAWER_NAME: &str = "debug aabb";
const INITIAL_INSTRUCTION_SIZE: usize = 256;
const INDEX_COUNT: usize = 24;

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct InstanceData {
    world: [[f32; 4]; 4],
    color: [f32; 4],
}

pub(crate) struct DebugAabbDrawer {
    vertex_buffer: Buffer<SimpleVertex>,
    index_buffer: Buffer<u16>,
    instance_data_buffer: Buffer<InstanceData>,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
    pipeline: RenderPipeline,
    draw_count: usize,
    instance_data: Vec<InstanceData>,
}

impl Drawer<{ BindGroupCount::One }, { ColorAttachmentCount::One }, { DepthAttachmentCount::None }> for DebugAabbDrawer {
    type Context = PostProcessingRenderPassContext;
    type DrawData<'data> = Option<()>;

    fn new(
        _capabilities: &Capabilities,
        device: &Device,
        queue: &Queue,
        shader_compiler: &ShaderCompiler,
        _global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let shader_module = shader_compiler.create_shader_module("postprocessing", "debug_aabb");

        // Vertices are defined in world coordinates (Same as WGPU's NDC).
        let vertex_data = [
            SimpleVertex::new(Point3::new(-1.0, -1.0, -1.0)), // bottom left front
            SimpleVertex::new(Point3::new(-1.0, 1.0, -1.0)),  // top left front
            SimpleVertex::new(Point3::new(1.0, -1.0, -1.0)),  // bottom right front
            SimpleVertex::new(Point3::new(1.0, 1.0, -1.0)),   // top right front
            SimpleVertex::new(Point3::new(-1.0, -1.0, 1.0)),  // bottom left back
            SimpleVertex::new(Point3::new(-1.0, 1.0, 1.0)),   // top left back
            SimpleVertex::new(Point3::new(1.0, -1.0, 1.0)),   // bottom right back
            SimpleVertex::new(Point3::new(1.0, 1.0, 1.0)),    // top right back
        ];

        let index_data: [u16; INDEX_COUNT] = [
            0, 1, 2, 3, 4, 5, 6, 7, // sides
            1, 3, 3, 7, 7, 5, 5, 1, // top
            0, 2, 2, 6, 6, 4, 4, 0, // bottom
        ];

        let vertex_buffer = Buffer::with_data(
            device,
            queue,
            format!("{DRAWER_NAME} box vertex"),
            BufferUsages::VERTEX | BufferUsages::COPY_DST,
            &vertex_data,
        );

        let index_buffer = Buffer::with_data(
            device,
            queue,
            format!("{DRAWER_NAME} box index"),
            BufferUsages::INDEX | BufferUsages::COPY_DST,
            &index_data,
        );

        let instance_data_buffer = Buffer::with_capacity(
            device,
            format!("{DRAWER_NAME} instance data"),
            BufferUsages::COPY_DST | BufferUsages::STORAGE,
            (size_of::<InstanceData>() * INITIAL_INSTRUCTION_SIZE) as _,
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
            bind_group_layouts: &[bind_group_layouts[0], &bind_group_layout],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(DRAWER_NAME),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[SimpleVertex::buffer_layout()],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: render_pass_context.color_attachment_formats()[0],
                    blend: None,
                    write_mask: ColorWrites::default(),
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            cache: None,
            multiview_mask: None,
        });

        Self {
            vertex_buffer,
            index_buffer,
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
        pass.set_bind_group(1, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
        pass.draw_indexed(0..INDEX_COUNT as u32, 0, 0..self.draw_count as u32);
    }
}

impl Prepare for DebugAabbDrawer {
    fn prepare(&mut self, _device: &Device, instructions: &RenderInstruction) {
        self.draw_count = instructions.aabb.len();

        if self.draw_count == 0 {
            return;
        }

        self.instance_data.clear();

        for instruction in instructions.aabb.iter() {
            self.instance_data.push(InstanceData {
                world: instruction.world.into(),
                color: instruction.color.components_linear(),
            });
        }
    }

    fn upload(&mut self, device: &Device, staging_belt: &mut StagingBelt, command_encoder: &mut CommandEncoder) {
        let recreated = self
            .instance_data_buffer
            .write(device, staging_belt, command_encoder, &self.instance_data);

        if recreated {
            self.bind_group = Self::create_bind_group(device, &self.bind_group_layout, &self.instance_data_buffer);
        }
    }
}

impl DebugAabbDrawer {
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
