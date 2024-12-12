use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Instant;

use cgmath::Vector2;
#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize};
#[cfg(feature = "debug")]
use korangar_debug::profile_block;
use wgpu::util::StagingBelt;
use wgpu::{
    Adapter, CommandBuffer, CommandEncoder, CommandEncoderDescriptor, Device, Extent3d, ImageCopyBuffer, ImageCopyTexture, ImageDataLayout,
    Instance, Maintain, Origin3d, Queue, SurfaceTexture, TextureAspect, TextureFormat, TextureViewDescriptor,
};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use super::{
    AntiAliasingResource, Capabilities, EntityInstruction, FramePacer, FrameStage, GlobalContext, LimitFramerate, ModelInstruction, Msaa,
    Prepare, PresentModeInfo, ScreenSpaceAntiAliasing, ShadowDetail, Ssaa, Surface, TextureSamplerType, RENDER_TO_TEXTURE_FORMAT,
};
use crate::graphics::instruction::RenderInstruction;
use crate::graphics::passes::*;
use crate::interface::layout::ScreenSize;
use crate::loaders::TextureLoader;
use crate::NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS;

pub struct GraphicsEngineDescriptor {
    pub capabilities: Capabilities,
    pub instance: Instance,
    pub adapter: Arc<Adapter>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub texture_loader: Arc<TextureLoader>,
    pub picker_value: Arc<AtomicU64>,
}

/// Bind Group layout:
///
/// The safe default limit for bound bind-groups is 4.
/// When a set of a specific level is not bound, the bindings below it will
/// move up a level.
///
/// Set 0: Global Bindings
///
/// Set 1: Pass Bindings (For example point shadow project view matrices)
///
/// Set 2: Dispatcher / Drawer Bindings (For example indirection buffers)
///
/// Set 3: Resource Bindings (for example a texture group of a map)
pub struct GraphicsEngine {
    capabilities: Capabilities,
    instance: Instance,
    adapter: Arc<Adapter>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    staging_belt: StagingBelt,
    surface: Option<Surface>,
    frame_pacer: FramePacer,
    cpu_stage: FrameStage<Instant>,
    limit_framerate: bool,
    previous_surface_texture_format: Option<TextureFormat>,
    texture_loader: Arc<TextureLoader>,
    engine_context: Option<EngineContext>,
    picker_value: Arc<AtomicU64>,
    entity_sort_buffer: Vec<EntityInstruction>,
    model_sort_buffer: Vec<ModelInstruction>,
}

struct EngineContext {
    global_context: GlobalContext,

    interface_render_pass_context: InterfaceRenderPassContext,
    picker_render_pass_context: PickerRenderPassContext,
    directional_shadow_pass_context: DirectionalShadowRenderPassContext,
    point_shadow_pass_context: PointShadowRenderPassContext,
    light_culling_pass_context: LightCullingPassContext,
    forward_pass_context: ForwardRenderPassContext,
    water_pass_context: WaterRenderPassContext,
    cmaa2_pass_context: Cmaa2ComputePassContext,
    post_processing_pass_context: PostProcessingRenderPassContext,
    screen_blit_pass_context: ScreenBlitRenderPassContext,

    interface_rectangle_drawer: InterfaceRectangleDrawer,
    picker_entity_drawer: PickerEntityDrawer,
    picker_tile_drawer: PickerTileDrawer,
    directional_shadow_model_drawer: DirectionalShadowModelDrawer,
    directional_shadow_entity_drawer: DirectionalShadowEntityDrawer,
    directional_shadow_indicator_drawer: DirectionalShadowIndicatorDrawer,
    point_shadow_entity_drawer: PointShadowEntityDrawer,
    point_shadow_model_drawer: PointShadowModelDrawer,
    point_shadow_indicator_drawer: PointShadowIndicatorDrawer,
    light_culling_dispatcher: LightCullingDispatcher,
    forward_entity_drawer: ForwardEntityDrawer,
    forward_indicator_drawer: ForwardIndicatorDrawer,
    forward_model_drawer: ForwardModelDrawer,
    water_wave_drawer: WaterWaveDrawer,
    cmaa2_edge_colors_dispatcher: Cmaa2EdgeColorsDispatcher,
    cmaa2_calculate_dispatch_args_dispatcher: Cmaa2CalculateDispatchArgsDispatcher,
    cmaa2_process_candidates_dispatcher: Cmaa2ProcessCandidatesDispatcher,
    cmaa2_deferred_color_apply_dispatcher: Cmaa2DeferredColorApplyDispatcher,
    post_processing_effect_drawer: PostProcessingEffectDrawer,
    post_processing_fxaa_drawer: PostProcessingFxaaDrawer,
    post_processing_blitter_drawer: PostProcessingBlitterDrawer,
    post_processing_rectangle_drawer: PostProcessingRectangleDrawer,
    screen_blit_blitter_drawer: ScreenBlitBlitterDrawer,
    #[cfg(feature = "debug")]
    forward_aabb_drawer: ForwardAabbDrawer,
    #[cfg(feature = "debug")]
    forward_circle_drawer: ForwardCircleDrawer,
    #[cfg(feature = "debug")]
    post_processing_buffer_drawer: PostProcessingBufferDrawer,
    #[cfg(feature = "debug")]
    picker_marker_drawer: PickerMarkerDrawer,
}

impl GraphicsEngine {
    pub fn initialize(descriptor: GraphicsEngineDescriptor) -> GraphicsEngine {
        let staging_belt = StagingBelt::new(1048576); // 1 MiB
        let mut frame_pacer = FramePacer::new(60.0);
        let cpu_stage = frame_pacer.create_frame_stage(Instant::now());

        Self {
            capabilities: descriptor.capabilities,
            instance: descriptor.instance,
            adapter: descriptor.adapter,
            device: descriptor.device,
            queue: descriptor.queue,
            staging_belt,
            surface: None,
            frame_pacer,
            cpu_stage,
            limit_framerate: false,
            previous_surface_texture_format: None,
            texture_loader: descriptor.texture_loader,
            engine_context: None,
            picker_value: descriptor.picker_value,
            entity_sort_buffer: vec![],
            model_sort_buffer: vec![],
        }
    }

