mod state;
mod components;
mod elements;
mod windows;

use cgmath::{ Vector4, Vector2 };

use graphics::Renderer;

use self::state::{ InterfaceState, StateKey };
use self::components::*;
use self::elements::*;
use self::windows::{ WindowBuilder, basic_information_window, render_settings_window };

pub use self::state::StateProvider;

pub struct Interface {
    interface_state: InterfaceState,
    basic_information_window: Element,
    render_settings_window: Element,
}

impl Interface {

    pub fn new() -> Self {

        let mut interface_state = InterfaceState::new();
        let mut window_builder = WindowBuilder::new(200.0);

        let basic_information_window = basic_information_window(&mut window_builder, &mut interface_state);

        window_builder.reset();

        let render_settings_window = render_settings_window(&mut window_builder, &mut interface_state);

        return Self { interface_state, basic_information_window, render_settings_window };
    }

    pub fn hovered_element(&self, mouse_position: Vector2<f32>) -> Option<&Element> {

        if let Some(element) = self.basic_information_window.hovered_element(&self.interface_state, mouse_position) {
            return Some(element);
        }

        if let Some(element) = self.render_settings_window.hovered_element(&self.interface_state, mouse_position) {
            return Some(element);
        }

        return None;
    }

    pub fn move_hovered(&mut self, index: usize, offset: Vector2<f32>) {
        self.interface_state.move_offset(index, offset);
    }

    pub fn render(&self, renderer: &mut Renderer, state_provider: &StateProvider, focused_index: usize) {
        self.basic_information_window.render(renderer, &self.interface_state, state_provider, Vector2::new(0.0, 0.0), focused_index);
        self.render_settings_window.render(renderer, &self.interface_state, state_provider, Vector2::new(0.0, 0.0), focused_index);
    }
}
