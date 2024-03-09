use std::rc::Weak;

use procedural::size_bound;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::{Element, *};

pub struct Container {
    size_bound: Option<SizeBound>,
    border_size: Option<ScreenSize>,
    state: ContainerState,
}

impl Container {
    pub fn new(elements: Vec<ElementCell>) -> Self {
        Self {
            state: ContainerState::new(elements),
            border_size: None,
            size_bound: None,
        }
    }

    pub fn with_size(mut self, size_bound: SizeBound) -> Self {
        self.size_bound = Some(size_bound);
        self
    }
}

impl Element for Container {
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
        let size_bound = self.size_bound.as_ref().unwrap_or(&size_bound!(100%, ?));
        let border = self.border_size.unwrap_or_default();

        self.state
            .resolve(placement_resolver, interface_settings, theme, size_bound, border);
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        self.state.update()
    }

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation {
        self.state.hovered_element(mouse_position, mouse_mode, false)
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
