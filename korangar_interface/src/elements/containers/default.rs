use std::cell::RefCell;
use std::rc::Weak;

use super::ContainerState;
use crate::application::{Application, InterfaceRenderer, SizeTraitExt};
use crate::elements::{Element, ElementCell, ElementState, Focus};
use crate::event::{ChangeEvent, HoverInformation};
use crate::layout::{Dimension, PlacementResolver, SizeBound};

pub struct Container<App>
where
    App: Application,
{
    size_bound: Option<SizeBound>,
    border_size: Option<App::Size>,
    state: ContainerState<App>,
}

impl<App> Container<App>
where
    App: Application,
{
    pub fn new(elements: Vec<ElementCell<App>>) -> Self {
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

impl<App> Element<App> for Container<App>
where
    App: Application,
{
    fn get_state(&self) -> &ElementState<App> {
        &self.state.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<App> {
        &mut self.state.state
    }

    fn link_back(&mut self, weak_self: Weak<RefCell<dyn Element<App>>>, weak_parent: Option<Weak<RefCell<dyn Element<App>>>>) {
        self.state.link_back(weak_self, weak_parent);
    }

    fn is_focusable(&self) -> bool {
        self.state.is_focusable::<false>()
    }

    fn focus_next(&self, self_cell: ElementCell<App>, caller_cell: Option<ElementCell<App>>, focus: Focus) -> Option<ElementCell<App>> {
        self.state.focus_next::<false>(self_cell, caller_cell, focus)
    }

    fn restore_focus(&self, self_cell: ElementCell<App>) -> Option<ElementCell<App>> {
        self.state.restore_focus(self_cell)
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver<App>, application: &App, theme: &App::Theme) {
        let default_size_bound = SizeBound::only_height(Dimension::Flexible);
        let size_bound = self.size_bound.as_ref().unwrap_or(&default_size_bound);
        let border = self.border_size.unwrap_or(App::Size::zero());

        self.state.resolve(placement_resolver, application, theme, size_bound, border);
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        self.state.update()
    }

    fn hovered_element(&self, mouse_position: App::Position, mouse_mode: &App::MouseInputMode) -> HoverInformation<App> {
        self.state.hovered_element(mouse_position, mouse_mode, false)
    }

    fn render(
        &self,
        render_target: &mut <App::Renderer as InterfaceRenderer<App>>::Target,
        renderer: &App::Renderer,
        application: &App,
        theme: &App::Theme,
        parent_position: App::Position,
        screen_clip: App::Clip,
        hovered_element: Option<&dyn Element<App>>,
        focused_element: Option<&dyn Element<App>>,
        mouse_mode: &App::MouseInputMode,
        second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .state
            .element_renderer(render_target, renderer, application, parent_position, screen_clip);

        self.state.render(
            &mut renderer,
            application,
            theme,
            hovered_element,
            focused_element,
            mouse_mode,
            second_theme,
        );
    }
}
