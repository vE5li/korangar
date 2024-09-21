use std::sync::Arc;

use bytemuck::{cast_slice, Pod, Zeroable};
use cgmath::Vector3;
use wgpu::{
    include_wgsl, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites, Device, FragmentState, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PushConstantRange, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderModule,
    ShaderModuleDescriptor, ShaderStages, TextureFormat, TextureSampleType, TextureViewDimension, VertexState,
};

use super::{Camera, Color, DeferredRenderer, DeferredSubRenderer, Renderer, LIGHT_ATTACHMENT_BLEND};
use crate::Buffer;

const SHADER: ShaderModuleDescriptor = include_wgsl!("point.wgsl");

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Matrices {
    screen_to_world: [[f32; 4]; 4],
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Constants {
    position: [f32; 4],
    color: [f32; 4],
    screen_position: [f32; 2],
    screen_size: [f32; 2],
    range: f32,
}

pub struct PointLightRenderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    shader_module: ShaderModule,
    matrices_buffer: Buffer<Matrices>,
    bind_group_layout: BindGroupLayout,
    pipeline: RenderPipeline,
}

impl PointLightRenderer {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>, surface_format: TextureFormat) -> Self {
        let shader_module = device.create_shader_module(SHADER);
        let matrices_buffer = Buffer::with_capacity(
            &device,
            "point light matrices",
            BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            size_of::<Matrices>() as _,
        );
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("directional light"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: true,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: true,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Depth,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: true,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: matrices_buffer.byte_capacity(),
                    },
                    count: None,
                },
            ],
        });
        let pipeline = Self::create_pipeline(&device, &shader_module, &bind_group_layout, surface_format);

        Self {
            device,
            queue,
            shader_module,
            matrices_buffer,
            bind_group_layout,
            pipeline,
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn recreate_pipeline(&mut self, surface_format: TextureFormat) {
        self.pipeline = Self::create_pipeline(&self.device, &self.shader_module, &self.bind_group_layout, surface_format);
    }

    fn create_pipeline(
        device: &Device,
        shader_module: &ShaderModule,
        bind_group_layout: &BindGroupLayout,
        surface_format: TextureFormat,
    ) -> RenderPipeline {
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("point light"),
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX_FRAGMENT,
                range: 0..size_of::<Constants>() as _,
            }],
        });

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("point light"),
            layout: Some(&layout),
            vertex: VertexState {
                module: shader_module,
                entry_point: "vs_main",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(FragmentState {
                module: shader_module,
                entry_point: "fs_main",
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: surface_format,
                    blend: Some(LIGHT_ATTACHMENT_BLEND),
                    write_mask: ColorWrites::default(),
                })],
            }),
            multiview: None,
            cache: None,
        })
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn bind_pipeline(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, render_pass: &mut RenderPass, camera: &dyn Camera) {
        let matrices = Matrices {
            screen_to_world: camera.screen_to_world_matrix().into(),
        };
        self.matrices_buffer.write_exact(&self.queue, &[matrices]);

        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("point light"),
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(render_target.diffuse_buffer.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(render_target.normal_buffer.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(render_target.depth_buffer.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: self.matrices_buffer.as_entire_binding(),
                },
            ],
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("render point light"))]
    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        position: Vector3<f32>,
        color: Color,
        range: f32,
    ) {
        if render_target.bound_sub_renderer(DeferredSubRenderer::PointLight) {
            self.bind_pipeline(render_target, render_pass, camera);
        }

        let (top_left_position, bottom_right_position) = camera.billboard_coordinates(position, 10.0 * (range / 0.05).ln());

        if top_left_position.w < 0.1 && bottom_right_position.w < 0.1 && camera.distance_to(position) > range {
            return;
        }

        let (screen_position, screen_size) = camera.screen_position_size(top_left_position, bottom_right_position);

        let push_constants = Constants {
            position: [position.x, position.y, position.z, 1.0],
            color: color.components_linear(),
            screen_position: screen_position.into(),
            screen_size: screen_size.into(),
            range,
        };

        render_pass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 0, cast_slice(&[push_constants]));
        render_pass.draw(0..6, 0..1);
    }
}
