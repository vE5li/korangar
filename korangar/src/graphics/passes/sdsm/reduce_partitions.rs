use wgpu::{
    ComputePass, ComputePipeline, ComputePipelineDescriptor, Device, PipelineCompilationOptions, PipelineLayoutDescriptor, Queue,
    ShaderModuleDescriptor, include_wgsl,
};

use crate::graphics::passes::{BindGroupCount, ComputePassContext, Dispatch, SdsmPassContext};
use crate::graphics::shader_compiler::ShaderCompiler;
use crate::graphics::{Capabilities, GlobalContext, ScreenSize};

const SHADER: ShaderModuleDescriptor = include_wgsl!("../../../../shaders/passes/sdsm/reduce_partitions.wgsl");
const SHADER_MSAA: ShaderModuleDescriptor = include_wgsl!("../../../../shaders/passes/sdsm/reduce_partitions_msaa.wgsl");

const DISPATCHER_NAME: &str = "reduce partitions";

pub(crate) struct ReducePartitionsDispatcher {
    pipeline: ComputePipeline,
}

impl Dispatch<{ BindGroupCount::Two }> for ReducePartitionsDispatcher {
    type Context = SdsmPassContext;
    type DispatchData<'data> = ScreenSize;

    fn new(
        _capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        _shader_compiler: &ShaderCompiler,
        global_context: &GlobalContext,
        _compute_pass_context: &Self::Context,
    ) -> Self {
        let shader_module = match global_context.msaa.multisampling_activated() {
            false => device.create_shader_module(SHADER),
            true => device.create_shader_module(SHADER_MSAA),
        };

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
            entry_point: Some("main"),
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
        let (x, y) = calculate_dispatch_size(draw_data);
        pass.dispatch_workgroups(x, y, 1);
    }
}

fn calculate_dispatch_size(forward_size: ScreenSize) -> (u32, u32) {
    const REDUCE_TILE_DIM: u32 = 64;
    let tiles_x = (forward_size.width.ceil() as u32).div_ceil(REDUCE_TILE_DIM);
    let tiles_y = (forward_size.height.ceil() as u32).div_ceil(REDUCE_TILE_DIM);
    (tiles_x, tiles_y)
}
