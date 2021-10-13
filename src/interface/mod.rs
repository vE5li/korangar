mod state;
mod components;
mod elements;
mod windows;

use cgmath::Vector2;

use graphics::Renderer;

use self::state::{ InterfaceState, StateKey };
use self::components::*;
use self::elements::*;
use self::windows::{ WindowBuilder, render_settings_window };

pub use self::state::StateProvider;

pub struct Interface {
    interface_state: InterfaceState,
    render_settings_window: Element,
}

impl Interface {

    pub fn new() -> Self {

        let mut interface_state = InterfaceState::new();
        let mut window_builder = WindowBuilder::new(200.0);

        let render_settings_window = render_settings_window(&mut window_builder, &mut interface_state);

        return Self { interface_state, render_settings_window };
    }

    pub fn hovered_element(&self, mouse_position: Vector2<f32>) -> Option<&Element> {
        return self.render_settings_window.hovered_element(&self.interface_state, mouse_position);
    }

    pub fn move_hovered(&mut self, index: usize, offset: Vector2<f32>) {
        self.interface_state.move_offset(index, offset);
    }

    pub fn render(&self, renderer: &mut Renderer, state_provider: &StateProvider, focused_index: usize) {
        self.render_settings_window.render(renderer, &self.interface_state, state_provider, Vector2::new(0.0, 0.0), focused_index);
    }
}
