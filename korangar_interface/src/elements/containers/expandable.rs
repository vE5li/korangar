use std::cell::RefCell;
use std::rc::Weak;

use rust_state::{Context, RustState, SafeUnwrap, Selector, View};

use super::ContainerState;
use crate::application::{
    Application, InterfaceRenderer, MouseInputModeTrait, PartialSizeTrait, PartialSizeTraitExt, PositionTrait, PositionTraitExt,
    ScalingTrait, SizeTrait, SizeTraitExt,
};
use crate::elements::{Element, ElementCell, ElementState, Focus};
use crate::event::{ChangeEvent, ClickAction, HoverInformation};
use crate::layout::{Dimension, PlacementResolver, SizeBound};
use crate::theme::ExpandableTheme;

pub struct Expandable<App>
where
    App: Application,
{
    display: String,
    expanded: bool,
    open_size_bound: SizeBound,
    closed_size_bound: SizeBound,
    cached_closed_size: App::Size,
    state: ContainerState<App>,
}

impl<App> Expandable<App>
where
    App: Application,
{
    pub fn new(display: String, elements: Vec<ElementCell<App>>, expanded: bool) -> Self {
        let state = ContainerState::new(elements);

        Self {
            display,
            expanded,
            open_size_bound: SizeBound::only_height(Dimension::Flexible),
            closed_size_bound: SizeBound::only_height(Dimension::Absolute(18.0)),
            cached_closed_size: App::Size::zero(),
            state,
        }
    }
}

impl<App> Element<App> for Expandable<App>
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
        self.state.is_focusable::<true>()
    }

    fn focus_next(&self, self_cell: ElementCell<App>, caller_cell: Option<ElementCell<App>>, focus: Focus) -> Option<ElementCell<App>> {
        // TODO: fix collapsed elements being focusable
        self.state.focus_next::<true>(self_cell, caller_cell, focus)
    }

    fn restore_focus(&self, self_cell: ElementCell<App>) -> Option<ElementCell<App>> {
        self.state.restore_focus(self_cell)
    }

    fn resolve(&mut self, state: &View<App>, theme_selector: App::ThemeSelector, placement_resolver: &mut PlacementResolver<App>) {
        let closed_size = self
            .closed_size_bound
            .resolve_element::<App::PartialSize>(
                placement_resolver.get_available(),
                placement_resolver.get_remaining(),
                &placement_resolver.get_parent_limits(),
                *state.get_safe(&App::ScaleSelector::default()),
            )
            .finalize::<App::Size>();

        let size_bound = match self.expanded && !self.state.elements.is_empty() {
            true => &self.open_size_bound,
            false => &self.closed_size_bound,
        };

        let element_offset = state.get_safe(&ExpandableTheme::element_offset(theme_selector));
        let screen_position = App::Position::only_top(closed_size.height())
            .combined(*element_offset)
            .scaled(*state.get_safe(&App::ScaleSelector::default()));

        let border_size = state.get_safe(&ExpandableTheme::border_size(theme_selector));
        let (mut inner_placement_resolver, mut size, position) = placement_resolver.derive(size_bound, screen_position, *border_size);
        let parent_limits = inner_placement_resolver.get_parent_limits();

        if self.expanded && !self.state.elements.is_empty() {
            let gaps = state.get_safe(&ExpandableTheme::gaps(theme_selector));
            inner_placement_resolver.set_gaps(*gaps);

            self.state
                .elements
                .iter_mut()
                .for_each(|element| element.borrow_mut().resolve(state, theme_selector, &mut inner_placement_resolver));

            if self.open_size_bound.height.is_flexible() {
                let final_height = inner_placement_resolver.final_height()
                    + closed_size.height()
                    + element_offset.top() * state.get_safe(&App::ScaleSelector::default()).get_factor();

                let final_height = self.open_size_bound.validated_height(
                    final_height,
                    placement_resolver.get_available().height(),
                    placement_resolver.get_available().height(),
                    &parent_limits,
                    *state.get_safe(&App::ScaleSelector::default()),
                );

                size = App::PartialSize::new(size.width(), Some(final_height));
                placement_resolver.register_height(final_height);
            }
        }

        self.cached_closed_size = closed_size;
        self.state.state.cached_size = size.finalize();
        self.state.state.cached_position = position;
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        if !self.expanded || self.state.elements.is_empty() {
            return None;
        }

        self.state.update()
    }

    fn hovered_element(&self, mouse_position: App::Position, mouse_mode: &App::MouseInputMode) -> HoverInformation<App> {
        let absolute_position = mouse_position.relative_to(self.state.state.cached_position);

        if absolute_position.left() >= 0.0
            && absolute_position.top() >= 0.0
            && absolute_position.left() <= self.state.state.cached_size.width()
            && absolute_position.top() <= self.state.state.cached_size.height()
        {
            if self.expanded && !self.state.elements.is_empty() {
                for element in &self.state.elements {
                    match element.borrow().hovered_element(absolute_position, mouse_mode) {
                        HoverInformation::Hovered => return HoverInformation::Element(element.clone()),
                        HoverInformation::Missed => {}
                        hover_information => return hover_information,
                    }
                }
            }

            if mouse_mode.is_none() {
                return HoverInformation::Hovered;
            }
        }

        HoverInformation::Missed
    }

    fn left_click(&mut self, _state: &Context<App>, force_update: &mut bool) -> Vec<ClickAction<App>> {
        self.expanded = !self.expanded;
        *force_update = true;
        Vec::new()
    }

    fn render(
        &self,
        render_target: &mut <App::Renderer as InterfaceRenderer<App>>::Target,
        renderer: &App::Renderer,
        state: &View<App>,
        theme_selector: App::ThemeSelector,
        parent_position: App::Position,
        screen_clip: App::Clip,
        second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .state
            .element_renderer(render_target, renderer, state, parent_position, screen_clip);

        let corner_radius = state.get_safe(&ExpandableTheme::corner_radius(theme_selector));
        let background_color = match second_theme {
            true => state.get_safe(&ExpandableTheme::second_background_color(theme_selector)),
            false => state.get_safe(&ExpandableTheme::background_color(theme_selector)),
        };

        renderer.render_background(*corner_radius, *background_color);

        let icon_offset = state.get_safe(&ExpandableTheme::icon_offset(theme_selector));
        let icon_size = state.get_safe(&ExpandableTheme::icon_size(theme_selector));
        let foreground_color = state.get_safe(&ExpandableTheme::foreground_color(theme_selector));

        renderer.render_expand_arrow(*icon_offset, *icon_size, *foreground_color, self.expanded);

        let hovered_element = state.get_safe(&App::HoveredElementSelector::default());
        let focused_element = state.get_safe(&App::FocusedElementSelector::default());
        let highlighted = self.is_cell_self(&hovered_element) || self.is_cell_self(&focused_element);

        let foreground_color = match highlighted {
            true => state.get_safe(&ExpandableTheme::hovered_foreground_color(theme_selector)),
            false => state.get_safe(&ExpandableTheme::foreground_color(theme_selector)),
        };

        let text_offset = state.get_safe(&ExpandableTheme::text_offset(theme_selector));
        let font_size = state.get_safe(&ExpandableTheme::font_size(theme_selector));

        renderer.render_text(&self.display, *text_offset, *foreground_color, *font_size);

        if self.expanded && !self.state.elements.is_empty() {
            self.state.render(&mut renderer, state, theme_selector, !second_theme);
        }
    }
}
