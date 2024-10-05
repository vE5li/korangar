use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites, CompareFunction, DepthStencilState,
    Device, FragmentState, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass,
    RenderPipeline, RenderPipelineDescriptor, SamplerBindingType, ShaderModule, ShaderModuleDescriptor, ShaderStages, TextureFormat,
    VertexState,
};

use super::{Buffer, Camera, DeferredSubRenderer, Renderer};
use crate::graphics::renderers::sampler::{create_new_sampler, SamplerType};
use crate::graphics::{DeferredRenderer, ModelVertex, Texture, TextureGroup};
use crate::loaders::TextureLoader;

const SHADER: ShaderModuleDescriptor = include_wgsl!("title.wgsl");

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Matrices {
    view_projection: [[f32; 4]; 4],
}

pub struct TileRenderer {
    queue: Arc<Queue>,
    matrices_buffer: Buffer<Matrices>,
    tile_textures: TextureGroup,
    bind_group: BindGroup,
    pipeline: RenderPipeline,
}

impl TileRenderer {
    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        texture_loader: &mut TextureLoader,
        output_diffuse_format: TextureFormat,
        output_normal_format: TextureFormat,
        output_water_format: TextureFormat,
        output_depth_format: TextureFormat,
    ) -> Self {
        let shader_module = device.create_shader_module(SHADER);
        let matrices_buffer = Buffer::with_capacity(
            &device,
            "tile",
            BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            size_of::<Matrices>() as _,
        );
        let nearest_sampler = create_new_sampler(&device, "tile nearest", SamplerType::Nearest);
        let linear_sampler = create_new_sampler(&device, "tile anisotropic", SamplerType::LinearAnisotropic(4));
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("tile"),
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
            label: Some("tile uniform"),
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
            &shader_module,
            &bind_group_layout,
            output_diffuse_format,
            output_normal_format,
            output_water_format,
            output_depth_format,
        );

        #[cfg(feature = "debug")]
        let tile_textures: Vec<Arc<Texture>> = vec![
            texture_loader.get("0.png").unwrap(),
            texture_loader.get("1.png").unwrap(),
            texture_loader.get("2.png").unwrap(),
            texture_loader.get("3.png").unwrap(),
            texture_loader.get("4.png").unwrap(),
            texture_loader.get("5.png").unwrap(),
            texture_loader.get("6.png").unwrap(),
        ];

        #[cfg(feature = "debug")]
        let tile_textures = TextureGroup::new(&device, "tile textures", tile_textures);

        Self {
            queue,
            matrices_buffer,
            tile_textures,
            bind_group,
            pipeline,
        }
    }

    fn create_pipeline(
        device: &Device,
        shader_module: &ShaderModule,
        bind_group_layout: &BindGroupLayout,
        output_diffuse_format: TextureFormat,
        output_normal_format: TextureFormat,
        output_water_format: TextureFormat,
        output_depth_format: TextureFormat,
    ) -> RenderPipeline {
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("tile"),
            bind_group_layouts: &[bind_group_layout, TextureGroup::bind_group_layout(device)],
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("tile"),
            layout: Some(&layout),
            vertex: VertexState {
                module: shader_module,
                entry_point: "vs_main",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[ModelVertex::buffer_layout()],
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
            primitive: PrimitiveState::default(),
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

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_bind_group(1, self.tile_textures.bind_group(), &[]);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("render tiles"))]
    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        vertex_buffer: &Buffer<ModelVertex>,
    ) {
        if render_target.bound_sub_renderer(DeferredSubRenderer::Tile) {
            self.bind_pipeline(render_pass, camera);
        }

        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.draw(0..vertex_buffer.count(), 0..1);
    }
}
