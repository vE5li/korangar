use std::cmp::PartialOrd;

use cgmath::{Array, Vector4};
use derive_new::new;
use num::traits::NumOps;
use num::{clamp, NumCast, Zero};

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::{Element, *};

#[derive(new)]
pub struct Slider<T: Zero + NumOps + NumCast + Copy + PartialOrd> {
    value_pointer: *const T,
    minimum_value: T,
    maximum_value: T,
    change_event: Option<ChangeEvent>,
    #[new(value = "T::zero()")]
    cached_value: T,
    #[new(default)]
    state: ElementState,
}

impl<T: Zero + NumOps + NumCast + Copy + PartialOrd> Element for Slider<T> {
    fn get_state(&self) -> &ElementState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &InterfaceTheme) {
        self.state.resolve(placement_resolver, &theme.slider.size_constraint);
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        let current_value = unsafe { *self.value_pointer };

        if self.cached_value != current_value {
            self.cached_value = current_value;
            return Some(ChangeEvent::RENDER_WINDOW);
        }

        None
    }

    fn hovered_element(&self, mouse_position: Position, mouse_mode: &MouseInputMode) -> HoverInformation {
        match mouse_mode {
            MouseInputMode::None => self.state.hovered_element(mouse_position),
            MouseInputMode::DragElement((element, _)) if self.is_element_self(Some(&*element.borrow())) => HoverInformation::Hovered,
            _ => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, _force_update: &mut bool) -> Vec<ClickAction> {
        vec![ClickAction::DragElement]
    }

    fn drag(&mut self, mouse_delta: Position) -> Option<ChangeEvent> {
        let total_range = self.maximum_value.to_f32().unwrap() - self.minimum_value.to_f32().unwrap();
        let raw_value = self.cached_value.to_f32().unwrap() + (mouse_delta.x * total_range * 0.005);
        let new_value = clamp(
            raw_value,
            self.minimum_value.to_f32().unwrap(),
            self.maximum_value.to_f32().unwrap(),
        );

        unsafe {
            std::ptr::write(self.value_pointer as *mut T, T::from(new_value).unwrap());
        }
        self.change_event
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        _state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        theme: &InterfaceTheme,
        parent_position: Position,
        clip_size: ClipSize,
        hovered_element: Option<&dyn Element>,
        _focused_element: Option<&dyn Element>,
        _mouse_mode: &MouseInputMode,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, interface_settings, parent_position, clip_size);

        if self.is_element_self(hovered_element) {
            renderer.render_background(*theme.button.border_radius, *theme.slider.background_color);
        }

        let bar_size = Size::new(self.state.cached_size.x * 0.9, self.state.cached_size.y / 4.0);
        let offset = (self.state.cached_size - bar_size) / 2.0;

        renderer.render_rectangle(offset, bar_size, Vector4::from_value(0.5), *theme.slider.rail_color);

        let knob_size = Size::new(20.0 * *interface_settings.scaling, self.state.cached_size.y * 0.8);
        let total_range = self.maximum_value - self.minimum_value;
        let offset = Position::new(
            (self.state.cached_size.x - knob_size.x) / total_range.to_f32().unwrap()
                * (self.cached_value.to_f32().unwrap() - self.minimum_value.to_f32().unwrap()),
            (self.state.cached_size.y - knob_size.y) / 2.0,
        );

        renderer.render_rectangle(offset, knob_size, Vector4::from_value(4.0), *theme.slider.knob_color);
    }
}
