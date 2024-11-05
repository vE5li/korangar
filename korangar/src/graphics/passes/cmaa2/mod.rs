mod calculate_dispatch_args;
mod deferred_color_apply;
mod edge_colors;
mod process_candidates;

pub(crate) use calculate_dispatch_args::{Cmaa2CalculateDispatchArgsDispatchData, Cmaa2CalculateDispatchArgsDispatcher};
pub(crate) use deferred_color_apply::{Cmaa2DeferredColorApplyDispatchData, Cmaa2DeferredColorApplyDispatcher};
pub(crate) use edge_colors::Cmaa2EdgeColorsDispatcher;
pub(crate) use process_candidates::{Cmaa2ProcessCandidatesDispatcher, Cmaa2ProcessCandidatesDispatcherDispatchData};
use wgpu::{BindGroupLayout, CommandEncoder, ComputePass, ComputePassDescriptor, Device, Queue};

use super::{BindGroupCount, ComputePassContext};
use crate::graphics::{AntiAliasingResource, GlobalContext};

const PASS_NAME: &str = "cmaa2 compute pass";

pub(crate) struct Cmaa2ComputePassContext {}

impl ComputePassContext<{ BindGroupCount::One }> for Cmaa2ComputePassContext {
    type PassData<'data> = Option<()>;

    fn new(_device: &Device, _queue: &Queue, _global_context: &GlobalContext) -> Self {
        Self {}
    }

    fn create_pass<'encoder>(
        &mut self,
        encoder: &'encoder mut CommandEncoder,
        global_context: &GlobalContext,
        _pass_data: Self::PassData<'_>,
    ) -> ComputePass<'encoder> {
        let AntiAliasingResource::Cmaa2(cmaa2_resource) = &global_context.anti_aliasing_resources else {
            panic!("cmaa2 resources not set");
        };

        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some(PASS_NAME),
            timestamp_writes: None,
        });

        pass.set_bind_group(0, &cmaa2_resource.bind_group, &[]);

        pass
    }

    fn bind_group_layout(device: &Device) -> [&'static BindGroupLayout; 1] {
        [GlobalContext::cmaa2_bind_group_layout(device)]
    }
}
