use std::sync::Arc;

use bytemuck::{cast_slice, Pod, Zeroable};
use cgmath::Point3;
use korangar_util::collision::AABB;
use ragnarok_formats::transform::Transform;
use wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingType, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites, Device, FragmentState, IndexFormat,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, PushConstantRange, Queue, RenderPass,
    RenderPipeline, RenderPipelineDescriptor, ShaderModule, ShaderModuleDescriptor, ShaderStages, TextureFormat, VertexState,
};

use super::{Camera, Color, DeferredRenderer, DeferredSubRenderer, Renderer, WaterVertex};
use crate::world::Model;
use crate::Buffer;

const SHADER: ShaderModuleDescriptor = include_wgsl!("box.wgsl");

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Matrices {
    view_projection: [[f32; 4]; 4],
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Constants {
    world: [[f32; 4]; 4],
    color: [f32; 4],
}

pub struct BoxRenderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    shader_module: ShaderModule,
    vertex_buffer: Buffer<WaterVertex>,
    index_buffer: Buffer<u16>,
    matrices_buffer: Buffer<Matrices>,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
    pipeline: RenderPipeline,
}

impl BoxRenderer {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>, surface_format: TextureFormat) -> Self {
        let shader_module = device.create_shader_module(SHADER);

        // Vertices are defined in world coordinates (Same as WGPU's NDC).
        let vertex_data = [
            WaterVertex::new(Point3::new(-1.0, -1.0, -1.0)), // bottom left front
            WaterVertex::new(Point3::new(-1.0, 1.0, -1.0)),  // top left front
            WaterVertex::new(Point3::new(1.0, -1.0, -1.0)),  // bottom right front
            WaterVertex::new(Point3::new(1.0, 1.0, -1.0)),   // top right front
            WaterVertex::new(Point3::new(-1.0, -1.0, 1.0)),  // bottom left back
            WaterVertex::new(Point3::new(-1.0, 1.0, 1.0)),   // top left back
            WaterVertex::new(Point3::new(1.0, -1.0, 1.0)),   // bottom right back
            WaterVertex::new(Point3::new(1.0, 1.0, 1.0)),    // top right back
        ];

        let index_data = [
            0, 1, 2, 3, 4, 5, 6, 7, // sides
            1, 3, 3, 7, 7, 5, 5, 1, // top
            0, 2, 2, 6, 6, 4, 4, 0, // bottom
        ];

        let vertex_buffer = Buffer::with_data(
            &device,
            &queue,
            "box vertex",
            BufferUsages::VERTEX | BufferUsages::COPY_DST,
            &vertex_data,
        );

        let index_buffer = Buffer::with_data(
            &device,
            &queue,
            "box index",
            BufferUsages::INDEX | BufferUsages::COPY_DST,
            &index_data,
        );

        let matrices_buffer = Buffer::with_capacity(
            &device,
            "box matrices",
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            size_of::<Matrices>() as u64,
        );

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("box matrices"),
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

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("box matrices"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: matrices_buffer.as_entire_binding(),
            }],
        });

        let pipeline = Self::create_pipeline(&device, &shader_module, &bind_group_layout, surface_format);

        Self {
            device,
            queue,
            shader_module,
            vertex_buffer,
            index_buffer,
            matrices_buffer,
            bind_group_layout,
            bind_group,
            pipeline,
        }
    }

    #[korangar_debug::profile]
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
            label: Some("box"),
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX_FRAGMENT,
                range: 0..size_of::<Constants>() as _,
            }],
        });

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("box"),
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
                targets: &[Some(ColorTargetState {
                    format: surface_format,
                    blend: None,
                    write_mask: ColorWrites::default(),
                })],
            }),
            multiview: None,
            primitive: PrimitiveState {
                topology: PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            cache: None,
        })
    }

    #[korangar_debug::profile]
    fn bind_pipeline(&self, render_pass: &mut RenderPass, camera: &dyn Camera) {
        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let uniform_data = Matrices {
            view_projection: (projection_matrix * view_matrix).into(),
        };
        self.matrices_buffer.write_exact(&self.queue, &[uniform_data]);

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
    }

    #[korangar_debug::profile("render bounding box")]
    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        transform: &Transform,
        bounding_box: &AABB,
        color: Color,
    ) {
        if render_target.bound_sub_renderer(DeferredSubRenderer::BoundingBox) {
            self.bind_pipeline(render_pass, camera);
        }

        let world_matrix = Model::bounding_box_matrix(bounding_box, transform);

        let push_constants = Constants {
            world: world_matrix.into(),
            color: color.components_linear(),
        };

        render_pass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 0, cast_slice(&[push_constants]));
        render_pass.draw_indexed(0..self.index_buffer.count(), 0, 0..1);
    }
}
