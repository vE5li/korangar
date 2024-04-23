mod builder;

use std::cell::RefCell;
use std::rc::Rc;

use korangar_interface::application::{Application, FontSizeTraitExt};
use korangar_interface::elements::{Element, ElementState};
use korangar_interface::event::ChangeEvent;
use korangar_interface::layout::{Dimension, PlacementResolver};
use korangar_interface::size_bound;
use korangar_interface::state::{PlainRemote, Remote};

pub use self::builder::ChatBuilder;
use crate::graphics::{Color, InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{ScreenClip, ScreenPosition};
use crate::interface::theme::InterfaceTheme;
use crate::interface::windows::ChatMessage;
use crate::loaders::FontLoader;

pub struct Chat {
    messages: PlainRemote<Vec<ChatMessage>>,
    font_loader: Rc<RefCell<FontLoader>>,
    state: ElementState<InterfaceSettings>,
}

impl Element<InterfaceSettings> for Chat {
    fn get_state(&self) -> &ElementState<InterfaceSettings> {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<InterfaceSettings> {
        &mut self.state
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn resolve(
        &mut self,
        placement_resolver: &mut PlacementResolver<InterfaceSettings>,
        application: &InterfaceSettings,
        theme: &InterfaceTheme,
    ) {
        let mut size_bound = size_bound!(100%, 0);
        // Not sure why but 0.0 cuts off the lower part of the text, so add some
        // padding.
        let mut height = 5.0 * application.get_scaling_factor();

        // Dividing by the scaling is done to counteract the scaling being applied
        // twice per message. It's not the cleanest solution but it works.
        for message in self.messages.get().iter() {
            height += self
                .font_loader
                .borrow()
                .get_text_dimensions(
                    &message.text,
                    theme.chat.font_size.get().scaled(application.get_scaling()),
                    placement_resolver.get_available().width,
                )
                .height
                / application.get_scaling_factor();
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
        application: &InterfaceSettings,
        theme: &InterfaceTheme,
        parent_position: ScreenPosition,
        screen_clip: ScreenClip,
        _hovered_element: Option<&dyn Element<InterfaceSettings>>,
        _focused_element: Option<&dyn Element<InterfaceSettings>>,
        _mouse_mode: &MouseInputMode,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, application, parent_position, screen_clip);

        let mut offset = 0.0;

        for message in self.messages.get().iter() {
            let text = &message.text;

            renderer.render_text(
                text,
                ScreenPosition {
                    left: 0.2,
                    top: offset + 0.2,
                },
                Color::monochrome_u8(0),
                theme.chat.font_size.get(),
            );

            let message_color = match message.color {
                korangar_networking::MessageColor::Rgb { red, green, blue } => Color::rgb_u8(red, green, blue),
                korangar_networking::MessageColor::Broadcast => theme.chat.broadcast_color.get(),
                korangar_networking::MessageColor::Server => theme.chat.server_color.get(),
                korangar_networking::MessageColor::Error => theme.chat.broadcast_color.get(),
                korangar_networking::MessageColor::Information => theme.chat.information_color.get(),
            };

            // Dividing by the scaling is done to counteract the scaling being applied
            // twice per message. It's not the cleanest solution but it works.
            offset += renderer.render_text(
                text,
                ScreenPosition::only_top(offset),
                message_color,
                theme.chat.font_size.get(),
            ) / application.get_scaling_factor();
        }
    }
}
