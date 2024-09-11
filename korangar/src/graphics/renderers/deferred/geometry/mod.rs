use std::collections::HashMap;
use std::sync::Arc;

use bytemuck::{cast_slice, Pod, Zeroable};
use cgmath::{Matrix, Matrix4, SquareMatrix, Transform};
use wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites, CompareFunction, DepthStencilState,
    Device, Face, FragmentState, FrontFace, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor, PolygonMode,
    PrimitiveState, PushConstantRange, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, SamplerBindingType, ShaderModule,
    ShaderModuleDescriptor, ShaderStages, TextureFormat, VertexState,
};

use crate::graphics::renderers::deferred::DeferredSubRenderer;
use crate::graphics::renderers::sampler::{create_new_sampler, SamplerType};
use crate::graphics::{Buffer, Camera, DeferredRenderer, ModelVertex, Renderer, TextureGroup};

const SHADER: ShaderModuleDescriptor = include_wgsl!("geometry.wgsl");

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Matrices {
    view_projection: [[f32; 4]; 4],
    time: f32,
    padding: [u8; 12],
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Constants {
    world: [[f32; 4]; 4],
    inv_world: [[f32; 4]; 4],
}

pub struct GeometryRenderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    shader_module: ShaderModule,
    matrices_buffer: Buffer<Matrices>,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
    output_diffuse_format: TextureFormat,
    output_normal_format: TextureFormat,
    output_water_format: TextureFormat,
    output_depth_format: TextureFormat,
    pipeline: RenderPipeline,
}

impl GeometryRenderer {
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
            "geometry matrices",
            BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            size_of::<Matrices>() as _,
        );
        let nearest_sampler = create_new_sampler(&device, "geometry nearest", SamplerType::Nearest);
        let linear_sampler = create_new_sampler(&device, "geometry anisotropic", SamplerType::LinearAnisotropic(4));
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("geometry matrices"),
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
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("geometry uniforms"),
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: matrices_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&nearest_sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&linear_sampler),
                },
            ],
        });

        let pipeline = Self::create_pipeline(
            &device,
            &bind_group_layout,
            &shader_module,
            output_diffuse_format,
            output_normal_format,
            output_water_format,
            output_depth_format,
            #[cfg(feature = "debug")]
            false,
        );

        Self {
            device,
            queue,
            shader_module,
            matrices_buffer,
            bind_group_layout,
            bind_group,
            output_diffuse_format,
            output_normal_format,
            output_water_format,
            output_depth_format,
            pipeline,
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn recreate_pipeline(&mut self, #[cfg(feature = "debug")] wireframe: bool) {
        self.pipeline = Self::create_pipeline(
            &self.device,
            &self.bind_group_layout,
            &self.shader_module,
            self.output_diffuse_format,
            self.output_normal_format,
            self.output_water_format,
            self.output_depth_format,
            #[cfg(feature = "debug")]
            wireframe,
        );
    }

    fn create_pipeline(
        device: &Device,
        matrices_bind_group_layout: &BindGroupLayout,
        shader_module: &ShaderModule,
        output_diffuse_format: TextureFormat,
        output_normal_format: TextureFormat,
        output_water_format: TextureFormat,
        output_depth_format: TextureFormat,
        #[cfg(feature = "debug")] wireframe: bool,
    ) -> RenderPipeline {
        #[cfg(feature = "debug")]
        let (polygon_mode, additional_color) = match wireframe {
            true => (PolygonMode::Line, 1.0f64),
            false => (PolygonMode::Fill, 0.0f64),
        };

        #[cfg(not(feature = "debug"))]
        let (polygon_mode, additional_color) = (PolygonMode::Fill, 0.0f64);

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("geometry"),
            bind_group_layouts: &[matrices_bind_group_layout, TextureGroup::bind_group_layout(device)],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX,
                range: 0..size_of::<Constants>() as _,
            }],
        });

        let mut constants = HashMap::new();
        constants.insert("additional_color".to_owned(), additional_color);

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("geometry"),
            layout: Some(&layout),
            vertex: VertexState {
                module: shader_module,
                entry_point: "vs_main",
                compilation_options: PipelineCompilationOptions {
                    constants: &constants,
                    ..Default::default()
                },
                buffers: &[ModelVertex::buffer_layout()],
            },
            fragment: Some(FragmentState {
                module: shader_module,
                entry_point: "fs_main",
                compilation_options: PipelineCompilationOptions {
                    constants: &constants,
                    ..Default::default()
                },
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
                polygon_mode,
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
    fn bind_pipeline(&self, render_pass: &mut RenderPass, camera: &dyn Camera, time: f32) {
        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let uniform_data = Matrices {
            view_projection: (projection_matrix * view_matrix).into(),
            time,
            padding: Default::default(),
        };
        self.matrices_buffer.write_exact(&self.queue, &[uniform_data]);

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("render geometry"))]
    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        vertex_buffer: &Buffer<ModelVertex>,
        textures: &TextureGroup,
        world_matrix: Matrix4<f32>,
        time: f32,
    ) {
        if render_target.bound_sub_renderer(DeferredSubRenderer::Geometry) {
            self.bind_pipeline(render_pass, camera, time);
        }

        let push_constants = Constants {
            world: world_matrix.into(),
            inv_world: world_matrix.inverse_transform().unwrap_or(Matrix4::identity()).transpose().into(),
        };

        render_pass.set_push_constants(ShaderStages::VERTEX, 0, cast_slice(&[push_constants]));
        render_pass.set_bind_group(1, textures.bind_group(), &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.draw(0..vertex_buffer.count(), 0..1);
    }
}