    pub fn on_resume(
        &mut self,
        window: Arc<Window>,
        triple_buffering: bool,
        vsync: bool,
        limit_framerate: LimitFramerate,
        shadow_detail: ShadowDetail,
        texture_sampler_type: TextureSamplerType,
        msaa: Msaa,
        ssaa: Ssaa,
        screen_space_anti_aliasing: ScreenSpaceAntiAliasing,
        high_quality_interface: bool,
    ) {
        self.set_limit_framerate(limit_framerate);

        // Android devices need to drop the surface on suspend, so we might need to
        // re-create it.
        if self.surface.is_none() {
            time_phase!("create surface", {
                let screen_size: ScreenSize = window.inner_size().max(PhysicalSize::new(1, 1)).into();
                let raw_surface = self.instance.create_surface(window).unwrap();
                let surface = Surface::new(
                    &self.adapter,
                    self.device.clone(),
                    raw_surface,
                    screen_size.width as u32,
                    screen_size.height as u32,
                    triple_buffering,
                    vsync,
                );

                let surface_texture_format = surface.format();

                if self.previous_surface_texture_format != Some(surface_texture_format) {
                    self.previous_surface_texture_format = Some(surface_texture_format);
                    self.engine_context = None;

                    time_phase!("create contexts", {
                        let high_quality_interface = self.check_high_quality_interface_requirements(high_quality_interface, screen_size);
                        let ssaa = self.check_ssaa_requirements(ssaa, screen_size);

                        let global_context = GlobalContext::new(
                            &self.device,
                            &self.queue,
                            &self.capabilities,
                            &self.texture_loader,
                            surface_texture_format,
                            msaa,
                            ssaa,
                            screen_space_anti_aliasing,
                            screen_size,
                            shadow_detail,
                            texture_sampler_type,
                            high_quality_interface,
                        );

                        let interface_render_pass_context =
                            InterfaceRenderPassContext::new(&self.device, &self.queue, &self.texture_loader, &global_context);
                        let picker_render_pass_context =
                            PickerRenderPassContext::new(&self.device, &self.queue, &self.texture_loader, &global_context);
                        let directional_shadow_pass_context =
                            DirectionalShadowRenderPassContext::new(&self.device, &self.queue, &self.texture_loader, &global_context);
                        let point_shadow_pass_context =
                            PointShadowRenderPassContext::new(&self.device, &self.queue, &self.texture_loader, &global_context);
                        let light_culling_pass_context = LightCullingPassContext::new(&self.device, &self.queue, &global_context);
                        let forward_pass_context =
                            ForwardRenderPassContext::new(&self.device, &self.queue, &self.texture_loader, &global_context);
                        let water_pass_context =
                            WaterRenderPassContext::new(&self.device, &self.queue, &self.texture_loader, &global_context);
                        let cmaa2_pass_context = Cmaa2ComputePassContext::new(&self.device, &self.queue, &global_context);
                        let post_processing_pass_context =
                            PostProcessingRenderPassContext::new(&self.device, &self.queue, &self.texture_loader, &global_context);
                        let screen_blit_pass_context =
                            ScreenBlitRenderPassContext::new(&self.device, &self.queue, &self.texture_loader, &global_context);
                    });

                    time_phase!("create computer and drawer", {
                        let interface_rectangle_drawer = InterfaceRectangleDrawer::new(
                            &self.capabilities,
                            &self.device,
                            &self.queue,
                            &global_context,
                            &interface_render_pass_context,
                        );
                        let picker_entity_drawer = PickerEntityDrawer::new(
                            &self.capabilities,
                            &self.device,
                            &self.queue,
                            &global_context,
                            &picker_render_pass_context,
                        );
                        let picker_tile_drawer = PickerTileDrawer::new(
                            &self.capabilities,
                            &self.device,
                            &self.queue,
                            &global_context,
                            &picker_render_pass_context,
                        );
                        let directional_shadow_model_drawer = DirectionalShadowModelDrawer::new(
                            &self.capabilities,
                            &self.device,
                            &self.queue,
                            &global_context,
                            &directional_shadow_pass_context,
                        );
                        let directional_shadow_entity_drawer = DirectionalShadowEntityDrawer::new(
                            &self.capabilities,
                            &self.device,
                            &self.queue,
                            &global_context,
                            &directional_shadow_pass_context,
                        );
                        let directional_shadow_indicator_drawer = DirectionalShadowIndicatorDrawer::new(
                            &self.capabilities,
                            &self.device,
                            &self.queue,
                            &global_context,
                            &directional_shadow_pass_context,
                        );
                        let point_shadow_model_drawer = PointShadowModelDrawer::new(
                            &self.capabilities,
                            &self.device,
                            &self.queue,
                            &global_context,
                            &point_shadow_pass_context,
                        );
                        let point_shadow_entity_drawer = PointShadowEntityDrawer::new(
                            &self.capabilities,
                            &self.device,
                            &self.queue,
                            &global_context,
                            &point_shadow_pass_context,
                        );
                        let point_shadow_indicator_drawer = PointShadowIndicatorDrawer::new(
                            &self.capabilities,
                            &self.device,
                            &self.queue,
                            &global_context,
                            &point_shadow_pass_context,
                        );
                        let light_culling_dispatcher = LightCullingDispatcher::new(
                            &self.capabilities,
                            &self.device,
                            &self.queue,
                            &global_context,
                            &light_culling_pass_context,
                        );
                        let ForwardResources {
                            forward_entity_drawer,
                            forward_indicator_drawer,
                            forward_model_drawer,

                            #[cfg(feature = "debug")]
                            forward_aabb_drawer,

                            #[cfg(feature = "debug")]
                            forward_circle_drawer,
                        } = ForwardResources::create(
                            &self.capabilities,
                            &self.device,
                            &self.queue,
                            &global_context,
                            &forward_pass_context,
                        );
                        let water_wave_drawer = WaterWaveDrawer::new(
                            &self.capabilities,
                            &self.device,
                            &self.queue,
                            &global_context,
                            &water_pass_context,
                        );
                        let Cmaa2Resources {
                            cmaa2_edge_colors_dispatcher,
                            cmaa2_calculate_dispatch_args_dispatcher,
                            cmaa2_process_candidates_dispatcher,
                            cmaa2_deferred_color_apply_dispatcher,
                        } = Cmaa2Resources::create(
                            &self.capabilities,
                            &self.device,
                            &self.queue,
                            &global_context,
                            &cmaa2_pass_context,
                        );
                        let PostProcessingResources {
                            post_processing_effect_drawer,
                            post_processing_fxaa_drawer,
                            post_processing_blitter_drawer,
                            post_processing_rectangle_drawer,
                            #[cfg(feature = "debug")]
                            post_processing_buffer_drawer,
                        } = PostProcessingResources::create(
                            &self.capabilities,
                            &self.device,
                            &self.queue,
                            &global_context,
                            &post_processing_pass_context,
                        );
                        let screen_blit_blitter_drawer = ScreenBlitBlitterDrawer::new(
                            &self.capabilities,
                            &self.device,
                            &self.queue,
                            &global_context,
                            &screen_blit_pass_context,
                        );
                        #[cfg(feature = "debug")]
                        let picker_marker_drawer = PickerMarkerDrawer::new(
                            &self.capabilities,
                            &self.device,
                            &self.queue,
                            &global_context,
                            &picker_render_pass_context,
                        );
                    });

                    self.engine_context = Some(EngineContext {
                        global_context,
                        interface_render_pass_context,
                        picker_render_pass_context,
                        directional_shadow_pass_context,
                        point_shadow_pass_context,
                        light_culling_pass_context,
                        forward_pass_context,
                        water_pass_context,
                        cmaa2_pass_context,
                        post_processing_pass_context,
                        screen_blit_pass_context,
                        interface_rectangle_drawer,
                        picker_entity_drawer,
                        picker_tile_drawer,
                        directional_shadow_model_drawer,
                        directional_shadow_entity_drawer,
                        directional_shadow_indicator_drawer,
                        point_shadow_model_drawer,
                        point_shadow_indicator_drawer,
                        point_shadow_entity_drawer,
                        light_culling_dispatcher,
                        forward_entity_drawer,
                        forward_indicator_drawer,
                        forward_model_drawer,
                        water_wave_drawer,
                        cmaa2_edge_colors_dispatcher,
                        cmaa2_calculate_dispatch_args_dispatcher,
                        cmaa2_process_candidates_dispatcher,
                        cmaa2_deferred_color_apply_dispatcher,
                        post_processing_effect_drawer,
                        post_processing_fxaa_drawer,
                        post_processing_blitter_drawer,
                        post_processing_rectangle_drawer,
                        screen_blit_blitter_drawer,
                        #[cfg(feature = "debug")]
                        forward_aabb_drawer,
                        #[cfg(feature = "debug")]
                        forward_circle_drawer,
                        #[cfg(feature = "debug")]
                        post_processing_buffer_drawer,
                        #[cfg(feature = "debug")]
                        picker_marker_drawer,
                    })
                }

                self.surface = Some(surface);

                #[cfg(feature = "debug")]
                print_debug!("created {}", "surface".magenta());
            });
        }
    }

