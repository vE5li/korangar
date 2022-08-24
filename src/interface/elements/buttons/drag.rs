use cgmath::Vector2;
use derive_new::new;
use num::Zero;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::interface::{Element, *};

#[derive(new)]
pub struct DragButton {
    window_title: String,
    #[new(value = "Size::zero()")]
    cached_size: Size,
    #[new(value = "Position::zero()")]
    cached_position: Position,
}

impl Element for DragButton {

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &Theme) {

        let (size, position) = placement_resolver.allocate(&theme.window.title_size_constraint);
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
        Some(ClickAction::MoveInterface)
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
        let clip_size = Vector2::new(f32::min(clip_size.x, absolute_position.x + self.cached_size.x), clip_size.y);

        if matches!(hovered_element, Some(reference) if std::ptr::eq(reference as *const _ as *const (), self as *const _ as *const ())) {

            renderer.render_rectangle(
                render_target,
                absolute_position,
                self.cached_size,
                clip_size,
                *theme.window.title_border_radius * *interface_settings.scaling,
                *theme.window.title_background_color,
            );
        }

        renderer.render_text(
            render_target,
            &self.window_title,
            absolute_position + *theme.window.text_offset * *interface_settings.scaling,
            clip_size,
            *theme.window.foreground_color,
            *theme.window.font_size * *interface_settings.scaling,
        );
    }
}
