use cgmath::Vector2;

use graphics::{ Renderer, Color };
use interface::StateProvider;

pub struct TextComponent {
    offset: Vector2<f32>,
    display: String,
    color: Color,
    font_size: f32,
}

impl TextComponent {

    pub fn new(offset: Vector2<f32>, display: String, color: Color, font_size: f32) -> Self {
        return Self { offset, display, color, font_size };
    }

    pub fn render(&self, renderer: &mut Renderer, position: Vector2<f32>) {
        renderer.render_text(&self.display, position + self.offset, self.color, self.font_size);
    }
}

pub struct DynamicTextComponent {
    offset: Vector2<f32>,
    formatter: Box<dyn Fn(&StateProvider) -> String>,
    color: Color,
    font_size: f32,
}

impl DynamicTextComponent {

    pub fn new(offset: Vector2<f32>, formatter: Box<dyn Fn(&StateProvider) -> String>, color: Color, font_size: f32) -> Self {
        return Self { offset, formatter, color, font_size };
    }

    pub fn render(&self, renderer: &mut Renderer, state_provider: &StateProvider, position: Vector2<f32>) {
        let text = (self.formatter)(state_provider);
        renderer.render_text(&text, position + self.offset, self.color, self.font_size);
    }
}
