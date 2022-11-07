use procedural::dimension;

use super::{ElementEvent, ElementText};
use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::{MouseInputMode, UserEvent};
use crate::interface::{Element, *};

// TODO: move this
pub type Selector = Box<dyn Fn() -> bool>;
pub type ColorSelector = Box<dyn Fn(&Theme) -> Color>;

#[derive(Default)]
pub struct Button {
    text: Option<ElementText>,
    event: Option<ElementEvent>,
    disabled_selector: Option<Selector>,
    foreground_color: Option<ColorSelector>,
    background_color: Option<ColorSelector>,
    width_constraint: Option<DimensionConstraint>,
    state: ElementState,
}

impl Button {
    pub fn with_static_text(mut self, text: &'static str) -> Self {
        self.text = Some(ElementText::Static(text));
        self
    }

    pub fn with_dynamic_text(mut self, text: String) -> Self {
        self.text = Some(ElementText::Dynamic(text));
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

    pub fn with_disabled_selector(mut self, disabled_selector: impl Fn() -> bool + 'static) -> Self {
        self.disabled_selector = Some(Box::new(disabled_selector));
        self
    }

    pub fn with_foreground_color(mut self, foreground_color: impl Fn(&Theme) -> Color + 'static) -> Self {
        self.foreground_color = Some(Box::new(foreground_color));
        self
    }

    pub fn with_background_color(mut self, background_color: impl Fn(&Theme) -> Color + 'static) -> Self {
        self.background_color = Some(Box::new(background_color));
        self
    }

    pub fn with_width(mut self, width_constraint: DimensionConstraint) -> Self {
        self.width_constraint = Some(width_constraint);
        self
    }

    pub fn wrap(self) -> ElementCell {
        Rc::new(RefCell::new(self))
    }

    fn is_disabled(&self) -> bool {
        self.disabled_selector.as_ref().map(|selector| !selector()).unwrap_or(false)
    }
}

impl Element for Button {
    fn get_state(&self) -> &ElementState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state
    }

    fn is_focusable(&self) -> bool {
        !self.is_disabled()
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
        if self.is_disabled() {
            return None;
        }

        self.event.as_mut().and_then(ElementEvent::execute)
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
        hovered_element: Option<&dyn Element>,
        focused_element: Option<&dyn Element>,
        _mouse_mode: &MouseInputMode,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, interface_settings, parent_position, clip_size);

        let disabled = self.is_disabled();
        let background_color = match self.is_element_self(hovered_element) || self.is_element_self(focused_element) {
            _ if disabled => *theme.button.disabled_background_color,
            true => *theme.button.hovered_background_color,
            false if self.background_color.is_some() => (self.background_color.as_ref().unwrap())(theme),
            false => *theme.button.background_color,
        };

        renderer.render_background(*theme.button.border_radius, background_color);

        if let Some(text) = &self.text {
            let foreground_color = if disabled {
                *theme.button.disabled_foreground_color
            } else {
                self.foreground_color
                    .as_ref()
                    .map(|closure| closure(theme))
                    .unwrap_or(*theme.button.foreground_color)
            };

            renderer.render_text(
                text.get_str(),
                *theme.button.text_offset,
                foreground_color,
                *theme.button.font_size,
            );
        }
    }
}