    fn check_high_quality_interface_requirements(&self, mut high_quality_interface: bool, screen_size: ScreenSize) -> bool {
        if high_quality_interface {
            let max_texture_dimension_2d = self.capabilities.get_max_texture_dimension_2d();
            let interface_size = screen_size * 2.0;

            if max_texture_dimension_2d < interface_size.width as u32 && max_texture_dimension_2d < interface_size.height as u32 {
                high_quality_interface = false;

                #[cfg(feature = "debug")]
                print_debug!(
                    "[{}] can't enable high quality interface because texture would be too large",
                    "error".red()
                );
            }
        }

        high_quality_interface
    }

    fn check_ssaa_requirements(&self, mut ssaa: Ssaa, screen_size: ScreenSize) -> Ssaa {
        if ssaa.supersampling_activated() {
            let max_texture_dimension_2d = self.capabilities.get_max_texture_dimension_2d();
            let forward_size = ssaa.calculate_size(screen_size);

            if max_texture_dimension_2d < forward_size.width as u32 && max_texture_dimension_2d < forward_size.height as u32 {
                ssaa = Ssaa::Off;

                #[cfg(feature = "debug")]
                print_debug!(
                    "[{}] can't enable super sampling because texture would be too large",
                    "error".red()
                );
            }
        }

        ssaa
    }

    pub fn on_suspended(&mut self) {
        // Android devices are expected to drop their surface view.
        if cfg!(target_os = "android") {
            self.surface = None;
        }
    }

    pub fn on_resize(&mut self, screen_size: ScreenSize) {
        if let Some(surface) = self.surface.as_mut() {
            surface.update_window_size(screen_size);
        }
    }

    pub fn set_vsync(&mut self, enabled: bool) {
        if let Some(surface) = self.surface.as_mut() {
            surface.set_vsync(enabled);
        }
    }

    pub fn set_limit_framerate(&mut self, limit_framerate: LimitFramerate) {
        match limit_framerate {
            LimitFramerate::Unlimited => {
                self.limit_framerate = false;
            }
            LimitFramerate::Limit(rate) => {
                self.limit_framerate = true;
                self.frame_pacer.set_monitor_frequency(f64::from(rate));
            }
        }
    }

    pub fn set_triple_buffering(&mut self, enabled: bool) {
        if let Some(surface) = self.surface.as_mut() {
            surface.set_triple_buffering(enabled);
        }
    }

    pub fn set_texture_sampler_type(&mut self, texture_sampler_type: TextureSamplerType) {
        if let Some(engine_context) = self.engine_context.as_mut() {
            engine_context
                .global_context
                .update_texture_sampler(&self.device, &self.capabilities, texture_sampler_type);
        }
    }

    pub fn set_screen_space_anti_aliasing(&mut self, screen_space_anti_aliasing: ScreenSpaceAntiAliasing) {
        if let Some(engine_context) = self.engine_context.as_mut() {
            engine_context
                .global_context
                .update_screen_space_anti_aliasing(&self.device, screen_space_anti_aliasing);
        }
    }

