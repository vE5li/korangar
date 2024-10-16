use std::cell::RefCell;
use std::rc::Weak;

use super::ContainerState;
use crate::application::{Application, MouseInputModeTrait, PositionTrait, PositionTraitExt, SizeTrait, SizeTraitExt};
use crate::elements::{Element, ElementCell, ElementState, Focus};
use crate::event::{ChangeEvent, HoverInformation};
use crate::layout::{PlacementResolver, SizeBound};
use crate::theme::{ButtonTheme, InterfaceTheme};
use crate::ColorSelector;

const SCROLL_SPEED: f32 = 0.8;

pub struct ScrollView<App>
where
    App: Application,
{
    scroll: f32,
    children_height: f32,
    state: ContainerState<App>,
    size_bound: SizeBound,
    background_color: Option<ColorSelector<App>>,
}

impl<App> ScrollView<App>
where
    App: Application,
{
    pub fn new(elements: Vec<ElementCell<App>>, size_bound: SizeBound) -> Self {
        let scroll = 0.0;
        let children_height = 0.0;
        let state = ContainerState::new(elements);
        let background_color = None;

        Self {
            scroll,
            children_height,
            state,
            size_bound,
            background_color,
        }
    }

    pub fn with_background_color(mut self, background_color: impl Fn(&App::Theme) -> App::Color + 'static) -> Self {
        self.background_color = Some(Box::new(background_color));
        self
    }

    fn clamp_scroll(&mut self) {
        self.scroll = self
            .scroll
            .clamp(0.0, (self.children_height - self.state.state.cached_size.height()).max(0.0));
    }
}

impl<App> Element<App> for ScrollView<App>
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
        self.children_height = self
            .state
            .resolve(placement_resolver, application, theme, &self.size_bound, App::Size::zero());
        self.clamp_scroll();
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        self.state.update()
    }

    fn hovered_element(&self, mouse_position: App::Position, mouse_mode: &App::MouseInputMode) -> HoverInformation<App> {
        let absolute_position = mouse_position.relative_to(self.state.state.cached_position);

        if absolute_position.left() >= 0.0
            && absolute_position.top() >= 0.0
            && absolute_position.left() <= self.state.state.cached_size.width()
            && absolute_position.top() <= self.state.state.cached_size.height()
        {
            for element in &self.state.elements {
                match element
                    .borrow()
                    .hovered_element(absolute_position.combined(App::Position::only_top(self.scroll)), mouse_mode)
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
            .element_renderer(renderer, application, parent_position, screen_clip);

        if let Some(color_selector) = &self.background_color {
            renderer.render_background(theme.button().corner_radius(), color_selector(theme));
        }

        renderer.set_scroll(self.scroll);

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
