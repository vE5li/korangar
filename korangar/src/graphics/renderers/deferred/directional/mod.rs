use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use cgmath::{Matrix4, Vector3};
use wgpu::{
    include_wgsl, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites, Device, FragmentState, PipelineCompilationOptions,
    PipelineLayoutDescriptor, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerBindingType, ShaderModule,
    ShaderModuleDescriptor, ShaderStages, TextureFormat, TextureSampleType, TextureViewDimension, VertexState,
};

use super::{Camera, Color, DeferredRenderer, DeferredSubRenderer, Renderer, Texture, LIGHT_ATTACHMENT_BLEND};
use crate::graphics::renderers::sampler::{create_new_sampler, SamplerType};
use crate::Buffer;

const SHADER: ShaderModuleDescriptor = include_wgsl!("directional.wgsl");

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Matrices {
    screen_to_world: [[f32; 4]; 4],
    light: [[f32; 4]; 4],
    color: [f32; 4],
    direction: [f32; 4],
}

pub struct DirectionalLightRenderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    shader_module: ShaderModule,
    matrices_buffer: Buffer<Matrices>,
    linear_sampler: Sampler,
    bind_group_layout: BindGroupLayout,
    pipeline: RenderPipeline,
}

impl DirectionalLightRenderer {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>, surface_format: TextureFormat) -> Self {
        let shader_module = device.create_shader_module(SHADER);
        let matrices_buffer = Buffer::with_capacity(
            &device,
            "directional light matrices",
            BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            size_of::<Matrices>() as u64,
        );
        let linear_sampler = create_new_sampler(&device, "directional light linear", SamplerType::Linear);
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
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Depth,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 5,
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
            linear_sampler,
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
            label: Some("directional light"),
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("directional light"),
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
    fn bind_pipeline(&self, render_pass: &mut RenderPass) {
        render_pass.set_pipeline(&self.pipeline);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("render directional light"))]
    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        shadow_map: &Texture,
        light_matrix: Matrix4<f32>,
        direction: Vector3<f32>,
        color: Color,
        intensity: f32,
    ) {
        if render_target.bound_sub_renderer(DeferredSubRenderer::DirectionalLight) {
            self.bind_pipeline(render_pass);
        }

        let color = Color::rgb(color.red * intensity, color.green * intensity, color.blue * intensity);

        let matrices = Matrices {
            screen_to_world: camera.screen_to_world_matrix().into(),
            light: light_matrix.into(),
            color: color.components_linear(),
            direction: [direction.x, direction.y, direction.z, 1.0],
        };
        self.matrices_buffer.write_exact(&self.queue, &[matrices]);

        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("directional light"),
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
                    resource: BindingResource::TextureView(shadow_map.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Sampler(&self.linear_sampler),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: self.matrices_buffer.as_entire_binding(),
                },
            ],
        });

        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}
