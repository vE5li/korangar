use graphics::Color;

use super::super::*;

const HEIGHT: f32 = 12.0;
const SIDE_OFFSET: f32 = 10.0;

pub fn bar(window_builder: &mut WindowBuilder, stretch_color: Color, maximum_value_key: StateKey, current_value_key: StateKey, width: f32) -> Element {

    let element_index = window_builder.unique_identifier();
    let size = Vector2::new(width, HEIGHT);
    let background_offset = Vector2::new(SIDE_OFFSET, 1.0);
    let background_size = Vector2::new(width - SIDE_OFFSET * 2.0, HEIGHT);
    let background_corner_radius = Vector4::new(1.0, 1.0, 1.0, 1.0);
    let background_color = Color::new(60, 60, 60);
    let text_offset = Vector2::new(50.0, 0.0);
    let text_color = Color::new(180, 180, 180);

    let maximum_value_key_clone = maximum_value_key.clone();
    let current_value_key_clone = current_value_key.clone();
    let formatter = move |state_provider: &StateProvider| -> String {
        let current = state_provider.get(&current_value_key_clone).to_number();
        let maximum = state_provider.get(&maximum_value_key_clone).to_number();
        return format!("{}/{}", current, maximum);
    };

    let position = window_builder.position(size);

    let background = Component::Rectangle(RectangleComponent::new(background_offset, background_size, background_corner_radius, background_color, background_color));
    let stretch = Component::Stretch(StretchComponent::new(background_offset, background_size, background_corner_radius, stretch_color, maximum_value_key, current_value_key));
    let text = Component::DynamicText(DynamicTextComponent::new(text_offset, Box::new(formatter), text_color, HEIGHT));

    let components = vec![background, stretch, text];

    return Element::new(components, element_index, position);
}
