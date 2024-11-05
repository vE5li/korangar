use wgpu::{
    include_wgsl, ComputePass, ComputePipeline, ComputePipelineDescriptor, Device, PipelineCompilationOptions, PipelineLayoutDescriptor,
    Queue, ShaderModuleDescriptor, TextureSampleType,
};

use crate::graphics::passes::{BindGroupCount, Cmaa2ComputePassContext, ComputePassContext, Dispatch};
use crate::graphics::{AttachmentTexture, Capabilities, GlobalContext};

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/edge_colors.wgsl");
const DISPATCHER_NAME: &str = "cmaa2 edge colors";

const INPUT_KERNEL_SIZE_X: u32 = 16;
const INPUT_KERNEL_SIZE_Y: u32 = 16;

pub(crate) struct Cmaa2EdgeColorsDispatcher {
    pipeline: ComputePipeline,
}

impl Dispatch<{ BindGroupCount::One }> for Cmaa2EdgeColorsDispatcher {
    type Context = Cmaa2ComputePassContext;
    type DispatchData<'data> = &'data AttachmentTexture;

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
            compilation_options: PipelineCompilationOptions::default(),
            cache: None,
        });

        Self { pipeline }
    }

    fn dispatch(&mut self, pass: &mut ComputePass<'_>, draw_data: Self::DispatchData<'_>) {
        let attachment_size = draw_data.get_unpadded_size();
        let output_kernel_size_x = INPUT_KERNEL_SIZE_X - 2;
        let output_kernel_size_y = INPUT_KERNEL_SIZE_Y - 2;
        let dispatch_x = (attachment_size.width + output_kernel_size_x * 2 - 1) / (output_kernel_size_x * 2);
        let dispatch_y = (attachment_size.height + output_kernel_size_y * 2 - 1) / (output_kernel_size_y * 2);

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(1, draw_data.get_bind_group(), &[]);
        pass.dispatch_workgroups(dispatch_x, dispatch_y, 1);
    }
}
