use managers::{ ModelManager, TextureManager };
use graphics::{ Renderer, VertexBuffer, Texture, Camera, Transform };

pub struct Object {
    vertex_buffer: VertexBuffer,
    texture: Texture,
    transform: Transform,
}

impl Object {

    pub fn new(model_manager: &mut ModelManager, texture_manager: &mut TextureManager, name: String) -> Self {
        let model_path = format!("/home/korangar/models/{}.obj", name);
        let texture_path = format!("/home/korangar/textures/{}.png", name);

        return Self {
            vertex_buffer: model_manager.get(model_path),
            texture: texture_manager.get(texture_path),
            transform: Transform::new(),
        }
    }

    pub fn render(&self, renderer: &mut Renderer, camera: &Camera) {
        renderer.draw_textured(camera, self.vertex_buffer.clone(), self.texture.clone(), &self.transform);
    }
}
