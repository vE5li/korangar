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
use vulkano::pipeline::{ GraphicsPipeline, PipelineBindPoint };
use vulkano::pipeline::viewport::Viewport;
use vulkano::descriptor_set::PersistentDescriptorSet;
use vulkano::render_pass::Subpass;
use vulkano::sync::GpuFuture;

use graphics::*;
use loaders::TextureLoader;

use self::vertex_shader::Shader as VertexShader;
use self::fragment_shader::Shader as FragmentShader;
use self::fragment_shader::ty::Constants;

pub struct DebugRenderer {
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: VertexShader,
    fragment_shader: FragmentShader,
    pub object_texture: Texture,
    pub light_texture: Texture,
    pub sound_texture: Texture,
    pub effect_texture: Texture,
    pub particle_texture: Texture,
    pub tile_textures: Vec<Texture>,
    pub step_textures: Vec<Texture>,
}

impl DebugRenderer {

    pub fn new(device: Arc<Device>, subpass: Subpass, viewport: Viewport, texture_loader: &mut TextureLoader, texture_future: &mut Box<dyn GpuFuture + 'static>) -> Self {

        let vertex_shader = VertexShader::load(device.clone()).unwrap();
        let fragment_shader = FragmentShader::load(device.clone()).unwrap();
        let pipeline = Self::create_pipeline(device, subpass, viewport, &vertex_shader, &fragment_shader);

        let object_texture = texture_loader.get(String::from("assets/object.png"), texture_future);
        let light_texture = texture_loader.get(String::from("assets/light.png"), texture_future);
        let sound_texture = texture_loader.get(String::from("assets/sound.png"), texture_future);
        let effect_texture = texture_loader.get(String::from("assets/effect.png"), texture_future);
        let particle_texture = texture_loader.get(String::from("assets/particle.png"), texture_future);
        let tile_textures = (0..7_i32).map(|index| texture_loader.get(format!("assets/{}.png", index), texture_future)).collect();
        let step_textures = ["goal", "straight", "diagonal"].iter().map(|index| texture_loader.get(format!("assets/{}.png", index), texture_future)).collect();

        return Self {
            pipeline,
            vertex_shader,
            fragment_shader,
            object_texture,
            light_texture,
            sound_texture,
            effect_texture,
            particle_texture,
            tile_textures,
            step_textures,
        };
    }

    pub fn recreate_pipeline(&mut self, device: Arc<Device>, subpass: Subpass, viewport: Viewport) {
        self.pipeline = Self::create_pipeline(device, subpass, viewport, &self.vertex_shader, &self.fragment_shader);
    }

    fn create_pipeline(device: Arc<Device>, subpass: Subpass, viewport: Viewport, vertex_shader: &VertexShader, fragment_shader: &FragmentShader) -> Arc<GraphicsPipeline> {

        let pipeline = GraphicsPipeline::start()
            .vertex_input_single_buffer::<ScreenVertex>()
            .vertex_shader(vertex_shader.main_entry_point(), ())
            .triangle_list()
            .viewports_dynamic_scissors_irrelevant(1)
            .viewports(iter::once(viewport))
            .fragment_shader(fragment_shader.main_entry_point(), ())
            .render_pass(subpass)
            .build(device)
            .unwrap();

        return Arc::new(pipeline);
    }

    pub fn render_buffers(&self, builder: &mut CommandBuilder, camera: &dyn Camera, diffuse_buffer: ImageBuffer, normal_buffer: ImageBuffer, depth_buffer: ImageBuffer, vertex_buffer: ScreenVertexBuffer, render_settings: &RenderSettings) {

        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.descriptor_set_layouts().get(0).unwrap().clone();

        let mut set_builder = PersistentDescriptorSet::start(descriptor_layout);

        set_builder
            .add_image(diffuse_buffer).unwrap()
            .add_image(normal_buffer).unwrap()
            .add_image(depth_buffer).unwrap();

        let set = Arc::new(set_builder.build().unwrap());

        let constants = Constants {
            screen_to_world_matrix: camera.get_screen_to_world_matrix().into(),
            show_diffuse_buffer: render_settings.show_diffuse_buffer as u32,
            show_normal_buffer: render_settings.show_normal_buffer as u32,
            show_depth_buffer: render_settings.show_depth_buffer as u32,
        };

        builder
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 0, set)
            .push_constants(layout, 0, constants)
            .bind_vertex_buffers(0, vertex_buffer)
            .draw(3, 1, 0, 0).unwrap();
    }
}
