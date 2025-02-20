use std::cmp::PartialOrd;

use num::traits::NumOps;
use num::{NumCast, Zero, clamp};

use crate::application::{
    Application, CornerRadiusTraitExt, MouseInputModeTrait, PositionTrait, PositionTraitExt, ScalingTrait, SizeTrait, SizeTraitExt,
};
use crate::elements::{Element, ElementState};
use crate::event::{ChangeEvent, ClickAction, HoverInformation};
use crate::layout::PlacementResolver;
use crate::theme::{ButtonTheme, InterfaceTheme, SliderTheme};

pub struct Slider<App, Value>
where
    App: Application,
    Value: Zero + NumOps + NumCast + Copy + PartialOrd + 'static,
{
    reference: &'static Value,
    minimum_value: Value,
    maximum_value: Value,
    change_event: Option<ChangeEvent>,
    cached_value: Value,
    state: ElementState<App>,
}

impl<App, Value> Slider<App, Value>
where
    App: Application,
    Value: Zero + NumOps + NumCast + Copy + PartialOrd + 'static,
{
    pub fn new(reference: &'static Value, minimum_value: Value, maximum_value: Value, change_event: Option<ChangeEvent>) -> Self {
        Self {
            reference,
            minimum_value,
            maximum_value,
            change_event,
            cached_value: Value::zero(),
            state: Default::default(),
        }
    }
}

impl<App, Value> Element<App> for Slider<App, Value>
where
    App: Application,
    Value: Zero + NumOps + NumCast + Copy + PartialOrd + 'static,
{
    fn get_state(&self) -> &ElementState<App> {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<App> {
        &mut self.state
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver<App>, _application: &App, theme: &App::Theme) {
        self.state.resolve(placement_resolver, &theme.slider().size_bound());
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        let current_value = *self.reference;

        if self.cached_value != current_value {
            self.cached_value = current_value;
            return Some(ChangeEvent::RENDER_WINDOW);
        }

        None
    }

    fn hovered_element(&self, mouse_position: App::Position, mouse_mode: &App::MouseInputMode) -> HoverInformation<App> {
        if mouse_mode.is_none() {
            self.state.hovered_element(mouse_position)
        } else if mouse_mode.is_self_dragged(self) {
            HoverInformation::Hovered
        } else {
            HoverInformation::Missed
        }
    }

    fn left_click(&mut self, _force_update: &mut bool) -> Vec<ClickAction<App>> {
        vec![ClickAction::DragElement]
    }

    fn drag(&mut self, mouse_delta: App::Position) -> Option<ChangeEvent> {
        let total_range = self.maximum_value.to_f32().unwrap() - self.minimum_value.to_f32().unwrap();
        let raw_value = self.cached_value.to_f32().unwrap() + (mouse_delta.left() * total_range * 0.005);
        let new_value = clamp(
            raw_value,
            self.minimum_value.to_f32().unwrap(),
            self.maximum_value.to_f32().unwrap(),
        );

        // SAFETY: Obviously this is totally unsafe, but considering this is a debug
        // tool I think it's acceptable.
        unsafe {
            #[allow(invalid_reference_casting)]
            std::ptr::write(self.reference as *const Value as *mut Value, Value::from(new_value).unwrap());
        }
        self.change_event
    }

    fn render(
        &self,
        renderer: &App::Renderer,
        application: &App,
        theme: &App::Theme,
        parent_position: App::Position,
        screen_clip: App::Clip,
        hovered_element: Option<&dyn Element<App>>,
        _focused_element: Option<&dyn Element<App>>,
        _mouse_mode: &App::MouseInputMode,
        _second_theme: bool,
    ) {
        let mut renderer = self.state.element_renderer(renderer, application, parent_position, screen_clip);

        if self.is_element_self(hovered_element) {
            renderer.render_background(theme.button().corner_radius(), theme.slider().background_color());
        }

        let bar_size = App::Size::new(self.state.cached_size.width() * 0.9, self.state.cached_size.height() / 4.0);
        let offset = App::Position::from_size((self.state.cached_size.shrink(bar_size)).halved());

        renderer.render_rectangle(offset, bar_size, App::CornerRadius::uniform(0.5), theme.slider().rail_color());

        let knob_size = App::Size::new(
            20.0 * application.get_scaling().get_factor(),
            self.state.cached_size.height() * 0.8,
        );
        let total_range = self.maximum_value - self.minimum_value;
        let offset = App::Position::new(
            (self.state.cached_size.width() - knob_size.width()) / total_range.to_f32().unwrap()
                * (self.cached_value.to_f32().unwrap() - self.minimum_value.to_f32().unwrap()),
            (self.state.cached_size.height() - knob_size.height()) / 2.0,
        );

        renderer.render_rectangle(offset, knob_size, App::CornerRadius::uniform(4.0), theme.slider().knob_color());
    }
}
