use std::sync::Arc;

use vulkano::device::Device;
use vulkano::image::SampleCount;
use vulkano::pipeline::graphics::color_blend::{AttachmentBlend, ColorBlendState};
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::pipeline::graphics::input_assembly::{InputAssemblyState, PrimitiveTopology};
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexDefinition, VertexInputState};
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo};
use vulkano::render_pass::Subpass;
use vulkano::shader::{EntryPoint, SpecializationConstant};

use super::SubpassAttachments;

pub(super) struct PipelineBuilder<'a, E, const S: SubpassAttachments>
where
    E: IntoIterator<Item = &'a EntryPoint>,
{
    stages: E,
    vertex_input_state: VertexInputState,
    input_assembly_state: InputAssemblyState,
    viewport_state: ViewportState,
    rasterization_state: RasterizationState,
    multisample_state: MultisampleState,
    color_blend_state: Option<ColorBlendState>,
    depth_stencil_state: Option<DepthStencilState>,
}

impl<'a, E, const S: SubpassAttachments> PipelineBuilder<'a, E, S>
where
    E: IntoIterator<Item = &'a EntryPoint>,
{
    pub(super) fn new(stages: E) -> Self {
        let color_blend_state = match S.color {
            0 => None,
            count => Some(ColorBlendState::new(count)),
        };

        let depth_stencil_state = match S.depth {
            0 => None,
            _ => Some(DepthStencilState::default()),
        };

        Self {
            stages,
            input_assembly_state: InputAssemblyState::default(),
            vertex_input_state: VertexInputState::default(),
            viewport_state: ViewportState::viewport_dynamic_scissor_irrelevant(),
            rasterization_state: RasterizationState::default(),
            multisample_state: MultisampleState::default(),
            color_blend_state,
            depth_stencil_state,
        }
    }

    pub(super) fn topology(mut self, topology: PrimitiveTopology) -> Self {
        self.input_assembly_state = InputAssemblyState::new().topology(topology);
        self
    }

    pub(super) fn vertex_input_state<T: Vertex>(mut self, vertex_shader: &EntryPoint) -> Self {
        self.vertex_input_state = T::per_vertex().definition(&vertex_shader.info().input_interface).unwrap();
        self
    }

    pub(super) fn fixed_viewport(mut self, viewport: Viewport) -> Self {
        self.viewport_state = ViewportState::viewport_fixed_scissor_irrelevant(std::iter::once(viewport));
        self
    }

    pub(super) fn rasterization_state(mut self, rasterization_state: RasterizationState) -> Self {
        self.rasterization_state = rasterization_state;
        self
    }

    pub(super) fn multisample(mut self, sample_count: SampleCount) -> Self {
        self.multisample_state = MultisampleState {
            rasterization_samples: sample_count,
            ..Default::default()
        };
        self
    }

    // TODO: only implement this for COUNT > 0 so that the unwrap cannot fail
    pub(super) fn blend_alpha(mut self) -> Self {
        let new_state = self.color_blend_state.take().unwrap().blend_alpha();
        self.color_blend_state = Some(new_state);
        self
    }

    // TODO: only implement this for COUNT > 0 so that the unwrap cannot fail
    pub(super) fn color_blend(mut self, attachment_blend: AttachmentBlend) -> Self {
        let new_state = self.color_blend_state.take().unwrap().blend(attachment_blend);
        self.color_blend_state = Some(new_state);
        self
    }

    // TODO: only implement this for COUNT > 0 so that the unwrap cannot fail
    pub(super) fn simple_depth_test(mut self) -> Self {
        self.depth_stencil_state = Some(DepthStencilState::simple_depth_test());
        self
    }

    // TODO: only implement this for COUNT > 0 so that the unwrap cannot fail
    pub(super) fn depth_stencil_state(mut self, depth_stencil_state: DepthStencilState) -> Self {
        self.depth_stencil_state = Some(depth_stencil_state);
        self
    }

    pub(super) fn build(self, device: Arc<Device>, subpass: Subpass) -> Arc<GraphicsPipeline> {
        let stages = self
            .stages
            .into_iter()
            .map(|entry| PipelineShaderStageCreateInfo::new(entry.clone()))
            .collect();

        let layout = PipelineLayout::new(
            device.clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(device.clone())
                .unwrap(),
        )
        .unwrap();

        GraphicsPipeline::new(device, None, GraphicsPipelineCreateInfo {
            stages,
            vertex_input_state: Some(self.vertex_input_state),
            input_assembly_state: Some(self.input_assembly_state),
            viewport_state: Some(self.viewport_state),
            rasterization_state: Some(self.rasterization_state),
            multisample_state: Some(self.multisample_state),
            color_blend_state: self.color_blend_state,
            depth_stencil_state: self.depth_stencil_state,
            subpass: Some(subpass.into()),
            ..GraphicsPipelineCreateInfo::layout(layout)
        })
        .unwrap()
    }

    pub(super) fn build_with_specialization<'b, C, H>(
        self,
        device: Arc<Device>,
        subpass: Subpass,
        specialization_constants: C,
    ) -> Arc<GraphicsPipeline>
    where
        C: IntoIterator<Item = H>,
        H: IntoIterator<Item = &'b (u32, SpecializationConstant)>,
    {
        let stages = self
            .stages
            .into_iter()
            .zip(specialization_constants)
            .map(|(entry, constants)| PipelineShaderStageCreateInfo {
                specialization_info: constants.into_iter().cloned().collect(),
                ..PipelineShaderStageCreateInfo::new(entry.clone())
            })
            .collect();

        let layout = PipelineLayout::new(
            device.clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(device.clone())
                .unwrap(),
        )
        .unwrap();

        GraphicsPipeline::new(device, None, GraphicsPipelineCreateInfo {
            stages,
            vertex_input_state: Some(self.vertex_input_state),
            input_assembly_state: Some(self.input_assembly_state),
            viewport_state: Some(self.viewport_state),
            rasterization_state: Some(RasterizationState::default()),
            multisample_state: Some(self.multisample_state),
            color_blend_state: self.color_blend_state,
            depth_stencil_state: self.depth_stencil_state,
            subpass: Some(subpass.into()),
            ..GraphicsPipelineCreateInfo::layout(layout)
        })
        .unwrap()
    }
}
