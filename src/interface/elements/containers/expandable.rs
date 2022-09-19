use std::rc::Weak;

use num::Zero;
use procedural::*;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::interface::{Element, *};

pub struct Expandable {
    display: String,
    expanded: bool,
    open_size_constraint: SizeConstraint,
    closed_size_constraint: SizeConstraint,
    cached_closed_size: Size,
    state: ContainerState,
}

impl Expandable {

    pub fn new(display: String, elements: Vec<ElementCell>, expanded: bool) -> Self {

        let state = ContainerState::new(elements);

        Self {
            display,
            expanded,
            open_size_constraint: constraint!(100%, ?),
            closed_size_constraint: constraint!(100%, 18),
            cached_closed_size: Size::zero(),
            state,
        }
    }
}

impl Element for Expandable {

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
        self.state.is_focusable::<true>()
    }

    fn focus_next(&self, self_cell: ElementCell, caller_cell: Option<ElementCell>, focus: Focus) -> Option<ElementCell> {
        // TODO: fix collapsed elements being focusable
        self.state.focus_next::<true>(self_cell, caller_cell, focus)
    }

    fn restore_focus(&self, self_cell: ElementCell) -> Option<ElementCell> {
        self.state.restore_focus(self_cell)
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, interface_settings: &InterfaceSettings, theme: &Theme) {

        let closed_size = self
            .closed_size_constraint
            .resolve_partial(
                placement_resolver.get_avalible(),
                placement_resolver.get_remaining(),
                *interface_settings.scaling,
            )
            .finalize();

        let (mut size, position) = match self.expanded {
            true => placement_resolver.allocate(&self.open_size_constraint),
            false => placement_resolver.allocate(&self.closed_size_constraint),
        };

        if self.expanded {

            let mut inner_placement_resolver = placement_resolver.derive(
                Position::new(0.0, closed_size.y) + *theme.expandable.element_offset * *interface_settings.scaling,
                *theme.expandable.border_size,
            );
            inner_placement_resolver.set_gaps(*theme.expandable.gaps);

            self.state.elements.iter_mut().for_each(|element| {

                element
                    .borrow_mut()
                    .resolve(&mut inner_placement_resolver, interface_settings, theme)
            });

            if self.open_size_constraint.height.is_flexible() {

                let final_height = inner_placement_resolver.final_height()
                    + closed_size.y
                    + theme.expandable.element_offset.y * *interface_settings.scaling
                    + theme.expandable.border_size.y * *interface_settings.scaling * 2.0;
                let final_height = self.open_size_constraint.validated_height(
                    final_height,
                    placement_resolver.get_avalible().y,
                    placement_resolver.get_avalible().y,
                    *interface_settings.scaling,
                );
                size.y = Some(final_height);
                placement_resolver.register_height(final_height);
            }
        }

        self.cached_closed_size = closed_size;
        self.state.state.cached_size = size.finalize();
        self.state.state.cached_position = position;
    }

    fn update(&mut self) -> Option<ChangeEvent> {

        if !self.expanded {
            return None;
        }

        self.state.update()
    }

    fn hovered_element(&self, mouse_position: Position) -> HoverInformation {

        let absolute_position = mouse_position - self.state.state.cached_position;

        if absolute_position.x >= 0.0
            && absolute_position.y >= 0.0
            && absolute_position.x <= self.state.state.cached_size.x
            && absolute_position.y <= self.state.state.cached_size.y
        {

            if self.expanded {
                for element in &self.state.elements {
                    match element.borrow().hovered_element(absolute_position) {
                        HoverInformation::Hovered => return HoverInformation::Element(element.clone()),
                        HoverInformation::Missed => {}
                        hover_information => return hover_information,
                    }
                }
            }

            return HoverInformation::Hovered;
        }

        HoverInformation::Missed
    }

    fn left_click(&mut self, force_update: &mut bool) -> Option<ClickAction> {

        self.expanded = !self.expanded;
        *force_update = true;
        None
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

        let background_color = match second_theme {
            true => *theme.expandable.second_background_color,
            false => *theme.expandable.background_color,
        };

        renderer.render_background(*theme.button.border_radius, background_color);

        renderer.render_expand_arrow(
            *theme.expandable.icon_offset,
            *theme.expandable.icon_size,
            *theme.expandable.foreground_color,
            self.expanded,
        );

        let foreground_color = match self.is_element_self(hovered_element) || self.is_element_self(focused_element) {
            true => *theme.expandable.hovered_foreground_color,
            false => *theme.expandable.foreground_color,
        };

        renderer.render_text(
            &self.display,
            *theme.expandable.text_offset,
            foreground_color,
            *theme.expandable.font_size,
        );

        if self.expanded {

            self.state.render(
                &mut renderer,
                state_provider,
                interface_settings,
                theme,
                hovered_element,
                focused_element,
                !second_theme,
            );
        }
    }
}
