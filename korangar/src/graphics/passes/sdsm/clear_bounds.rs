use wgpu::{
    ComputePass, ComputePipeline, ComputePipelineDescriptor, Device, PipelineCompilationOptions, PipelineLayoutDescriptor, Queue,
    ShaderModuleDescriptor, include_wgsl,
};

use crate::graphics::passes::{BindGroupCount, ComputePassContext, Dispatch, SdsmPassContext};
use crate::graphics::{Capabilities, GlobalContext};

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/clear_bounds.wgsl");

const DISPATCHER_NAME: &str = "clear bounds";

pub(crate) struct ClearBoundsDispatcher {
    pipeline: ComputePipeline,
}

impl Dispatch<{ BindGroupCount::Two }> for ClearBoundsDispatcher {
    type Context = SdsmPassContext;
    type DispatchData<'data> = ();

    fn new(
        _capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        global_context: &GlobalContext,
        _compute_pass_context: &Self::Context,
    ) -> Self {
        let shader_module = device.create_shader_module(SHADER);

        let pass_bind_group_layouts = Self::Context::bind_group_layout(device, global_context.msaa);

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DISPATCHER_NAME),
            bind_group_layouts: &[pass_bind_group_layouts[0], pass_bind_group_layouts[1]],
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

    fn dispatch(&mut self, pass: &mut ComputePass<'_>, _draw_data: Self::DispatchData<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.dispatch_workgroups(1, 1, 1);
    }
}
