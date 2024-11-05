use wgpu::{
    include_wgsl, ComputePass, ComputePipeline, ComputePipelineDescriptor, Device, PipelineCompilationOptions, PipelineLayoutDescriptor,
    Queue, ShaderModuleDescriptor, TextureSampleType,
};

use crate::graphics::passes::{BindGroupCount, Cmaa2ComputePassContext, ComputePassContext, Dispatch, DispatchIndirectArgs};
use crate::graphics::{AttachmentTexture, Buffer, Capabilities, GlobalContext};

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/process_candidates.wgsl");
const DISPATCHER_NAME: &str = "cmaa2 process candidates";

pub(crate) struct Cmaa2ProcessCandidatesDispatcherDispatchData<'a> {
    pub(crate) color_input_texture: &'a AttachmentTexture,
    pub(crate) dispatch_indirect_args_buffer: &'a Buffer<DispatchIndirectArgs>,
}

pub(crate) struct Cmaa2ProcessCandidatesDispatcher {
    pipeline: ComputePipeline,
}

impl Dispatch<{ BindGroupCount::One }> for Cmaa2ProcessCandidatesDispatcher {
    type Context = Cmaa2ComputePassContext;
    type DispatchData<'data> = Cmaa2ProcessCandidatesDispatcherDispatchData<'data>;

    fn new(
        _capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        _global_context: &GlobalContext,
        _compute_pass_context: &Self::Context,
    ) -> Self {
        let shader_module = device.create_shader_module(SHADER);

        let pass_bind_group_layouts = Self::Context::bind_group_layout(device);

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DISPATCHER_NAME),
            bind_group_layouts: &[
                pass_bind_group_layouts[0],
                &AttachmentTexture::bind_group_layout(device, TextureSampleType::Float { filterable: true }, false),
            ],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some(DISPATCHER_NAME),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: Some("cs_main"),
            compilation_options: PipelineCompilationOptions {
                zero_initialize_workgroup_memory: false,
                ..Default::default()
            },
            cache: None,
        });

        Self { pipeline }
    }

    fn dispatch(&mut self, pass: &mut ComputePass<'_>, draw_data: Self::DispatchData<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(1, draw_data.color_input_texture.get_bind_group(), &[]);
        pass.dispatch_workgroups_indirect(draw_data.dispatch_indirect_args_buffer.get_buffer(), 0);
    }
}
