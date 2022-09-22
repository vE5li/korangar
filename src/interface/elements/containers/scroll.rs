use std::rc::Weak;

use cgmath::Zero;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::interface::{Element, *};

const SCROLL_SPEED: f32 = 0.8;

pub struct ScrollView {
    scroll: f32,
    state: ContainerState,
    size_constraint: SizeConstraint,
}

impl ScrollView {

    pub fn new(elements: Vec<ElementCell>, size_constraint: SizeConstraint) -> Self {

        let scroll = 0.0;
        let state = ContainerState::new(elements);

        Self {
            scroll,
            state,
            size_constraint,
        }
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

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, interface_settings: &InterfaceSettings, theme: &Theme) {

        self.state.resolve(
            placement_resolver,
            interface_settings,
            theme,
            &self.size_constraint,
            Vector2::zero(),
        );
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        self.state.update()
    }

    fn hovered_element(&self, mouse_position: Position) -> HoverInformation {
        self.state.hovered_element::<true>(mouse_position + Vector2::new(0.0, self.scroll))
    }

    fn scroll(&mut self, delta: f32) -> Option<ChangeEvent> {

        self.scroll -= delta * SCROLL_SPEED;
        self.scroll = self.scroll.max(0.0);
        Some(ChangeEvent::RerenderWindow)
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
        second_theme: bool,
    ) {

        let mut renderer = self
            .state
            .state
            .element_renderer(render_target, renderer, interface_settings, parent_position, clip_size);

        renderer.set_scroll(self.scroll);

        self.state.render(
            &mut renderer,
            state_provider,
            interface_settings,
            theme,
            hovered_element,
            focused_element,
            second_theme,
        );
    }
}
