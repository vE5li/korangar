use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use wgpu::{
    include_wgsl, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    BufferBindingType, BufferUsages, ColorTargetState, ColorWrites, CompareFunction, DepthStencilState, Device, FragmentState,
    MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, ShaderModule, ShaderModuleDescriptor, ShaderStages, TextureFormat, VertexState,
};

use super::{Buffer, Camera, PickerRenderer, PickerSubRenderer, Renderer, TileVertex};

const SHADER: ShaderModuleDescriptor = include_wgsl!("title.wgsl");

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Matrices {
    view_projection: [[f32; 4]; 4],
}

pub struct TileRenderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    matrices_buffer: Buffer<Matrices>,
    bind_group_layout: BindGroupLayout,
    pipeline: RenderPipeline,
}

impl TileRenderer {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>, output_color_format: TextureFormat, output_depth_format: TextureFormat) -> Self {
        let shader_module = device.create_shader_module(SHADER);
        let matrices_buffer = Buffer::with_capacity(
            &device,
            "tile",
            BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            size_of::<Matrices>() as _,
        );
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("tile"),
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
            &shader_module,
            &bind_group_layout,
            output_color_format,
            output_depth_format,
        );

        Self {
            device,
            queue,
            matrices_buffer,
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
            label: Some("tile"),
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("tile"),
            layout: Some(&layout),
            vertex: VertexState {
                module: shader_module,
                entry_point: "vs_main",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[TileVertex::buffer_layout()],
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
            label: Some("geometry"),
            layout: &self.bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: self.matrices_buffer.as_entire_binding(),
            }],
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("render tiles"))]
    pub fn render(
        &self,
        render_target: &mut <PickerRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        vertex_buffer: &Buffer<TileVertex>,
    ) {
        if render_target.bound_sub_renderer(PickerSubRenderer::Tile) {
            self.bind_pipeline(render_pass, camera);
        }

        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.draw(0..vertex_buffer.count(), 0..1);
    }
}
