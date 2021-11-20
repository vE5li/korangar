use cgmath::Vector2;

use graphics::Color;

use super::super::*;

pub fn basic_information_window(window_builder: &mut WindowBuilder, interface_state: &mut InterfaceState) -> Element {

    let position = Vector2::new(20.0, 20.0);
    let health_point_color = Color::new(25, 150, 25);
    let spell_point_color = Color::new(25, 25, 150);
    let activity_point_color = Color::new(150, 150, 25);

    window_builder.new_row_spaced(2.0);
    let name_text = text(window_builder, String::from("vE5li"), Color::new(120, 120, 120), 14.0);
    let job_text = text(window_builder, String::from("Arch Mage"), Color::new(50, 150, 50), 12.0);

    window_builder.new_row_spaced(6.0);
    let health_point_text = text(window_builder, String::from("HP"), Color::new(120, 120, 120), 14.0);
    let health_point_bar = bar(window_builder, health_point_color, StateKey::PlayerMaximumHealthPoints, StateKey::PlayerCurrentHealthPoints, window_builder.remaining_width());

    let spell_point_text = text(window_builder, String::from("SP"), Color::new(120, 120, 120), 14.0);
    let spell_point_bar = bar(window_builder, spell_point_color, StateKey::PlayerMaximumSpellPoints, StateKey::PlayerCurrentSpellPoints, window_builder.remaining_width());

    let activity_point_text = text(window_builder, String::from("AP"), Color::new(120, 120, 120), 14.0);
    let activity_point_bar = bar(window_builder, activity_point_color, StateKey::PlayerMaximumActivityPoints, StateKey::PlayerCurrentActivityPoints, window_builder.remaining_width());
    window_builder.new_row_spaced(6.0);

    let elements = vec![name_text, job_text, health_point_text, health_point_bar, spell_point_text, spell_point_bar, activity_point_text, activity_point_bar];
    return window_builder.framed_window(interface_state, "basic information", elements, position);
}