    pub fn set_msaa(&mut self, msaa: Msaa) {
        if let Some(engine_context) = self.engine_context.as_mut() {
            engine_context.global_context.update_msaa(&self.device, msaa);

            let ForwardResources {
                forward_entity_drawer,
                forward_indicator_drawer,
                forward_model_drawer,
                #[cfg(feature = "debug")]
                forward_aabb_drawer,
                #[cfg(feature = "debug")]
                forward_circle_drawer,
            } = ForwardResources::create(
                &self.capabilities,
                &self.device,
                &self.queue,
                &engine_context.global_context,
                &engine_context.forward_pass_context,
            );

            let Cmaa2Resources {
                cmaa2_edge_colors_dispatcher,
                cmaa2_calculate_dispatch_args_dispatcher,
                cmaa2_process_candidates_dispatcher,
                cmaa2_deferred_color_apply_dispatcher,
            } = Cmaa2Resources::create(
                &self.capabilities,
                &self.device,
                &self.queue,
                &engine_context.global_context,
                &engine_context.cmaa2_pass_context,
            );

            let PostProcessingResources {
                post_processing_effect_drawer,
                post_processing_fxaa_drawer,
                post_processing_blitter_drawer,
                post_processing_rectangle_drawer,
                #[cfg(feature = "debug")]
                post_processing_buffer_drawer,
            } = PostProcessingResources::create(
                &self.capabilities,
                &self.device,
                &self.queue,
                &engine_context.global_context,
                &engine_context.post_processing_pass_context,
            );

            engine_context.forward_entity_drawer = forward_entity_drawer;
            engine_context.forward_indicator_drawer = forward_indicator_drawer;
            engine_context.forward_model_drawer = forward_model_drawer;
            engine_context.cmaa2_edge_colors_dispatcher = cmaa2_edge_colors_dispatcher;
            engine_context.cmaa2_calculate_dispatch_args_dispatcher = cmaa2_calculate_dispatch_args_dispatcher;
            engine_context.cmaa2_process_candidates_dispatcher = cmaa2_process_candidates_dispatcher;
            engine_context.cmaa2_deferred_color_apply_dispatcher = cmaa2_deferred_color_apply_dispatcher;
            engine_context.post_processing_effect_drawer = post_processing_effect_drawer;
            engine_context.post_processing_fxaa_drawer = post_processing_fxaa_drawer;
            engine_context.post_processing_blitter_drawer = post_processing_blitter_drawer;
            engine_context.post_processing_rectangle_drawer = post_processing_rectangle_drawer;

            engine_context.water_wave_drawer = WaterWaveDrawer::new(
                &self.capabilities,
                &self.device,
                &self.queue,
                &engine_context.global_context,
                &engine_context.water_pass_context,
            );

            #[cfg(feature = "debug")]
            {
                engine_context.forward_aabb_drawer = forward_aabb_drawer;
                engine_context.forward_circle_drawer = forward_circle_drawer;
                engine_context.post_processing_buffer_drawer = post_processing_buffer_drawer;
            }
        }
    }

    pub fn set_ssaa(&mut self, ssaa: Ssaa) {
        if let Some(engine_context) = self.engine_context.as_mut() {
            engine_context.global_context.update_ssaa(&self.device, ssaa);
        }
    }

    pub fn set_shadow_detail(&mut self, shadow_detail: ShadowDetail) {
        if let Some(engine_context) = self.engine_context.as_mut() {
            engine_context
                .global_context
                .update_shadow_size_textures(&self.device, shadow_detail);
        }
    }

    pub fn set_high_quality_interface(&mut self, high_quality_interface: bool) {
        if let Some(engine_context) = self.engine_context.as_mut() {
            engine_context
                .global_context
                .update_high_quality_interface(&self.device, high_quality_interface);
        }
    }

    pub fn get_backend_name(&self) -> String {
        self.adapter.get_info().backend.to_string()
    }

    pub fn get_present_mode_info(&self) -> PresentModeInfo {
        self.surface.as_ref().unwrap().present_mode_info()
    }

    pub fn get_supported_msaa(&self) -> Vec<(String, Msaa)> {
        self.capabilities
            .get_supported_msaa()
            .iter()
            .map(|msaa| (msaa.to_string(), *msaa))
            .collect()
    }

