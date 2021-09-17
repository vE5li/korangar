use std::sync::Arc;

use vulkano::device::Device;
use vulkano::sync::GpuFuture;

use managers::{ ModelManager, TextureManager };
use graphics::{ Renderer, VertexBuffer, Texture, Camera, Transform };

pub struct Object {
    vertex_buffer: VertexBuffer,
    texture: Texture,
    normal_map: Texture,
    specular_map: Texture,
    transform: Transform,
}

impl Object {

    pub fn new(model_manager: &mut ModelManager, texture_manager: &mut TextureManager, name: String) -> (Self, Box<dyn GpuFuture + 'static>) {
        let model_path = format!("/home/korangar/models/{}.obj", name);
        let texture_path = format!("/home/korangar/textures/{}.png", name);
        let normal_map_path = format!("/home/korangar/textures/{}_bump.png", name);
        let specular_map_path = format!("/home/korangar/textures/{}_specular.png", name);

        let vertex_buffer = model_manager.get(model_path);
        let (texture, texture_future) = texture_manager.get(texture_path);
        let (normal_map, normal_map_future) = texture_manager.get(normal_map_path);
        let (specular_map, specular_map_future) = texture_manager.get(specular_map_path);

        let future = texture_future.join(normal_map_future).join(specular_map_future).boxed();

        return (Self {
            vertex_buffer: vertex_buffer,
            texture: texture,
            normal_map: normal_map,
            specular_map: specular_map,
            transform: Transform::new(),
        }, future);
    }

    //pub fn render(&self, renderer: &mut Renderer, camera: &Camera) {
    //  renderer.draw_textured(camera, self.vertex_buffer.clone(), self.texture.clone(), self.normal_map.clone(), self.specular_map.clone(), &self.transform);
    //}
}
