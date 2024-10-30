use wgpu::{
    include_wgsl, ComputePass, ComputePipeline, ComputePipelineDescriptor, Device, PipelineCompilationOptions, PipelineLayoutDescriptor,
    Queue, ShaderModuleDescriptor,
};

use crate::graphics::passes::light_culling::LightCullingPassContext;
use crate::graphics::passes::{BindGroupCount, ComputePassContext, Dispatch};
use crate::graphics::{calculate_light_tile_count, Capabilities, GlobalContext};
use crate::interface::layout::ScreenSize;

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/light_culling.wgsl");
const DISPATCHER_NAME: &str = "light culling";

pub(crate) struct LightCullingDispatcher {
    pipeline: ComputePipeline,
}

impl Dispatch<{ BindGroupCount::Two }> for LightCullingDispatcher {
    type Context = LightCullingPassContext;
    type DispatchData<'data> = ScreenSize;

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

    fn dispatch(&mut self, pass: &mut ComputePass<'_>, draw_data: Self::DispatchData<'_>) {
        pass.set_pipeline(&self.pipeline);
        let (x, y) = calculate_dispatch_size(draw_data);
        pass.dispatch_workgroups(x, y, 1);
    }
}

fn calculate_dispatch_size(screen_size: ScreenSize) -> (u32, u32) {
    let (tiles_x, tiles_y) = calculate_light_tile_count(screen_size);

    // Round up division by workgroup size (8)
    let dispatch_x = (tiles_x + 7) / 8;
    let dispatch_y = (tiles_y + 7) / 8;

    (dispatch_x, dispatch_y)
}
