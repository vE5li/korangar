use cgmath::Array;
use procedural::*;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::interface::{Element, *};
use crate::network::ChatMessage;

pub struct Chat {
    // TODO: make this TrackedState
    messages: Rc<RefCell<Vec<ChatMessage>>>,
    cached_message_count: usize,
    state: ElementState,
}

impl Chat {
    pub fn new(messages: Rc<RefCell<Vec<ChatMessage>>>) -> Self {
        let cached_message_count = messages.borrow().len();
        let state = ElementState::default();

        Self {
            messages,
            cached_message_count,
            state,
        }
    }
}

impl Element for Chat {
    fn get_state(&self) -> &ElementState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &Theme) {
        let mut size_constraint = constraint!(100%, 0);
        size_constraint.height = Dimension::Absolute(self.messages.borrow().len() as f32 * *theme.chat.font_size);

        self.state.resolve(placement_resolver, &size_constraint);
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        let messages = self.messages.borrow();

        if messages.len() != self.cached_message_count {
            self.cached_message_count = messages.len();
            return Some(ChangeEvent::Reresolve); // TODO: reresolve window would be preferred
        }

        None
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        _state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        theme: &Theme,
        parent_position: Position,
        clip_size: ClipSize,
        _hovered_element: Option<&dyn Element>,
        _focused_element: Option<&dyn Element>,
        _mouse_mode: &MouseInputMode,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, interface_settings, parent_position, clip_size);

        for (message_index, message) in self.messages.borrow().iter().enumerate() {
            renderer.render_text(
                &message.text,
                Vector2::new(0.0, message_index as f32 * *theme.chat.font_size) + Vector2::from_value(0.2),
                Color::monochrome(0),
                *theme.chat.font_size,
            );

            renderer.render_text(
                &message.text,
                Vector2::new(0.0, message_index as f32 * *theme.chat.font_size),
                message.color,
                *theme.chat.font_size,
            );
        }
    }
}