    pub fn get_window_size(&self) -> Vector2<usize> {
        self.surface.as_ref().unwrap().window_size()
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn wait_for_next_frame(&mut self) -> SurfaceTexture {
        // Before we wait for the next frame, we verify that the surface is still valid.
        if let Some(surface) = self.surface.as_mut()
            && surface.is_invalid()
        {
            #[cfg(feature = "debug")]
            profile_block!("re-configure surface and textures");

            surface.reconfigure();

            let screen_size = surface.window_screen_size();

            let high_quality_interface = self
                .engine_context
                .as_ref()
                .map(|engine_context| engine_context.global_context.high_quality_interface)
                .unwrap_or(false);

            if high_quality_interface && !self.check_high_quality_interface_requirements(high_quality_interface, screen_size) {
                if let Some(engine_context) = self.engine_context.as_mut() {
                    engine_context.global_context.update_high_quality_interface(&self.device, false);
                }
            }

            let ssaa = self
                .engine_context
                .as_ref()
                .map(|engine_context| engine_context.global_context.ssaa)
                .unwrap_or(Ssaa::Off);

            if ssaa.supersampling_activated() && !self.check_ssaa_requirements(ssaa, screen_size).supersampling_activated() {
                if let Some(engine_context) = self.engine_context.as_mut() {
                    engine_context.global_context.update_ssaa(&self.device, Ssaa::Off);
                }
            }

            if let Some(engine_context) = self.engine_context.as_mut() {
                engine_context
                    .global_context
                    .update_screen_size_resources(&self.device, screen_size);
            }
        }

        if self.limit_framerate {
            self.frame_pacer.wait_for_frame();
        }
        self.frame_pacer.begin_frame_stage(self.cpu_stage, Instant::now());

        self.surface.as_mut().expect("surface not set").acquire()
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn render_next_frame(&mut self, frame: SurfaceTexture, mut instruction: RenderInstruction) {
        assert!(instruction.point_light_shadow_caster.len() <= NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS);

        self.sort_instructions(&mut instruction);

        // Reclaim all staging buffers that the GPU has finished reading from.
        self.staging_belt.recall();

        // Calculate and stage the uploading of GPU data that is needed for the frame.
        let prepare_command_buffer = self.prepare_frame_data(&instruction);

        // Record all draw commands.
        let (
            interface_command_buffer,
            picker_command_buffer,
            directional_shadow_command_buffer,
            point_shadow_command_buffer,
            light_culling_command_buffer,
            forward_command_buffer,
            post_processing_command_buffer,
        ) = self.draw_frame(&frame, &instruction);

        // Queue all staging belt writes.
        self.staging_belt.finish();

        self.queue_picker_value();
        self.wait_and_submit_frame(
            prepare_command_buffer,
            interface_command_buffer,
            picker_command_buffer,
            directional_shadow_command_buffer,
            point_shadow_command_buffer,
            light_culling_command_buffer,
            forward_command_buffer,
            post_processing_command_buffer,
        );

        // Schedule the presentation of the frame.
        frame.present();

        self.frame_pacer.end_frame_stage(self.cpu_stage, Instant::now());
    }

    /// We use glidesort here, since we need a stable sorting algorith, or else
    /// we could get Z-fighting and also want to re-use the sorting buffers
    /// between frames, so that we don't need to allocate each time.
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn sort_instructions(&mut self, instructions: &mut RenderInstruction) {
        // Back to front for entities.
        glidesort::sort_with_vec_by(instructions.entities, &mut self.entity_sort_buffer, |a, b| {
            b.distance.total_cmp(&a.distance)
        });

        for batch in instructions.model_batches {
            let start = batch.offset;
            let end = batch.offset + batch.count;

            glidesort::sort_with_vec_by(&mut instructions.models[start..end], &mut self.model_sort_buffer, |a, b| {
                match (a.transparent, b.transparent) {
                    // Front to back for opaque models.
                    (false, false) => a.distance.total_cmp(&b.distance),
                    // Back to front for transparent models.
                    (true, true) => b.distance.total_cmp(&a.distance),
                    // Opaque objects come before transparent ones.
                    (false, true) => std::cmp::Ordering::Less,
                    (true, false) => std::cmp::Ordering::Greater,
                }
            });
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn wait_and_submit_frame(
        &mut self,
        prepare_command_buffer: CommandBuffer,
        interface_command_buffer: CommandBuffer,
        picker_command_buffer: CommandBuffer,
        directional_shadow_command_buffer: CommandBuffer,
        point_shadow_command_buffer: CommandBuffer,
        light_culling_command_buffer: CommandBuffer,
        forward_command_buffer: CommandBuffer,
        post_processing_command_buffer: CommandBuffer,
    ) {
        // We have gathered all data for the next frame and can now wait until the GPU
        // is ready to accept the command buffers for the next frame. This is the
        // best time to resolve async operations like reading the piker value that need
        // to be synced with the GPU.
        self.device.poll(Maintain::Wait);
        self.queue.submit([
            prepare_command_buffer,
            interface_command_buffer,
            picker_command_buffer,
            directional_shadow_command_buffer,
            point_shadow_command_buffer,
            light_culling_command_buffer,
            forward_command_buffer,
            post_processing_command_buffer,
        ]);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn queue_picker_value(&mut self) {
        if let Some(engine_context) = self.engine_context.as_ref() {
            engine_context
                .global_context
                .picker_value_buffer
                .queue_read_u64(self.picker_value.clone());
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn prepare_frame_data(&mut self, instruction: &RenderInstruction) -> CommandBuffer {
        let context = self.engine_context.as_mut().unwrap();

        // We spawn a task for all the potentially long-running prepare functions.
        rayon::in_place_scope(|scope| {
            scope.spawn(|_| {
                context.directional_shadow_entity_drawer.prepare(&self.device, instruction);
                context.directional_shadow_model_drawer.prepare(&self.device, instruction);
            });
            scope.spawn(|_| {
                context.forward_entity_drawer.prepare(&self.device, instruction);
                context.forward_model_drawer.prepare(&self.device, instruction);
            });
            scope.spawn(|_| {
                context.interface_rectangle_drawer.prepare(&self.device, instruction);
                context.water_wave_drawer.prepare(&self.device, instruction);
            });
            scope.spawn(|_| {
                context.point_shadow_entity_drawer.prepare(&self.device, instruction);
                context.point_shadow_model_drawer.prepare(&self.device, instruction);
            });
            scope.spawn(|_| {
                context.post_processing_effect_drawer.prepare(&self.device, instruction);
                context.post_processing_rectangle_drawer.prepare(&self.device, instruction);
            });
            #[cfg(feature = "debug")]
            scope.spawn(|_| {
                context.picker_marker_drawer.prepare(&self.device, instruction);
                context.forward_aabb_drawer.prepare(&self.device, instruction);
            });
            #[cfg(feature = "debug")]
            scope.spawn(|_| {
                context.post_processing_buffer_drawer.prepare(&self.device, instruction);
                context.forward_circle_drawer.prepare(&self.device, instruction);
            });

            context.global_context.prepare(&self.device, instruction);
            context.directional_shadow_pass_context.prepare(&self.device, instruction);
            context.point_shadow_pass_context.prepare(&self.device, instruction);
            context.picker_entity_drawer.prepare(&self.device, instruction);
        });

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor::default());

        let mut visitor = UploadVisitor {
            device: &self.device,
            staging_belt: &mut self.staging_belt,
            encoder: &mut encoder,
        };

        visitor.upload(&mut context.directional_shadow_entity_drawer);
        visitor.upload(&mut context.directional_shadow_model_drawer);
        visitor.upload(&mut context.directional_shadow_pass_context);
        visitor.upload(&mut context.global_context);
        visitor.upload(&mut context.interface_rectangle_drawer);
        visitor.upload(&mut context.picker_entity_drawer);
        visitor.upload(&mut context.point_shadow_entity_drawer);
        visitor.upload(&mut context.point_shadow_model_drawer);
        visitor.upload(&mut context.point_shadow_pass_context);
        visitor.upload(&mut context.post_processing_effect_drawer);
        visitor.upload(&mut context.forward_entity_drawer);
        visitor.upload(&mut context.forward_model_drawer);
        visitor.upload(&mut context.water_wave_drawer);
        visitor.upload(&mut context.post_processing_rectangle_drawer);

        #[cfg(feature = "debug")]
        {
            visitor.upload(&mut context.forward_aabb_drawer);
            visitor.upload(&mut context.post_processing_buffer_drawer);
            visitor.upload(&mut context.forward_circle_drawer);
            visitor.upload(&mut context.picker_marker_drawer);
        }

        encoder.finish()
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn draw_frame(
        &mut self,
        frame: &SurfaceTexture,
        instruction: &RenderInstruction,
    ) -> (
        CommandBuffer,
        CommandBuffer,
        CommandBuffer,
        CommandBuffer,
        CommandBuffer,
        CommandBuffer,
        CommandBuffer,
    ) {
        let frame_view = &frame.texture.create_view(&TextureViewDescriptor::default());
        let engine_context = self.engine_context.as_mut().unwrap();

        let mut picker_encoder = self.device.create_command_encoder(&CommandEncoderDescriptor::default());
        let mut interface_encoder = self.device.create_command_encoder(&CommandEncoderDescriptor::default());
        let mut directional_shadow_encoder = self.device.create_command_encoder(&CommandEncoderDescriptor::default());
        let mut point_shadow_encoder = self.device.create_command_encoder(&CommandEncoderDescriptor::default());
        let mut light_culling_encoder = self.device.create_command_encoder(&CommandEncoderDescriptor::default());
        let mut forward_encoder = self.device.create_command_encoder(&CommandEncoderDescriptor::default());
        let mut post_processing_encoder = self.device.create_command_encoder(&CommandEncoderDescriptor::default());

        rayon::in_place_scope(|scope| {
            // Picker Pass
            scope.spawn(|_| {
                let mut render_pass =
                    engine_context
                        .picker_render_pass_context
                        .create_pass(&mut picker_encoder, &engine_context.global_context, None);

                engine_context
                    .picker_tile_drawer
                    .draw(&mut render_pass, instruction.map_picker_tile_vertex_buffer);
                engine_context.picker_entity_drawer.draw(&mut render_pass, instruction.entities);
                #[cfg(feature = "debug")]
                {
                    engine_context.picker_marker_drawer.draw(&mut render_pass, None);
                }

                drop(render_pass);

                // Copy the picker value from the texture into the buffer.
                let bytes_per_row = engine_context.global_context.picker_buffer_texture.get_bytes_per_row();
                let unpadded_texture_size = engine_context.global_context.picker_buffer_texture.get_unpadded_size();
                let x = (unpadded_texture_size.width - 1).min(instruction.picker_position.left as u32);
                let y = (unpadded_texture_size.height - 1).min(instruction.picker_position.top as u32);

                picker_encoder.copy_texture_to_buffer(
                    ImageCopyTexture {
                        texture: engine_context.global_context.picker_buffer_texture.get_texture(),
                        mip_level: 0,
                        origin: Origin3d { x, y, z: 0 },
                        aspect: TextureAspect::All,
                    },
                    ImageCopyBuffer {
                        buffer: engine_context.global_context.picker_value_buffer.get_buffer(),
                        layout: ImageDataLayout {
                            offset: 0,
                            bytes_per_row,
                            rows_per_image: None,
                        },
                    },
                    Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                );
            });

            // Interface Pass
            scope.spawn(|_| {
                let mut render_pass = engine_context.interface_render_pass_context.create_pass(
                    &mut interface_encoder,
                    &engine_context.global_context,
                    instruction.clear_interface,
                );

                engine_context
                    .interface_rectangle_drawer
                    .draw(&mut render_pass, instruction.interface);
            });

            // Directional Shadow Caster Pass
            scope.spawn(|_| {
                let mut render_pass = engine_context.directional_shadow_pass_context.create_pass(
                    &mut directional_shadow_encoder,
                    &engine_context.global_context,
                    None,
                );

                let draw_data = ModelBatchDrawData {
                    batches: instruction.directional_model_batches,
                    instructions: instruction.directional_shadow_models,
                    #[cfg(feature = "debug")]
                    show_wireframe: false,
                };

                engine_context.directional_shadow_model_drawer.draw(&mut render_pass, draw_data);
                engine_context
                    .directional_shadow_indicator_drawer
                    .draw(&mut render_pass, instruction.indicator.as_ref());
                engine_context
                    .directional_shadow_entity_drawer
                    .draw(&mut render_pass, instruction.directional_shadow_entities);
            });

            // Point Shadow Caster Pass
            scope.spawn(|_| {
                (0..instruction.point_light_shadow_caster.len()).for_each(|shadow_caster_index| {
                    (0..6).for_each(|face_index| {
                        let pass_data = PointShadowData {
                            shadow_caster_index,
                            face_index,
                        };
                        let model_data = PointShadowModelBatchData {
                            pass_data,
                            caster: instruction.point_light_shadow_caster,
                            instructions: instruction.point_shadow_models,
                        };
                        let entity_data = PointShadowEntityBatchData {
                            pass_data,
                            caster: instruction.point_light_shadow_caster,
                            instructions: instruction.point_shadow_entities,
                        };

                        let mut render_pass = engine_context.point_shadow_pass_context.create_pass(
                            &mut point_shadow_encoder,
                            &engine_context.global_context,
                            pass_data,
                        );

                        engine_context.point_shadow_model_drawer.draw(&mut render_pass, &model_data);
                        engine_context.point_shadow_entity_drawer.draw(&mut render_pass, &entity_data);
                        engine_context
                            .point_shadow_indicator_drawer
                            .draw(&mut render_pass, instruction.indicator.as_ref());
                    });
                });
            });

            // Light Culling Pass
            scope.spawn(|_| {
                let mut compute_pass =
                    engine_context
                        .light_culling_pass_context
                        .create_pass(&mut light_culling_encoder, &engine_context.global_context, None);

                engine_context
                    .light_culling_dispatcher
                    .dispatch(&mut compute_pass, engine_context.global_context.forward_size);

                drop(compute_pass);

                // Forward Pass
                let mut render_pass =
                    engine_context
                        .forward_pass_context
                        .create_pass(&mut forward_encoder, &engine_context.global_context, None);

                let draw_data = ModelBatchDrawData {
                    batches: instruction.model_batches,
                    instructions: instruction.models,
                    #[cfg(feature = "debug")]
                    show_wireframe: instruction.render_settings.show_wireframe,
                };

                engine_context.forward_entity_drawer.draw(&mut render_pass, instruction.entities);

                engine_context
                    .forward_indicator_drawer
                    .draw(&mut render_pass, instruction.indicator.as_ref());

                engine_context.forward_model_drawer.draw(&mut render_pass, draw_data);

                #[cfg(feature = "debug")]
                {
                    engine_context.forward_aabb_drawer.draw(&mut render_pass, None);
                    engine_context.forward_circle_drawer.draw(&mut render_pass, None);
                }

                if instruction.water.is_some() {
                    drop(render_pass);

                    let mut render_pass =
                        engine_context
                            .water_pass_context
                            .create_pass(&mut forward_encoder, &engine_context.global_context, None);

                    engine_context
                        .water_wave_drawer
                        .draw(&mut render_pass, &engine_context.global_context.forward_depth_texture);
                }
            });

            // Post Processing Pass
            let render_pass = if let Some(supersampled_color_texture) = engine_context.global_context.supersampled_color_texture.as_ref() {
                let mut render_pass = engine_context.post_processing_pass_context.create_pass(
                    &mut post_processing_encoder,
                    &engine_context.global_context,
                    supersampled_color_texture,
                );

                let blitter_data = PostProcessingBlitterDrawData {
                    target_texture_format: RENDER_TO_TEXTURE_FORMAT,
                    source_texture: engine_context.global_context.get_forward_texture(),
                    luma_in_alpha: false,
                    alpha_blending: false,
                };

                engine_context.post_processing_blitter_drawer.draw(&mut render_pass, blitter_data);

                render_pass
            } else {
                engine_context.post_processing_pass_context.create_pass(
                    &mut post_processing_encoder,
                    &engine_context.global_context,
                    engine_context.global_context.get_forward_texture(),
                )
            };

            let mut render_pass = match &engine_context.global_context.screen_space_anti_aliasing {
                ScreenSpaceAntiAliasing::Off => render_pass,
                ScreenSpaceAntiAliasing::Fxaa => {
                    drop(render_pass);

                    let AntiAliasingResource::Fxaa(fxaa_resources) = &engine_context.global_context.anti_aliasing_resources else {
                        panic!("fxaa resources not set")
                    };

                    // We blit the forward texture and calculate the luma in the alpha channel.
                    let mut render_pass = engine_context.post_processing_pass_context.create_pass(
                        &mut post_processing_encoder,
                        &engine_context.global_context,
                        &fxaa_resources.color_with_luma_texture,
                    );

                    let blitter_data = PostProcessingBlitterDrawData {
                        target_texture_format: fxaa_resources.color_with_luma_texture.get_format(),
                        source_texture: engine_context.global_context.get_color_texture(),
                        luma_in_alpha: true,
                        alpha_blending: false,
                    };
                    engine_context.post_processing_blitter_drawer.draw(&mut render_pass, blitter_data);

                    drop(render_pass);

                    let mut render_pass = engine_context.post_processing_pass_context.create_pass(
                        &mut post_processing_encoder,
                        &engine_context.global_context,
                        engine_context.global_context.get_color_texture(),
                    );

                    engine_context
                        .post_processing_fxaa_drawer
                        .draw(&mut render_pass, &fxaa_resources.color_with_luma_texture);

                    render_pass
                }
                ScreenSpaceAntiAliasing::Cmaa2 => {
                    drop(render_pass);

                    let AntiAliasingResource::Cmaa2(cmaa2_resources) = &engine_context.global_context.anti_aliasing_resources else {
                        panic!("cmaa2 resources not set")
                    };

                    let mut compute_pass =
                        engine_context
                            .cmaa2_pass_context
                            .create_pass(&mut post_processing_encoder, &engine_context.global_context, None);

                    let color_input_texture = engine_context.global_context.get_color_texture();

                    engine_context
                        .cmaa2_edge_colors_dispatcher
                        .dispatch(&mut compute_pass, color_input_texture);

                    engine_context
                        .cmaa2_calculate_dispatch_args_dispatcher
                        .dispatch(&mut compute_pass, Cmaa2CalculateDispatchArgsDispatchData::ProcessCandidates);

                    let process_candidates_data = Cmaa2ProcessCandidatesDispatcherDispatchData {
                        color_input_texture,
                        dispatch_indirect_args_buffer: &cmaa2_resources.indirect_buffer,
                    };
                    engine_context
                        .cmaa2_process_candidates_dispatcher
                        .dispatch(&mut compute_pass, process_candidates_data);

                    engine_context
                        .cmaa2_calculate_dispatch_args_dispatcher
                        .dispatch(&mut compute_pass, Cmaa2CalculateDispatchArgsDispatchData::DeferredColorApply);

                    let output_bind_group = &GlobalContext::create_cmaa2_output_bind_group(&self.device, color_input_texture);

                    let deferred_color_apply_data = Cmaa2DeferredColorApplyDispatchData {
                        dispatch_indirect_args_buffer: &cmaa2_resources.indirect_buffer,
                        output_bind_group,
                    };
                    engine_context
                        .cmaa2_deferred_color_apply_dispatcher
                        .dispatch(&mut compute_pass, deferred_color_apply_data);

                    drop(compute_pass);

                    engine_context.post_processing_pass_context.create_pass(
                        &mut post_processing_encoder,
                        &engine_context.global_context,
                        engine_context.global_context.get_color_texture(),
                    )
                }
            };

            let rectangle_data = PostProcessingRectangleDrawData {
                layer: PostProcessingRectangleLayer::Bottom,
                instructions: instruction.bottom_layer_rectangles,
            };
            engine_context
                .post_processing_rectangle_drawer
                .draw(&mut render_pass, rectangle_data);

            engine_context
                .post_processing_effect_drawer
                .draw(&mut render_pass, instruction.effects);

            let rectangle_data = PostProcessingRectangleDrawData {
                layer: PostProcessingRectangleLayer::Middle,
                instructions: instruction.middle_layer_rectangles,
            };
            engine_context
                .post_processing_rectangle_drawer
                .draw(&mut render_pass, rectangle_data);

            #[cfg(feature = "debug")]
            {
                let buffer_data = PostProcessingBufferDrawData {
                    render_settings: &instruction.render_settings,
                    debug_bind_group: &engine_context.global_context.debug_bind_group,
                };

                engine_context.post_processing_buffer_drawer.draw(&mut render_pass, buffer_data);
            }

            if instruction.show_interface {
                let blitter_data = PostProcessingBlitterDrawData {
                    target_texture_format: RENDER_TO_TEXTURE_FORMAT,
                    source_texture: &engine_context.global_context.interface_buffer_texture,
                    luma_in_alpha: false,
                    alpha_blending: true,
                };
                engine_context.post_processing_blitter_drawer.draw(&mut render_pass, blitter_data);
            }

            let rectangle_data = PostProcessingRectangleDrawData {
                layer: PostProcessingRectangleLayer::Top,
                instructions: instruction.top_layer_rectangles,
            };
            engine_context
                .post_processing_rectangle_drawer
                .draw(&mut render_pass, rectangle_data);

            // We can now do the final blit to the surface texture.
            drop(render_pass);

            let mut render_pass = engine_context.screen_blit_pass_context.create_pass(
                &mut post_processing_encoder,
                &engine_context.global_context,
                frame_view,
            );

            let color_texture = engine_context.global_context.get_color_texture();

            engine_context.screen_blit_blitter_drawer.draw(&mut render_pass, color_texture);
        });

        (
            picker_encoder.finish(),
            interface_encoder.finish(),
            directional_shadow_encoder.finish(),
            point_shadow_encoder.finish(),
            light_culling_encoder.finish(),
            forward_encoder.finish(),
            post_processing_encoder.finish(),
        )
    }
}

struct UploadVisitor<'a> {
    device: &'a Device,
    staging_belt: &'a mut StagingBelt,
    encoder: &'a mut CommandEncoder,
}

impl<'a> UploadVisitor<'a> {
    fn upload(&mut self, context: &mut impl Prepare) {
        context.upload(self.device, self.staging_belt, self.encoder);
    }
}

struct ForwardResources {
    forward_entity_drawer: ForwardEntityDrawer,
    forward_indicator_drawer: ForwardIndicatorDrawer,
    forward_model_drawer: ForwardModelDrawer,

    #[cfg(feature = "debug")]
    forward_aabb_drawer: ForwardAabbDrawer,
    #[cfg(feature = "debug")]
    forward_circle_drawer: ForwardCircleDrawer,
}

impl ForwardResources {
    fn create(
        capabilities: &Capabilities,
        device: &Device,
        queue: &Queue,
        global_context: &GlobalContext,
        forward_pass_context: &ForwardRenderPassContext,
    ) -> Self {
        let forward_entity_drawer = ForwardEntityDrawer::new(capabilities, device, queue, global_context, forward_pass_context);
        let forward_indicator_drawer = ForwardIndicatorDrawer::new(capabilities, device, queue, global_context, forward_pass_context);
        let forward_model_drawer = ForwardModelDrawer::new(capabilities, device, queue, global_context, forward_pass_context);
        #[cfg(feature = "debug")]
        let forward_aabb_drawer = ForwardAabbDrawer::new(capabilities, device, queue, global_context, forward_pass_context);
        #[cfg(feature = "debug")]
        let forward_circle_drawer = ForwardCircleDrawer::new(capabilities, device, queue, global_context, forward_pass_context);

        Self {
            forward_entity_drawer,
            forward_indicator_drawer,
            forward_model_drawer,
            #[cfg(feature = "debug")]
            forward_aabb_drawer,
            #[cfg(feature = "debug")]
            forward_circle_drawer,
        }
    }
}

struct Cmaa2Resources {
    cmaa2_edge_colors_dispatcher: Cmaa2EdgeColorsDispatcher,
    cmaa2_calculate_dispatch_args_dispatcher: Cmaa2CalculateDispatchArgsDispatcher,
    cmaa2_process_candidates_dispatcher: Cmaa2ProcessCandidatesDispatcher,
    cmaa2_deferred_color_apply_dispatcher: Cmaa2DeferredColorApplyDispatcher,
}

impl Cmaa2Resources {
    fn create(
        capabilities: &Capabilities,
        device: &Device,
        queue: &Queue,
        global_context: &GlobalContext,
        cmaa2_pass_context: &Cmaa2ComputePassContext,
    ) -> Self {
        let cmaa2_edge_colors_dispatcher = Cmaa2EdgeColorsDispatcher::new(capabilities, device, queue, global_context, cmaa2_pass_context);
        let cmaa2_calculate_dispatch_args_dispatcher =
            Cmaa2CalculateDispatchArgsDispatcher::new(capabilities, device, queue, global_context, cmaa2_pass_context);
        let cmaa2_process_candidates_dispatcher =
            Cmaa2ProcessCandidatesDispatcher::new(capabilities, device, queue, global_context, cmaa2_pass_context);
        let cmaa2_deferred_color_apply_dispatcher =
            Cmaa2DeferredColorApplyDispatcher::new(capabilities, device, queue, global_context, cmaa2_pass_context);

        Self {
            cmaa2_edge_colors_dispatcher,
            cmaa2_calculate_dispatch_args_dispatcher,
            cmaa2_process_candidates_dispatcher,
            cmaa2_deferred_color_apply_dispatcher,
        }
    }
}

struct PostProcessingResources {
    post_processing_effect_drawer: PostProcessingEffectDrawer,
    post_processing_fxaa_drawer: PostProcessingFxaaDrawer,
    post_processing_blitter_drawer: PostProcessingBlitterDrawer,
    post_processing_rectangle_drawer: PostProcessingRectangleDrawer,
    #[cfg(feature = "debug")]
    post_processing_buffer_drawer: PostProcessingBufferDrawer,
}

impl PostProcessingResources {
    fn create(
        capabilities: &Capabilities,
        device: &Device,
        queue: &Queue,
        global_context: &GlobalContext,
        post_processing_pass_context: &PostProcessingRenderPassContext,
    ) -> Self {
        let post_processing_effect_drawer =
            PostProcessingEffectDrawer::new(capabilities, device, queue, global_context, post_processing_pass_context);
        let post_processing_fxaa_drawer =
            PostProcessingFxaaDrawer::new(capabilities, device, queue, global_context, post_processing_pass_context);
        let post_processing_blitter_drawer =
            PostProcessingBlitterDrawer::new(capabilities, device, queue, global_context, post_processing_pass_context);
        let post_processing_rectangle_drawer =
            PostProcessingRectangleDrawer::new(capabilities, device, queue, global_context, post_processing_pass_context);
        #[cfg(feature = "debug")]
        let post_processing_buffer_drawer =
            PostProcessingBufferDrawer::new(capabilities, device, queue, global_context, post_processing_pass_context);

        Self {
            post_processing_effect_drawer,
            post_processing_fxaa_drawer,
            post_processing_blitter_drawer,
            post_processing_rectangle_drawer,
            #[cfg(feature = "debug")]
            post_processing_buffer_drawer,
        }
    }
}
