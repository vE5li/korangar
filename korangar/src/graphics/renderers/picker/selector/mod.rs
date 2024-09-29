use std::num::NonZeroU64;
use std::sync::Arc;

use bytemuck::{cast_slice, Pod, Zeroable};
use cgmath::Vector2;
use wgpu::{
    include_wgsl, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, BufferBindingType, ComputePass, ComputePipeline, ComputePipelineDescriptor, Device, PipelineLayoutDescriptor,
    PushConstantRange, ShaderModule, ShaderModuleDescriptor, ShaderStages, TextureSampleType, TextureViewDimension,
};
use TextureSampleType::Uint;

use super::{PickerRenderer, PickerSubRenderer, Renderer};

const SHADER: ShaderModuleDescriptor = include_wgsl!("selector.wgsl");

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Constants {
    pointer_position: [u32; 2],
}

pub struct Selector {
    device: Arc<Device>,
    bind_group_layout: BindGroupLayout,
    pipeline: ComputePipeline,
}

impl Selector {
    pub fn new(device: Arc<Device>) -> Self {
        let shader_module = device.create_shader_module(SHADER);
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("selector"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Texture {
                        sample_type: Uint,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(<PickerRenderer as Renderer>::Target::picker_value_size() as _),
                    },
                    count: None,
                },
            ],
        });

        let pipeline = Self::create_pipeline(&device, &shader_module, &bind_group_layout);

        Self {
            device,
            bind_group_layout,
            pipeline,
        }
    }

    fn create_pipeline(device: &Device, shader_module: &ShaderModule, bind_group_layout: &BindGroupLayout) -> ComputePipeline {
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("selector"),
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::COMPUTE,
                range: 0..size_of::<Constants>() as _,
            }],
        });

        device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("selector"),
            layout: Some(&layout),
            module: shader_module,
            entry_point: "cs_main",
            compilation_options: Default::default(),
            cache: None,
        })
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn bind_pipeline(&self, render_target: &mut <PickerRenderer as Renderer>::Target, render_pass: &mut ComputePass) {
        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("selector"),
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(render_target.texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: render_target.buffer.as_entire_binding(),
                },
            ],
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("dispatch selector"))]
    pub fn dispatch(
        &self,
        render_target: &mut <PickerRenderer as Renderer>::Target,
        compute_pass: &mut ComputePass,
        pointer_position: Vector2<u32>,
    ) {
        if render_target.bound_sub_renderer(PickerSubRenderer::Selector) {
            self.bind_pipeline(render_target, compute_pass);
        }

        let push_constants = Constants {
            pointer_position: pointer_position.into(),
        };

        compute_pass.set_push_constants(0, cast_slice(&[push_constants]));
        compute_pass.dispatch_workgroups(1, 1, 1);
    }
}
