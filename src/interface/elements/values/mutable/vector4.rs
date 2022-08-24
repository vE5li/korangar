use std::cmp::PartialOrd;
use std::fmt::Display;

use cgmath::Vector4;
use derive_new::new;
use num::traits::NumOps;
use num::{NumCast, Zero};

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::interface::{Element, Vector4Window, *};

#[derive(new)]
pub struct MutableVector4Value<T: Zero + NumOps + NumCast + Copy + PartialOrd + Display + 'static> {
    name: String,
    inner_pointer: *const Vector4<T>,
    minimum_value: Vector4<T>,
    maximum_value: Vector4<T>,
    change_event: Option<ChangeEvent>,
    #[new(value = "Vector4::new(T::zero(), T::zero(), T::zero(), T::zero())")]
    cached_inner: Vector4<T>,
    #[new(default)]
    cached_values: String,
    #[new(value = "Size::zero()")]
    cached_size: Size,
    #[new(value = "Position::zero()")]
    cached_position: Position,
}

impl<T: Zero + NumOps + NumCast + Copy + PartialOrd + Display + 'static> Element for MutableVector4Value<T> {

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &Theme) {

        let (size, position) = placement_resolver.allocate(&theme.value.size_constraint);
        self.cached_size = size.finalize();
        self.cached_position = position;
    }

    fn update(&mut self) -> Option<ChangeEvent> {

        let current_value = unsafe { *self.inner_pointer };

        // TODO: fix behavior in case the initial value is 0.0 for all components
        if self.cached_inner != current_value {

            self.cached_inner = current_value;
            self.cached_values = format!(
                "{:.1}, {:.1}, {:.1}, {:.1}",
                self.cached_inner.x, self.cached_inner.y, self.cached_inner.z, self.cached_inner.w
            );
            return Some(ChangeEvent::RerenderWindow);
        }

        None
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

        Some(ClickAction::OpenWindow(Box::new(Vector4Window::new(
            self.name.clone(),
            self.inner_pointer,
            self.minimum_value,
            self.maximum_value,
            self.change_event,
        ))))
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
                *theme.value.border_radius * *interface_settings.scaling,
                *theme.value.hovered_background_color,
            ),

            false => renderer.render_rectangle(
                render_target,
                absolute_position,
                self.cached_size,
                clip_size,
                *theme.value.border_radius * *interface_settings.scaling,
                *theme.value.background_color,
            ),
        }

        renderer.render_text(
            render_target,
            &self.cached_values,
            absolute_position + *theme.value.text_offset * *interface_settings.scaling,
            clip_size,
            *theme.value.foreground_color,
            *theme.value.font_size * *interface_settings.scaling,
        );
    }
}
