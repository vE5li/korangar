use std::rc::Weak;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::{Element, *};

const SCROLL_SPEED: f32 = 0.8;

pub struct ScrollView {
    scroll: f32,
    children_height: f32,
    state: ContainerState,
    size_constraint: SizeConstraint,
    background_color: Option<ColorSelector>,
}

impl ScrollView {
    pub fn new(elements: Vec<ElementCell>, size_constraint: SizeConstraint) -> Self {
        let scroll = 0.0;
        let children_height = 0.0;
        let state = ContainerState::new(elements);
        let background_color = None;

        Self {
            scroll,
            children_height,
            state,
            size_constraint,
            background_color,
        }
    }

    pub fn with_background_color(mut self, background_color: impl Fn(&InterfaceTheme) -> Color + 'static) -> Self {
        self.background_color = Some(Box::new(background_color));
        self
    }

    fn clamp_scroll(&mut self) {
        self.scroll = self
            .scroll
            .clamp(0.0, (self.children_height - self.state.state.cached_size.height).max(0.0));
    }
}

impl Element for ScrollView {
    fn get_state(&self) -> &ElementState {
        &self.state.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state.state
    }

    fn link_back(&mut self, weak_self: Weak<RefCell<dyn Element>>, weak_parent: Option<Weak<RefCell<dyn Element>>>) {
        self.state.link_back(weak_self, weak_parent);
    }

    fn is_focusable(&self) -> bool {
        self.state.is_focusable::<false>()
    }

    fn focus_next(&self, self_cell: ElementCell, caller_cell: Option<ElementCell>, focus: Focus) -> Option<ElementCell> {
        self.state.focus_next::<false>(self_cell, caller_cell, focus)
    }

    fn restore_focus(&self, self_cell: ElementCell) -> Option<ElementCell> {
        self.state.restore_focus(self_cell)
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, interface_settings: &InterfaceSettings, theme: &InterfaceTheme) {
        self.children_height = self.state.resolve(
            placement_resolver,
            interface_settings,
            theme,
            &self.size_constraint,
            ScreenSize::default(),
        );
        self.clamp_scroll();
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        self.state.update()
    }

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation {
        let absolute_position = ScreenPosition::from_size(mouse_position - self.state.state.cached_position);

        if absolute_position.left >= 0.0
            && absolute_position.top >= 0.0
            && absolute_position.left <= self.state.state.cached_size.width
            && absolute_position.top <= self.state.state.cached_size.height
        {
            for element in &self.state.elements {
                match element
                    .borrow()
                    .hovered_element(absolute_position + ScreenPosition::only_top(self.scroll), mouse_mode)
                {
                    HoverInformation::Hovered => return HoverInformation::Element(element.clone()),
                    HoverInformation::Missed => {}
                    hover_information => return hover_information,
                }
            }

            if mouse_mode.is_none() {
                return HoverInformation::Hovered;
            }
        }

        HoverInformation::Missed
    }

    fn scroll(&mut self, delta: f32) -> Option<ChangeEvent> {
        self.scroll -= delta * SCROLL_SPEED;
        self.clamp_scroll();
        Some(ChangeEvent::RENDER_WINDOW)
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        theme: &InterfaceTheme,
        parent_position: ScreenPosition,
        screen_clip: ScreenClip,
        hovered_element: Option<&dyn Element>,
        focused_element: Option<&dyn Element>,
        mouse_mode: &MouseInputMode,
        second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .state
            .element_renderer(render_target, renderer, interface_settings, parent_position, screen_clip);

        if let Some(color_selector) = &self.background_color {
            renderer.render_background(theme.button.corner_radius.get(), color_selector(theme));
        }

        renderer.set_scroll(self.scroll);

        self.state.render(
            &mut renderer,
            state_provider,
            interface_settings,
            theme,
            hovered_element,
            focused_element,
            mouse_mode,
            second_theme,
        );
    }
}
