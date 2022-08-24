use std::sync::Arc;

use cgmath::Vector2;
use vulkano::image::ImageAccess;
use vulkano::sync::GpuFuture;

use crate::graphics::{Color, DeferredRenderer, Renderer};
use crate::loaders::{ActionLoader, Actions, Sprite, SpriteLoader};

pub struct MouseCursor {
    sprite: Arc<Sprite>,
    actions: Arc<Actions>,
}

impl MouseCursor {

    pub fn new(
        sprite_loader: &mut SpriteLoader,
        action_loader: &mut ActionLoader,
        texture_future: &mut Box<dyn GpuFuture + 'static>,
    ) -> Self {

        let sprite = sprite_loader.get("cursors.spr", texture_future).unwrap();
        let actions = action_loader.get("cursors.act").unwrap();

        Self { sprite, actions }
    }

    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        renderer: &DeferredRenderer,
        mouse_position: Vector2<f32>,
        color: Color,
    ) {

        let dimesions = self.sprite.textures[0]
            .image()
            .dimensions()
            .width_height()
            .map(|dimension| dimension as f32);

        renderer.render_sprite(
            render_target,
            self.sprite.textures[0].clone(),
            mouse_position,
            Vector2::from(dimesions),
            color,
        );
    }
}
