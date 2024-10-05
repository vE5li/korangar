use std::sync::Arc;

use cgmath::{EuclideanSpace, Point3};
use wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, CompareFunction, DepthStencilState, Device, FragmentState, MultisampleState, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, SamplerBindingType,
    ShaderModule, ShaderModuleDescriptor, ShaderStages, TextureFormat, VertexState,
};

use crate::graphics::renderers::point_shadow::PointShadowSubRenderer;
use crate::graphics::renderers::sampler::{create_new_sampler, SamplerType};
use crate::graphics::renderers::DrawIndirectArgs;
use crate::graphics::*;

const SHADER: ShaderModuleDescriptor = include_wgsl!("geometry.wgsl");

pub struct GeometryRenderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    sampler_bind_group: BindGroup,
    pipeline: RenderPipeline,
    instance_data: Vec<CubeFaceInstanceData>,
    draw_commands: Vec<DrawIndirectArgs>,
    instance_indices: Vec<u32>,
}

impl GeometryRenderer {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>, output_depth_format: TextureFormat) -> Self {
        let shader_module = device.create_shader_module(SHADER);
        let nearest_sampler = create_new_sampler(&device, "geometry nearest", SamplerType::Nearest);
        let sampler_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("geometry sampler"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            }],
        });
        let sampler_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("geometry"),
            layout: &sampler_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Sampler(&nearest_sampler),
            }],
        });

        let pipeline = Self::create_pipeline(&device, &shader_module, &sampler_bind_group_layout, output_depth_format);

        Self {
            device,
            queue,
            sampler_bind_group,
            pipeline,
            instance_data: Vec::new(),
            draw_commands: Vec::new(),
            instance_indices: Vec::new(),
        }
    }

    fn create_pipeline(
        device: &Device,
        shader_module: &ShaderModule,
        sampler_bind_group_layout: &BindGroupLayout,
        output_depth_format: TextureFormat,
    ) -> RenderPipeline {
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("geometry"),
            bind_group_layouts: &[
                sampler_bind_group_layout,
                CubeFaceBuffer::bind_group_layout(device),
                TextureGroup::bind_group_layout(device),
            ],
            push_constant_ranges: &[],
        });

        let instance_index_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: size_of::<u32>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Uint32,
                offset: 0,
                shader_location: 5,
            }],
        };

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("geometry"),
            layout: Some(&layout),
            vertex: VertexState {
                module: shader_module,
                entry_point: "vs_main",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[ModelVertex::buffer_layout(), instance_index_buffer_layout],
            },
            fragment: Some(FragmentState {
                module: shader_module,
                entry_point: "fs_main",
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[],
            }),
            multiview: None,
            primitive: PrimitiveState::default(),
            multisample: MultisampleState::default(),
            depth_stencil: Some(DepthStencilState {
                format: output_depth_format,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            cache: None,
        })
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn bind_pipeline(&self, render_pass: &mut RenderPass) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.sampler_bind_group, &[]);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("geometry renderer"))]
    pub fn render(
        &mut self,
        render_target: &mut <PointShadowRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        light_position: Point3<f32>,
        instructions: &[GeometryInstruction],
        vertex_buffer: &Buffer<ModelVertex>,
        textures: &TextureGroup,
        time: f32,
    ) {
        if instructions.is_empty() {
            return;
        }

        if render_target.bind_sub_renderer(PointShadowSubRenderer::Geometry) {
            self.bind_pipeline(render_pass);
        }

        self.instance_data.clear();
        self.draw_commands.clear();
        self.instance_indices.clear();

        for (instance_index, instruction) in instructions.iter().enumerate() {
            self.instance_data.push(CubeFaceInstanceData {
                world: instruction.world_matrix.into(),
                light_position: light_position.to_vec().extend(time).into(),
            });

            self.draw_commands.push(DrawIndirectArgs {
                vertex_count: instruction.vertex_count,
                instance_count: 1,
                first_vertex: instruction.vertex_offset,
                first_instance: instance_index as u32,
            });

            self.instance_indices.push(instance_index as u32);
        }

        render_target
            .face_instance_buffer()
            .write(&self.device, &self.queue, &self.instance_data);

        render_target
            .face_draw_command_buffer()
            .write(&self.device, &self.queue, &self.draw_commands);

        render_target
            .face_instance_index_vertex_buffer()
            .write(&self.device, &self.queue, &self.instance_indices);

        render_pass.set_bind_group(1, &render_target.face_bind_group(&self.device), &[]);
        render_pass.set_bind_group(2, textures.bind_group(), &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, render_target.face_instance_index_vertex_buffer().slice(..));
        render_pass.multi_draw_indirect(
            render_target.face_draw_command_buffer().get_buffer(),
            0,
            self.draw_commands.len() as u32,
        );
    }
}
