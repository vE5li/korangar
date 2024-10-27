mod light_culling;

pub(crate) use light_culling::LightCullingDispatcher;
use wgpu::{BindGroupLayout, CommandEncoder, ComputePass, ComputePassDescriptor, Device, Queue};

use super::{BindGroupCount, ComputePassContext};
use crate::graphics::GlobalContext;
const PASS_NAME: &str = "light culling pass pass";

pub(crate) struct LightCullingPassContext {}

impl ComputePassContext<{ BindGroupCount::Two }> for LightCullingPassContext {
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
        pass.set_bind_group(1, &global_context.light_culling_bind_group, &[]);

        pass
    }

    fn bind_group_layout(device: &Device) -> [&'static BindGroupLayout; 2] {
        [
            GlobalContext::global_bind_group_layout(device),
            GlobalContext::light_culling_bind_group_layout(device),
        ]
    }
}
