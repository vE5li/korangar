use std::rc::Weak;

use procedural::size_bound;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::{Element, *};

pub struct Expandable {
    display: String,
    expanded: bool,
    open_size_bound: SizeBound,
    closed_size_bound: SizeBound,
    cached_closed_size: ScreenSize,
    state: ContainerState,
}

impl Expandable {
    pub fn new(display: String, elements: Vec<ElementCell>, expanded: bool) -> Self {
        let state = ContainerState::new(elements);

        Self {
            display,
            expanded,
            open_size_bound: size_bound!(100%, ?),
            closed_size_bound: size_bound!(100%, 18),
            cached_closed_size: ScreenSize::default(),
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

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, interface_settings: &InterfaceSettings, theme: &InterfaceTheme) {
        let closed_size = self
            .closed_size_bound
            .resolve_element(
                placement_resolver.get_available(),
                placement_resolver.get_remaining(),
                &placement_resolver.get_parent_limits(),
                interface_settings.scaling.get(),
            )
            .finalize();

        let size_bound = match self.expanded && !self.state.elements.is_empty() {
            true => &self.open_size_bound,
            false => &self.closed_size_bound,
        };

        let screen_position =
            ScreenPosition::only_top(closed_size.height) + theme.expandable.element_offset.get() * interface_settings.scaling.get();

        let (mut inner_placement_resolver, mut size, position) =
            placement_resolver.derive(size_bound, screen_position, theme.expandable.border_size.get());
        let parent_limits = inner_placement_resolver.get_parent_limits();

        if self.expanded && !self.state.elements.is_empty() {
            inner_placement_resolver.set_gaps(theme.expandable.gaps.get());

            self.state.elements.iter_mut().for_each(|element| {
                element
                    .borrow_mut()
                    .resolve(&mut inner_placement_resolver, interface_settings, theme)
            });

            if self.open_size_bound.height.is_flexible() {
                let final_height = inner_placement_resolver.final_height()
                    + closed_size.height
                    + theme.expandable.element_offset.get().top * interface_settings.scaling.get()
                    + theme.expandable.border_size.get().height * interface_settings.scaling.get() * 2.0;

                let final_height = self.open_size_bound.validated_height(
                    final_height,
                    placement_resolver.get_available().height,
                    placement_resolver.get_available().height,
                    &parent_limits,
                    interface_settings.scaling.get(),
                );

                size.height = Some(final_height);
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

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation {
        let absolute_position = ScreenPosition::from_size(mouse_position - self.state.state.cached_position);

        if absolute_position.left >= 0.0
            && absolute_position.top >= 0.0
            && absolute_position.left <= self.state.state.cached_size.width
            && absolute_position.top <= self.state.state.cached_size.height
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

    fn left_click(&mut self, force_update: &mut bool) -> Vec<ClickAction> {
        self.expanded = !self.expanded;
        *force_update = true;
        Vec::new()
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

        let background_color = match second_theme {
            true => theme.expandable.second_background_color.get(),
            false => theme.expandable.background_color.get(),
        };

        renderer.render_background(theme.button.corner_radius.get(), background_color);

        renderer.render_expand_arrow(
            theme.expandable.icon_offset.get(),
            theme.expandable.icon_size.get(),
            theme.expandable.foreground_color.get(),
            self.expanded,
        );

        let foreground_color = match self.is_element_self(hovered_element) || self.is_element_self(focused_element) {
            true => theme.expandable.hovered_foreground_color.get(),
            false => theme.expandable.foreground_color.get(),
        };

        renderer.render_text(
            &self.display,
            theme.expandable.text_offset.get(),
            foreground_color,
            theme.expandable.font_size.get(),
        );

        if self.expanded && !self.state.elements.is_empty() {
            self.state.render(
                &mut renderer,
                state_provider,
                interface_settings,
                theme,
                hovered_element,
                focused_element,
                mouse_mode,
                !second_theme,
            );
        }
    }
}
