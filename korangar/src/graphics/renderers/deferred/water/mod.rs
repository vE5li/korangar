use std::sync::Arc;

use bytemuck::checked::cast_slice;
use bytemuck::{Pod, Zeroable};
use wgpu::{
    include_wgsl, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    BufferBindingType, BufferUsages, ColorTargetState, ColorWrites, CompareFunction, DepthStencilState, Device, FragmentState,
    MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor, PushConstantRange, Queue, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, ShaderModule, ShaderModuleDescriptor, ShaderStages, TextureFormat, VertexState,
};

use super::DeferredSubRenderer;
use crate::graphics::*;

const SHADER: ShaderModuleDescriptor = include_wgsl!("water.wgsl");

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Matrices {
    view: [[f32; 4]; 4],
    projection: [[f32; 4]; 4],
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Constants {
    wave_offset: f32,
}

pub struct WaterRenderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    matrices_buffer: Buffer<Matrices>,
    matrices_bind_group_layout: BindGroupLayout,
    pipeline: RenderPipeline,
}

impl WaterRenderer {
    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        output_diffuse_format: TextureFormat,
        output_normal_format: TextureFormat,
        output_water_format: TextureFormat,
        output_depth_format: TextureFormat,
    ) -> Self {
        let shader_module = device.create_shader_module(SHADER);
        let matrices_buffer = Buffer::with_capacity(
            &device,
            "water matrices",
            BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            size_of::<Matrices>() as _,
        );
        let matrices_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("water renderer"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: matrices_buffer.byte_capacity(),
                },
                count: None,
            }],
        });

        let pipeline = Self::create_pipeline(
            &device,
            &matrices_bind_group_layout,
            &shader_module,
            output_diffuse_format,
            output_normal_format,
            output_water_format,
            output_depth_format,
        );

        Self {
            device,
            queue,
            matrices_buffer,
            matrices_bind_group_layout,
            pipeline,
        }
    }

    fn create_pipeline(
        device: &Device,
        matrices_bind_group_layout: &BindGroupLayout,
        shader_module: &ShaderModule,
        output_diffuse_format: TextureFormat,
        output_normal_format: TextureFormat,
        output_water_format: TextureFormat,
        output_depth_format: TextureFormat,
    ) -> RenderPipeline {
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("water renderer"),
            bind_group_layouts: &[matrices_bind_group_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX,
                range: 0..size_of::<Constants>() as _,
            }],
        });

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("water"),
            layout: Some(&layout),
            vertex: VertexState {
                module: shader_module,
                entry_point: "vs_main",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[WaterVertex::buffer_layout()],
            },
            fragment: Some(FragmentState {
                module: shader_module,
                entry_point: "fs_main",
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[
                    Some(ColorTargetState {
                        format: output_diffuse_format,
                        blend: None,
                        write_mask: ColorWrites::default(),
                    }),
                    Some(ColorTargetState {
                        format: output_normal_format,
                        blend: None,
                        write_mask: ColorWrites::default(),
                    }),
                    Some(ColorTargetState {
                        format: output_water_format,
                        blend: None,
                        write_mask: ColorWrites::default(),
                    }),
                ],
            }),
            multiview: None,
            multisample: MultisampleState {
                count: 4,
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: output_depth_format,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Greater,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            primitive: Default::default(),
            cache: None,
        })
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn bind_pipeline(&self, render_pass: &mut RenderPass, camera: &dyn Camera) {
        let (view, projection) = camera.view_projection_matrices();
        let uniform_data = Matrices {
            view: view.into(),
            projection: projection.into(),
        };
        self.matrices_buffer.write_exact(&self.queue, &[uniform_data]);

        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("water renderer"),
            layout: &self.matrices_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: self.matrices_buffer.as_entire_binding(),
            }],
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("render water"))]
    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        vertex_buffer: &Buffer<WaterVertex>,
        day_timer: f32,
    ) {
        if render_target.bound_sub_renderer(DeferredSubRenderer::Water) {
            self.bind_pipeline(render_pass, camera);
        }

        let push_constants = Constants { wave_offset: day_timer };

        render_pass.set_push_constants(ShaderStages::VERTEX, 0, cast_slice(&[push_constants]));
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.draw(0..vertex_buffer.count(), 0..1);
    }
}
