use wgpu::{
    include_wgsl, BindGroup, ComputePass, ComputePipeline, ComputePipelineDescriptor, Device, PipelineCompilationOptions,
    PipelineLayoutDescriptor, Queue, ShaderModuleDescriptor,
};

use crate::graphics::passes::{BindGroupCount, Cmaa2ComputePassContext, ComputePassContext, Dispatch, DispatchIndirectArgs};
use crate::graphics::{Buffer, Capabilities, GlobalContext};

const SHADER_SRC: ShaderModuleDescriptor = include_wgsl!("shader/deferred_color_apply.wgsl");
const DISPATCHER_NAME: &str = "cmaa2 deferred color apply";

pub(crate) struct Cmaa2DeferredColorApplyDispatchData<'a> {
    pub(crate) dispatch_indirect_args_buffer: &'a Buffer<DispatchIndirectArgs>,
    pub(crate) output_bind_group: &'a BindGroup,
}

pub(crate) struct Cmaa2DeferredColorApplyDispatcher {
    pipeline: ComputePipeline,
}

impl Dispatch<{ BindGroupCount::One }> for Cmaa2DeferredColorApplyDispatcher {
    type Context = Cmaa2ComputePassContext;
    type DispatchData<'data> = Cmaa2DeferredColorApplyDispatchData<'data>;

    fn new(
        _capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        _global_context: &GlobalContext,
        _compute_pass_context: &Self::Context,
    ) -> Self {
        let shader_module = device.create_shader_module(SHADER_SRC);

        let pass_bind_group_layouts = Self::Context::bind_group_layout(device);

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DISPATCHER_NAME),
            bind_group_layouts: &[pass_bind_group_layouts[0], GlobalContext::cmaa2_output_bind_group_layout(device)],
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
        pass.set_bind_group(1, draw_data.output_bind_group, &[]);
        pass.dispatch_workgroups_indirect(draw_data.dispatch_indirect_args_buffer.get_buffer(), 0);
    }
}
