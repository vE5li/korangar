mod clear_bounds;
mod clear_partitions;
mod compute_custom_partitions;
mod compute_partitions;
mod reduce_bounds;
mod reduce_partitions;

pub(crate) use clear_bounds::ClearBoundsDispatcher;
pub(crate) use clear_partitions::ClearPartitionsDispatcher;
pub(crate) use compute_custom_partitions::ComputeCustomPartitionsDispatcher;
pub(crate) use compute_partitions::ComputePartitionsDispatcher;
pub(crate) use reduce_bounds::ReduceBoundsDispatcher;
pub(crate) use reduce_partitions::ReducePartitionsDispatcher;
use wgpu::{BindGroupLayout, CommandEncoder, ComputePass, ComputePassDescriptor, Device, Queue};

use super::{BindGroupCount, ComputePassContext};
use crate::graphics::{GlobalContext, Msaa};

const PASS_NAME: &str = "sdsm compute pass";

pub(crate) struct SdsmPassContext {}

impl ComputePassContext<{ BindGroupCount::Two }> for SdsmPassContext {
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
        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some(PASS_NAME),
            timestamp_writes: None,
        });

        pass.set_bind_group(0, &global_context.global_bind_group, &[]);
        pass.set_bind_group(1, &global_context.sdsm_bind_group, &[]);

        pass
    }

    fn bind_group_layout(device: &Device, msaa: Msaa) -> [&'static BindGroupLayout; 2] {
        [
            GlobalContext::global_bind_group_layout(device),
            GlobalContext::sdsm_bind_group_layout(device, msaa),
        ]
    }
}
