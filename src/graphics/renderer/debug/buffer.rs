mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/debug_vertex_shader.glsl"
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/debug_fragment_shader.glsl"
    }
}

use std::sync::Arc;
use std::iter;

use vulkano::device::Device;
use vulkano::pipeline::{ GraphicsPipeline, PipelineBindPoint, Pipeline };
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::viewport::{ Viewport, ViewportState };
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::sampler::{ Sampler, Filter, SamplerAddressMode };
use vulkano::render_pass::Subpass;
use vulkano::shader::ShaderModule;
use vulkano::sync::GpuFuture;

use crate::graphics::*;
use crate::loaders::TextureLoader;

use self::fragment_shader::ty::Constants;

pub struct DebugRenderer {
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    nearest_sampler: Arc<Sampler>,
    pub object_texture: Texture,
    pub light_texture: Texture,
    pub sound_texture: Texture,
    pub effect_texture: Texture,
    pub particle_texture: Texture,
    pub entity_texture: Texture,
    pub tile_textures: Vec<Texture>,
    pub step_textures: Vec<Texture>,
}

impl DebugRenderer {

    pub fn new(device: Arc<Device>, subpass: Subpass, viewport: Viewport, texture_loader: &mut TextureLoader, texture_future: &mut Box<dyn GpuFuture + 'static>) -> Self {

        let vertex_shader = vertex_shader::load(device.clone()).unwrap();
        let fragment_shader = fragment_shader::load(device.clone()).unwrap();
        let pipeline = Self::create_pipeline(device.clone(), subpass, viewport, &vertex_shader, &fragment_shader);

        let object_texture = texture_loader.get("assets/object.png", texture_future).unwrap();
        let light_texture = texture_loader.get("assets/light.png", texture_future).unwrap();
        let sound_texture = texture_loader.get("assets/sound.png", texture_future).unwrap();
        let effect_texture = texture_loader.get("assets/effect.png", texture_future).unwrap();
        let particle_texture = texture_loader.get("assets/particle.png", texture_future).unwrap();
        let entity_texture = texture_loader.get("assets/entity.png", texture_future).unwrap();
        let tile_textures = (0..7_i32).map(|index| texture_loader.get(&format!("assets/{}.png", index), texture_future).unwrap()).collect();
        let step_textures = ["goal", "straight", "diagonal"].iter().map(|index| texture_loader.get(&format!("assets/{}.png", index), texture_future).unwrap()).collect();
 
        let nearest_sampler = Sampler::start(device)
            .filter(Filter::Nearest)
            .address_mode(SamplerAddressMode::ClampToEdge)
            .build()
            .unwrap();

        Self {
            pipeline,
            vertex_shader,
            fragment_shader,
            nearest_sampler,
            object_texture,
            light_texture,
            sound_texture,
            effect_texture,
            entity_texture,
            particle_texture,
            tile_textures,
            step_textures,
        }
    }

    pub fn recreate_pipeline(&mut self, device: Arc<Device>, subpass: Subpass, viewport: Viewport) {
        self.pipeline = Self::create_pipeline(device, subpass, viewport, &self.vertex_shader, &self.fragment_shader);
    }

    fn create_pipeline(device: Arc<Device>, subpass: Subpass, viewport: Viewport, vertex_shader: &ShaderModule, fragment_shader: &ShaderModule) -> Arc<GraphicsPipeline> {
        GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<ScreenVertex>())
            .vertex_shader(vertex_shader.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant(iter::once(viewport)))
            .fragment_shader(fragment_shader.entry_point("main").unwrap(), ())
            .render_pass(subpass)
            .build(device)
            .unwrap()
    }

    pub fn render_buffers(&self, builder: &mut CommandBuilder, camera: &dyn Camera, diffuse_buffer: ImageBuffer, normal_buffer: ImageBuffer, water_buffer: ImageBuffer, depth_buffer: ImageBuffer, picker_buffer: ImageBuffer, vertex_buffer: ScreenVertexBuffer, render_settings: &RenderSettings) {

        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.descriptor_set_layouts().get(0).unwrap().clone();

        let set = PersistentDescriptorSet::new(descriptor_layout, [
            WriteDescriptorSet::image_view(0, diffuse_buffer),
            WriteDescriptorSet::image_view(1, normal_buffer),
            WriteDescriptorSet::image_view(2, water_buffer),
            WriteDescriptorSet::image_view(3, depth_buffer),
            WriteDescriptorSet::image_view_sampler(4, picker_buffer, self.nearest_sampler.clone()),
        ]).unwrap(); 

        let constants = Constants {
            screen_to_world_matrix: camera.get_screen_to_world_matrix().into(),
            show_diffuse_buffer: render_settings.show_diffuse_buffer as u32,
            show_normal_buffer: render_settings.show_normal_buffer as u32,
            show_water_buffer: render_settings.show_water_buffer as u32,
            show_depth_buffer: render_settings.show_depth_buffer as u32,
            show_picker_buffer: render_settings.show_picker_buffer as u32,
        };

        builder
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 0, set)
            .push_constants(layout, 0, constants)
            .bind_vertex_buffers(0, vertex_buffer)
            .draw(3, 1, 0, 0).unwrap();
    }
}
