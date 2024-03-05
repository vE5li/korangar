mod builder;

use procedural::*;

pub use self::builder::ChatBuilder;
use crate::graphics::{InterfaceRenderer, Renderer};
use crate::interface::{Element, *};
use crate::loaders::FontLoader;
use crate::network::ChatMessage;

pub struct Chat {
    messages: Remote<Vec<ChatMessage>>,
    font_loader: Rc<RefCell<FontLoader>>,
    // TODO: make this Remote
    stamp: bool,
    state: ElementState,
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
        let mut size_bound = size_bound!(100%, 0);
        // Not sure why but 0.0 cuts off the lower part of the text, so add some
        // padding.
        let mut height = 5.0 * interface_settings.scaling.get();

        // NOTE: Dividing by the scaling is done to counteract the scaling being applied
        // twice per message. It's not the cleanest solution but it works.
        for message in self.messages.borrow().iter() {
            height += self
                .font_loader
                .borrow()
                .get_text_dimensions(
                    message.stamped_text(self.stamp),
                    theme.chat.font_size.get() * interface_settings.scaling.get(),
                    placement_resolver.get_available().width,
                )
                .y
                / interface_settings.scaling.get();
        }

        size_bound.height = Dimension::Absolute(height);
        self.state.resolve(placement_resolver, &size_bound);
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        self.messages.consume_changed().then_some(ChangeEvent::RESOLVE_WINDOW)
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
                Color::monochrome_u8(0),
                theme.chat.font_size.get(),
            );

            // NOTE: Dividing by the scaling is done to counteract the scaling being applied
            // twice per message. It's not the cleanest solution but it works.
            offset += renderer.render_text(
                text,
                ScreenPosition::only_top(offset),
                message.color,
                theme.chat.font_size.get(),
            ) / interface_settings.scaling.get();
        }
    }
}
