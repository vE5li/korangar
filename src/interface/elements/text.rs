use graphics::Color;

use super::super::*;

pub fn text(window_builder: &mut WindowBuilder, display: String, text_color: Color, font_size: f32) -> Element {

    let element_index = window_builder.unique_identifier();
    let text_offset = Vector2::new(5.0, 0.0);
    let size = text_offset + Vector2::new(display.len() as f32 * (font_size / 2.0), font_size);

    let position = window_builder.position(size);

    let text = Component::Text(TextComponent::new(text_offset, display, text_color, font_size));

    let components = vec![text];

    return Element::new(components, element_index, position);
}

