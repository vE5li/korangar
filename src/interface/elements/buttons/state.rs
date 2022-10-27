use procedural::dimension;

use super::{ElementEvent, ElementText};
use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::{MouseInputMode, UserEvent};
use crate::interface::{Element, *};

#[derive(Default)]
pub struct StateButton {
    text: Option<ElementText>,
    selector: Option<Box<dyn Fn(&StateProvider) -> bool>>,
    event: Option<ElementEvent>,
    width_constraint: Option<DimensionConstraint>,
    transparent_background: bool,
    state: ElementState,
}

impl StateButton {
    pub fn with_static_text(mut self, text: &'static str) -> Self {
        self.text = Some(ElementText::Static(text));
        self
    }

    pub fn with_dynamic_text(mut self, text: String) -> Self {
        self.text = Some(ElementText::Dynamic(text));
        self
    }

    pub fn with_selector(mut self, selector: impl Fn(&StateProvider) -> bool + 'static) -> Self {
        self.selector = Some(Box::new(selector));
        self
    }

    pub fn with_event(mut self, event: UserEvent) -> Self {
        self.event = Some(ElementEvent::Event(event));
        self
    }

    pub fn with_action_closure(mut self, event_closure: impl Fn() -> Option<ClickAction> + 'static) -> Self {
        self.event = Some(ElementEvent::ActionClosure(Box::new(event_closure)));
        self
    }

    pub fn with_closure(mut self, closure: impl FnMut() + 'static) -> Self {
        self.event = Some(ElementEvent::Closure(Box::new(closure)));
        self
    }

    pub fn with_transparent_background(mut self) -> Self {
        self.transparent_background = true;
        self
    }

    pub fn with_width(mut self, width_constraint: DimensionConstraint) -> Self {
        self.width_constraint = Some(width_constraint);
        self
    }

    pub fn wrap(self) -> ElementCell {
        assert!(self.selector.is_some(), "state buttons need a selector");
        Rc::new(RefCell::new(self))
    }
}

impl Element for StateButton {
    fn get_state(&self) -> &ElementState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &Theme) {
        let size_constraint = self
            .width_constraint
            .as_ref()
            .unwrap_or(&dimension!(100%))
            .add_height(theme.button.height_constraint);

        self.state.resolve(placement_resolver, &size_constraint);
    }

    fn hovered_element(&self, mouse_position: Position, mouse_mode: &MouseInputMode) -> HoverInformation {
        match mouse_mode {
            MouseInputMode::None => self.state.hovered_element(mouse_position),
            _ => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, _force_update: &mut bool) -> Option<ClickAction> {
        self.event.as_mut().and_then(ElementEvent::execute)
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        theme: &Theme,
        parent_position: Position,
        clip_size: ClipSize,
        hovered_element: Option<&dyn Element>,
        focused_element: Option<&dyn Element>,
        _mouse_mode: &MouseInputMode,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, interface_settings, parent_position, clip_size);

        let highlighted = self.is_element_self(hovered_element) || self.is_element_self(focused_element);

        if !self.transparent_background {
            let background_color = match highlighted {
                true => *theme.button.hovered_background_color,
                false => *theme.button.background_color,
            };

            renderer.render_background(*theme.button.border_radius, background_color);
        }

        let foreground_color = match self.transparent_background && highlighted {
            true => *theme.button.hovered_foreground_color,
            false => *theme.button.foreground_color,
        };

        renderer.render_checkbox(
            *theme.button.icon_offset,
            *theme.button.icon_size,
            foreground_color,
            (self.selector.as_ref().unwrap())(state_provider),
        );

        if let Some(text) = &self.text {
            renderer.render_text(
                text.get_str(),
                *theme.button.icon_text_offset,
                foreground_color,
                *theme.button.font_size,
            );
        }
    }
}
