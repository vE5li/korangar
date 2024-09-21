use std::sync::Arc;

use bytemuck::{cast_slice, Pod, Zeroable};
use cgmath::{Vector2, Vector3};
use ragnarok_packets::EntityId;
use wgpu::{
    include_wgsl, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites, CompareFunction, DepthStencilState, Device, FragmentState,
    MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, PushConstantRange, Queue, RenderPass,
    RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerBindingType, ShaderModule, ShaderModuleDescriptor, ShaderStages,
    TextureFormat, TextureSampleType, TextureViewDimension, VertexState,
};

use super::{Buffer, Camera, PickerRenderer, PickerSubRenderer, Renderer, Texture};
use crate::graphics::renderers::sampler::{create_new_sampler, SamplerType};
use crate::graphics::renderers::PickerTarget;

const SHADER: ShaderModuleDescriptor = include_wgsl!("entity.wgsl");

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Matrices {
    view: [[f32; 4]; 4],
    projection: [[f32; 4]; 4],
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Constants {
    world: [[f32; 4]; 4],
    texture_position: [f32; 2],
    texture_size: [f32; 2],
    identifier: u32,
    mirror: u32,
}

pub struct EntityRenderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    matrices_buffer: Buffer<Matrices>,
    nearest_sampler: Sampler,
    bind_group_layout: BindGroupLayout,
    pipeline: RenderPipeline,
}

impl EntityRenderer {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>, output_color_format: TextureFormat, output_depth_format: TextureFormat) -> Self {
        let shader_module = device.create_shader_module(SHADER);
        let matrices_buffer = Buffer::with_capacity(
            &device,
            "entity",
            BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            size_of::<Matrices>() as _,
        );
        let nearest_sampler = create_new_sampler(&device, "entity nearest", SamplerType::Nearest);
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("entity"),
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
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
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

        let pipeline = Self::create_pipeline(
            &device,
            &shader_module,
            &bind_group_layout,
            output_color_format,
            output_depth_format,
        );

        Self {
            device,
            queue,
            matrices_buffer,
            nearest_sampler,
            bind_group_layout,
            pipeline,
        }
    }

    fn create_pipeline(
        device: &Device,
        shader_module: &ShaderModule,
        bind_group_layout: &BindGroupLayout,
        output_color_format: TextureFormat,
        output_depth_format: TextureFormat,
    ) -> RenderPipeline {
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("entity"),
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX_FRAGMENT,
                range: 0..size_of::<Constants>() as _,
            }],
        });

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("entity"),
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
                targets: &[Some(ColorTargetState {
                    format: output_color_format,
                    blend: None,
                    write_mask: ColorWrites::default(),
                })],
            }),
            multiview: None,
            primitive: PrimitiveState::default(),
            multisample: MultisampleState::default(),
            depth_stencil: Some(DepthStencilState {
                format: output_depth_format,
                depth_write_enabled: false,
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
            view: view_matrix.into(),
            projection: projection_matrix.into(),
        };
        self.matrices_buffer.write_exact(&self.queue, &[uniform_data]);

        render_pass.set_pipeline(&self.pipeline);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("render entity"))]
    pub fn render(
        &self,
        render_target: &mut <PickerRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        texture: &Texture,
        position: Vector3<f32>,
        origin: Vector3<f32>,
        scale: Vector2<f32>,
        cell_count: Vector2<usize>,
        cell_position: Vector2<usize>,
        entity_id: EntityId,
        mirror: bool,
    ) {
        if render_target.bound_sub_renderer(PickerSubRenderer::Entity) {
            self.bind_pipeline(render_pass, camera);
        }

        let texture_dimensions = texture.get_extend();
        let size = Vector2::new(
            texture_dimensions.width as f32 * scale.x / 10.0,
            texture_dimensions.height as f32 * scale.y / 10.0,
        );

        let world_matrix = camera.billboard_matrix(position, origin, size);
        let texture_size = Vector2::new(1.0 / cell_count.x as f32, 1.0 / cell_count.y as f32);
        let texture_position = Vector2::new(texture_size.x * cell_position.x as f32, texture_size.y * cell_position.y as f32);
        let picker_target = PickerTarget::Entity(entity_id);

        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("entity"),
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: self.matrices_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&self.nearest_sampler),
                },
            ],
        });

        let push_constants = Constants {
            world: world_matrix.into(),
            texture_position: texture_position.into(),
            texture_size: texture_size.into(),
            identifier: picker_target.into(),
            mirror: mirror as u32,
        };

        render_pass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 0, cast_slice(&[push_constants]));
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}
