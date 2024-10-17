use wgpu::{
    include_wgsl, ComputePass, ComputePipeline, ComputePipelineDescriptor, Device, PipelineLayoutDescriptor, Queue, ShaderModuleDescriptor,
};

use crate::graphics::passes::{BindGroupCount, ComputePassContext, Dispatch, SelectorComputePassContext};
use crate::graphics::GlobalContext;

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/copy.wgsl");
const DISPATCHER_NAME: &str = "selector copy";

pub(crate) struct SelectorCopyDispatcher {
    pipeline: ComputePipeline,
}

impl Dispatch<{ BindGroupCount::Two }> for SelectorCopyDispatcher {
    type Context = SelectorComputePassContext;
    type DispatchData<'data> = Option<()>;

    fn new(device: &Device, _queue: &Queue, _global_context: &GlobalContext, _render_pass_context: &Self::Context) -> Self {
        let shader_module = device.create_shader_module(SHADER);

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DISPATCHER_NAME),
            bind_group_layouts: &Self::Context::bind_group_layout(device),
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some(DISPATCHER_NAME),
            layout: Some(&layout),
            module: &shader_module,
            entry_point: "cs_main",
            compilation_options: Default::default(),
            cache: None,
        });

        Self { pipeline }
    }

    fn dispatch(&mut self, pass: &mut ComputePass<'_>, _dispatch_data: Self::DispatchData<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.dispatch_workgroups(1, 1, 1);
    }
}
