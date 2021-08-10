use managers::{ ModelManager, TextureManager };
use graphics::{ Renderer, VertexBuffer, Texture, Camera, Transform };

pub struct Object {
    vertex_buffer: VertexBuffer,
    texture: Texture,
    bump_map: Texture,
    specular_map: Texture,
    transform: Transform,
}

impl Object {

    pub fn new(model_manager: &mut ModelManager, texture_manager: &mut TextureManager, name: String) -> Self {
        let model_path = format!("/home/korangar/models/{}.obj", name);
        let texture_path = format!("/home/korangar/textures/{}.png", name);
        let bump_map_path = format!("/home/korangar/textures/{}_bump.png", name);
        let specular_map_path = format!("/home/korangar/textures/{}_specular.png", name);

        return Self {
            vertex_buffer: model_manager.get(model_path),
            texture: texture_manager.get(texture_path),
            bump_map: texture_manager.get(bump_map_path),
            specular_map: texture_manager.get(specular_map_path),
            transform: Transform::new(),
        }
    }

    pub fn render(&self, renderer: &mut Renderer, camera: &Camera) {
        renderer.draw_textured(camera, self.vertex_buffer.clone(), self.texture.clone(), self.bump_map.clone(), self.specular_map.clone(), &self.transform);
    }
}
