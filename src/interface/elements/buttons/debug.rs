use cgmath::Vector2;
use derive_new::new;
use num::Zero;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::UserEvent;
use crate::interface::{Element, *};

#[derive(new)]
pub struct DebugButton {
    text: &'static str,
    event: UserEvent,
    menu_button: bool,
    #[new(value = "Size::zero()")]
    cached_size: Size,
    #[new(value = "Position::zero()")]
    cached_position: Position,
}

impl Element for DebugButton {

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &Theme) {

        let size_constraint = match self.menu_button {
            true => &theme.button.menu_size_constraint,
            false => &theme.button.size_constraint,
        };

        let (size, position) = placement_resolver.allocate(size_constraint);
        self.cached_size = size.finalize();
        self.cached_position = position;
    }

    fn hovered_element(&self, mouse_position: Position) -> HoverInformation {

        let absolute_position = mouse_position - self.cached_position;

        if absolute_position.x >= 0.0
            && absolute_position.y >= 0.0
            && absolute_position.x <= self.cached_size.x
            && absolute_position.y <= self.cached_size.y
        {
            return HoverInformation::Hovered;
        }

        HoverInformation::Missed
    }

    fn left_click(&mut self, _force_update: &mut bool) -> Option<ClickAction> {
        Some(ClickAction::Event(self.event.clone()))
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        _state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        theme: &Theme,
        parent_position: Position,
        clip_size: Size,
        hovered_element: Option<&dyn Element>,
        _focused_element: Option<&dyn Element>,
        _second_theme: bool,
    ) {

        let absolute_position = parent_position + self.cached_position;
        let clip_size = clip_size.zip(absolute_position + self.cached_size, f32::min);

        match matches!(hovered_element, Some(reference) if std::ptr::eq(reference as *const _ as *const (), self as *const _ as *const ()))
        {

            true => renderer.render_rectangle(
                render_target,
                absolute_position,
                self.cached_size,
                clip_size,
                *theme.button.border_radius * *interface_settings.scaling,
                *theme.button.hovered_background_color,
            ),

            false => renderer.render_rectangle(
                render_target,
                absolute_position,
                self.cached_size,
                clip_size,
                *theme.button.border_radius * *interface_settings.scaling,
                *theme.button.background_color,
            ),
        }

        let offset = Vector2::new(
            0.0,
            (self.cached_size.y - *theme.button.font_size * *interface_settings.scaling) / 2.0,
        );
        renderer.render_debug_icon(
            render_target,
            absolute_position + offset + *theme.button.icon_offset * *interface_settings.scaling,
            *theme.button.icon_size * *interface_settings.scaling,
            clip_size,
            *theme.button.debug_foreground_color,
        );
        renderer.render_text(
            render_target,
            self.text,
            absolute_position + offset + *theme.button.icon_text_offset * *interface_settings.scaling,
            clip_size,
            *theme.button.debug_foreground_color,
            *theme.button.font_size * *interface_settings.scaling,
        );
    }
}
