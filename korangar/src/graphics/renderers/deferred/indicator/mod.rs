use std::sync::Arc;

use bytemuck::checked::cast_slice;
use bytemuck::{Pod, Zeroable};
use cgmath::Vector3;
use wgpu::{
    include_wgsl, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites, CompareFunction, DepthStencilState, Device, Face,
    FragmentState, FrontFace, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, PushConstantRange,
    Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerBindingType, ShaderModule, ShaderModuleDescriptor,
    ShaderStages, TextureFormat, TextureSampleType, TextureViewDimension, VertexState,
};

use super::DeferredSubRenderer;
use crate::graphics::renderers::sampler::{create_new_sampler, SamplerType};
use crate::graphics::*;

const SHADER: ShaderModuleDescriptor = include_wgsl!("indicator.wgsl");

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Matrices {
    view_projection: [[f32; 4]; 4],
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Constants {
    upper_left: [f32; 4],
    upper_right: [f32; 4],
    lower_left: [f32; 4],
    lower_right: [f32; 4],
    color: [f32; 4],
}

pub struct IndicatorRenderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    matrices_buffer: Buffer<Matrices>,
    nearest_sampler: Sampler,
    matrices_bind_group_layout: BindGroupLayout,
    texture_bind_group_layout: BindGroupLayout,
    pipeline: RenderPipeline,
}

impl IndicatorRenderer {
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
            "indicator matrices",
            BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            size_of::<Matrices>() as _,
        );
        let nearest_sampler = create_new_sampler(&device, "indicator nearest", SamplerType::Nearest);
        let matrices_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: matrices_buffer.byte_capacity(),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let texture_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let pipeline = Self::create_pipeline(
            &device,
            &matrices_bind_group_layout,
            &texture_bind_group_layout,
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
            nearest_sampler,
            matrices_bind_group_layout,
            texture_bind_group_layout,
            pipeline,
        }
    }

    fn create_pipeline(
        device: &Device,
        matrices_bind_group_layout: &BindGroupLayout,
        texture_bind_group_layout: &BindGroupLayout,
        shader_module: &ShaderModule,
        output_diffuse_format: TextureFormat,
        output_normal_format: TextureFormat,
        output_water_format: TextureFormat,
        output_depth_format: TextureFormat,
    ) -> RenderPipeline {
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("indicator texture"),
            bind_group_layouts: &[matrices_bind_group_layout, texture_bind_group_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX_FRAGMENT,
                range: 0..size_of::<Constants>() as _,
            }],
        });

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("indicator"),
            layout: Some(&layout),
            vertex: VertexState {
                module: shader_module,
                entry_point: "vs_main",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
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
            primitive: PrimitiveState {
                cull_mode: Some(Face::Back),
                front_face: FrontFace::Ccw,
                ..Default::default()
            },
            multisample: MultisampleState {
                count: 4,
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: output_depth_format,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            cache: None,
        })
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn bind_pipeline(&self, render_pass: &mut RenderPass, camera: &dyn Camera) {
        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let uniform_data = Matrices {
            view_projection: (projection_matrix * view_matrix).into(),
        };
        self.matrices_buffer.write_exact(&self.queue, &[uniform_data]);

        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("indicator renderer"),
            layout: &self.matrices_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: self.matrices_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&self.nearest_sampler),
                },
            ],
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn render_ground_indicator(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        texture: &Texture,
        color: Color,
        upper_left: Vector3<f32>,
        upper_right: Vector3<f32>,
        lower_left: Vector3<f32>,
        lower_right: Vector3<f32>,
    ) {
        if render_target.bound_sub_renderer(DeferredSubRenderer::Indicator) {
            self.bind_pipeline(render_pass, camera);
        }

        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("indicator texture"),
            layout: &self.texture_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(texture.get_texture_view()),
            }],
        });

        let push_constants = Constants {
            upper_left: upper_left.extend(1.0).into(),
            upper_right: upper_right.extend(1.0).into(),
            lower_left: lower_left.extend(1.0).into(),
            lower_right: lower_right.extend(1.0).into(),
            color: color.components_linear(),
        };

        render_pass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 0, cast_slice(&[push_constants]));
        render_pass.set_bind_group(1, &bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}
