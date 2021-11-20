use input::UserEvent;
use graphics::Color;

use super::super::*;

pub fn checkbutton(window_builder: &mut WindowBuilder, display: String, event: UserEvent, state_key: StateKey, width: f32) -> Element {

    let element_index = window_builder.unique_identifier();
    let size = Vector2::new(width, 21.0);
    let background_corner_radius = Vector4::new(1.0, 1.0, 1.0, 1.0);
    let background_color = Color::new(30, 30, 30);
    let focused_background_color = Color::new(40, 40, 40);
    let text_offset = Vector2::new(22.0, 2.0);
    let text_color = Color::new(150, 150, 150);
    let font_size = 14.0;

    let position = window_builder.position(size);

    let checkbox_offset = Vector2::new(8.0, 5.0);
    let checkbox_size = Vector2::new(9.0, 9.0);
    let checkbox_color = Color::new(150, 150, 150);

    let background = Component::Rectangle(RectangleComponent::new(Vector2::new(0.0, 0.0), size, background_corner_radius, background_color, focused_background_color));
    let checkbox = Component::Checkbox(CheckboxComponent::new(checkbox_offset, checkbox_size, checkbox_color, state_key));
    let text = Component::Text(TextComponent::new(text_offset, display, text_color, font_size));
    let hoverable = Component::Hoverable(HoverableComponent::new(size));
    let clickable = Component::Clickable(ClickableComponent::new(event));

    let components = vec![background, text, checkbox, hoverable, clickable];

    return Element::new(components, element_index, position);
}
