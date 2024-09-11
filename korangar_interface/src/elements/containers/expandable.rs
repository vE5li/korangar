use std::cell::RefCell;
use std::rc::Weak;

use super::ContainerState;
use crate::application::{
    Application, InterfaceRenderer, MouseInputModeTrait, PartialSizeTrait, PartialSizeTraitExt, PositionTrait, PositionTraitExt,
    ScalingTrait, SizeTrait, SizeTraitExt,
};
use crate::elements::{Element, ElementCell, ElementState, Focus};
use crate::event::{ChangeEvent, ClickAction, HoverInformation};
use crate::layout::{Dimension, PlacementResolver, SizeBound};
use crate::theme::{ExpandableTheme, InterfaceTheme};

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

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver<App>, application: &App, theme: &App::Theme) {
        let closed_size = self
            .closed_size_bound
            .resolve_element::<App::PartialSize>(
                placement_resolver.get_available(),
                placement_resolver.get_remaining(),
                &placement_resolver.get_parent_limits(),
                application.get_scaling(),
            )
            .finalize::<App::Size>();

        let size_bound = match self.expanded && !self.state.elements.is_empty() {
            true => &self.open_size_bound,
            false => &self.closed_size_bound,
        };

        let screen_position = App::Position::only_top(closed_size.height())
            .combined(theme.expandable().element_offset())
            .scaled(application.get_scaling());

        let (mut inner_placement_resolver, mut size, position) =
            placement_resolver.derive(size_bound, screen_position, theme.expandable().border_size());
        let parent_limits = inner_placement_resolver.get_parent_limits();

        if self.expanded && !self.state.elements.is_empty() {
            inner_placement_resolver.set_gaps(theme.expandable().gaps());

            self.state
                .elements
                .iter_mut()
                .for_each(|element| element.borrow_mut().resolve(&mut inner_placement_resolver, application, theme));

            if self.open_size_bound.height.is_flexible() {
                let final_height = inner_placement_resolver.final_height()
                    + closed_size.height()
                    + theme.expandable().element_offset().top() * application.get_scaling().get_factor();

                let final_height = self.open_size_bound.validated_height(
                    final_height,
                    placement_resolver.get_available().height(),
                    placement_resolver.get_available().height(),
                    &parent_limits,
                    application.get_scaling(),
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

    fn left_click(&mut self, force_update: &mut bool) -> Vec<ClickAction<App>> {
        self.expanded = !self.expanded;
        *force_update = true;
        Vec::new()
    }

    fn render(
        &self,
        render_target: &mut <App::Renderer as InterfaceRenderer<App>>::Target,
        render_pass: &mut App::RenderPass<'_>,
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
        let mut renderer =
            self.state
                .state
                .element_renderer(render_target, render_pass, renderer, application, parent_position, screen_clip);

        let background_color = match second_theme {
            true => theme.expandable().second_background_color(),
            false => theme.expandable().background_color(),
        };

        renderer.render_background(theme.expandable().corner_radius(), background_color);

        renderer.render_expand_arrow(
            theme.expandable().icon_offset(),
            theme.expandable().icon_size(),
            theme.expandable().foreground_color(),
            self.expanded,
        );

        let foreground_color = match self.is_element_self(hovered_element) || self.is_element_self(focused_element) {
            true => theme.expandable().hovered_foreground_color(),
            false => theme.expandable().foreground_color(),
        };

        renderer.render_text(
            &self.display,
            theme.expandable().text_offset(),
            foreground_color,
            theme.expandable().font_size(),
        );

        if self.expanded && !self.state.elements.is_empty() {
            self.state.render(
                &mut renderer,
                application,
                theme,
                hovered_element,
                focused_element,
                mouse_mode,
                !second_theme,
            );
        }
    }
}
