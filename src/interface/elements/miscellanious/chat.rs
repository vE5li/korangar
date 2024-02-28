use procedural::*;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::interface::{Element, *};
use crate::loaders::FontLoader;
use crate::network::ChatMessage;

pub struct Chat {
    // TODO: make this Remote
    messages: Rc<RefCell<Vec<ChatMessage>>>,
    font_loader: Rc<RefCell<FontLoader>>,
    // TODO: make this Remote
    stamp: bool,
    cached_message_count: usize,
    state: ElementState,
}

impl Chat {
    pub fn new(messages: Rc<RefCell<Vec<ChatMessage>>>, font_loader: Rc<RefCell<FontLoader>>) -> Self {
        let cached_message_count = messages.borrow().len();
        let state = ElementState::default();

        Self {
            messages,
            font_loader,
            stamp: true,
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

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, interface_settings: &InterfaceSettings, theme: &InterfaceTheme) {
        let mut size_constraint = constraint!(100%, 0);
        // Not sure why but 0.0 cuts off the lower part of the text, so add some
        // padding.
        let mut height = 5.0 * *interface_settings.scaling;

        for message in self.messages.borrow().iter() {
            height += self
                .font_loader
                .borrow()
                .get_text_dimensions(
                    message.stamped_text(self.stamp),
                    *theme.chat.font_size * *interface_settings.scaling,
                    placement_resolver.get_available().width,
                )
                .y;
        }

        size_constraint.height = Dimension::Absolute(height);
        self.state.resolve(placement_resolver, &size_constraint);
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        let messages = self.messages.borrow();

        if messages.len() != self.cached_message_count {
            self.cached_message_count = messages.len();
            return Some(ChangeEvent::RESOLVE_WINDOW);
        }

        None
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        _state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        theme: &InterfaceTheme,
        parent_position: ScreenPosition,
        screen_clip: ScreenClip,
        _hovered_element: Option<&dyn Element>,
        _focused_element: Option<&dyn Element>,
        _mouse_mode: &MouseInputMode,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, interface_settings, parent_position, screen_clip);

        let mut offset = 0.0;

        for message in self.messages.borrow().iter() {
            let text = message.stamped_text(self.stamp);

            renderer.render_text(
                text,
                ScreenPosition {
                    left: 0.2,
                    top: offset + 0.2,
                },
                Color::monochrome(0),
                *theme.chat.font_size,
            );

            offset += renderer.render_text(text, ScreenPosition::only_top(offset), message.color, *theme.chat.font_size);
        }
    }
}
