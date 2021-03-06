use cgmath::Vector2;
use std::sync::Arc;
use vulkano::sync::GpuFuture;

use crate::graphics::{ DeferredRenderer, Renderer, Color };
use crate::loaders::{ Sprite, Actions, SpriteLoader, ActionLoader };

pub struct MouseCursor {
    sprite: Arc<Sprite>,
    actions: Arc<Actions>,
}

impl MouseCursor {

    pub fn new(sprite_loader: &mut SpriteLoader, action_loader: &mut ActionLoader, texture_future: &mut Box<dyn GpuFuture + 'static>) -> Self {

        let sprite = sprite_loader.get("cursors.spr", texture_future).unwrap();
        let actions = action_loader.get("cursors.act").unwrap();

        Self {
            sprite,
            actions,
        }
    }

    pub fn render(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, renderer: &DeferredRenderer, mouse_position: Vector2<f32>, color: Color) {
        renderer.render_sprite(render_target, self.sprite.textures[0].clone(), mouse_position, vector2!(30.0), color);
    }
}
