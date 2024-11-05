use wgpu::{
    include_wgsl, ComputePass, ComputePipeline, ComputePipelineDescriptor, Device, PipelineCompilationOptions, PipelineLayoutDescriptor,
    Queue, ShaderModuleDescriptor,
};

use crate::graphics::passes::{BindGroupCount, Cmaa2ComputePassContext, ComputePassContext, Dispatch};
use crate::graphics::{Capabilities, GlobalContext};

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/calculate_dispatch_args.wgsl");
const DISPATCHER_NAME: &str = "cmaa2 calculate dispatch args";

#[derive(Copy, Clone)]
pub(crate) enum Cmaa2CalculateDispatchArgsDispatchData {
    ProcessCandidates,
    DeferredColorApply,
}

pub(crate) struct Cmaa2CalculateDispatchArgsDispatcher {
    pipeline: ComputePipeline,
}

impl Dispatch<{ BindGroupCount::One }> for Cmaa2CalculateDispatchArgsDispatcher {
    type Context = Cmaa2ComputePassContext;
    type DispatchData<'data> = Cmaa2CalculateDispatchArgsDispatchData;

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
            bind_group_layouts: &[pass_bind_group_layouts[0]],
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
        match draw_data {
            Cmaa2CalculateDispatchArgsDispatchData::ProcessCandidates => {
                pass.dispatch_workgroups(2, 1, 1);
            }
            Cmaa2CalculateDispatchArgsDispatchData::DeferredColorApply => {
                pass.dispatch_workgroups(1, 2, 1);
            }
        }
    }
}
