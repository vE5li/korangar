use procedural::*;
use num::Zero;

use crate::interface::Element;
use crate::interface::*;
use crate::graphics::{Renderer, InterfaceRenderer};

pub struct Expandable {
    display: String,
    elements: Vec<ElementCell>,
    expanded: bool,
    open_size_constraint: SizeConstraint,
    closed_size_constraint: SizeConstraint,
    cached_size: Size,
    cached_closed_size: Size,
    cached_position: Position,
}

impl Expandable {

    pub fn new(display: String, elements: Vec<ElementCell>, expanded: bool) -> Self {
        Self {
            display,
            expanded,
            open_size_constraint: constraint!(100%, ?),
            closed_size_constraint: constraint!(100%, 18),
            cached_size: Size::zero(),
            cached_closed_size: Size::zero(),
            cached_position: Position::zero(),
            elements,
        }
    }
}

impl Element for Expandable {

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, interface_settings: &InterfaceSettings, theme: &Theme) {

        let closed_size = self.closed_size_constraint.resolve_partial(placement_resolver.get_avalible(), placement_resolver.get_remaining(), *interface_settings.scaling).finalize();

        let (mut size, position) = match self.expanded {
            true => placement_resolver.allocate(&self.open_size_constraint),
            false => placement_resolver.allocate(&self.closed_size_constraint),
        };

        if self.expanded {
            let mut inner_placement_resolver = placement_resolver.derive(Position::new(0.0, closed_size.y) + *theme.expandable.element_offset * *interface_settings.scaling, *theme.expandable.border_size);
            inner_placement_resolver.set_gaps(*theme.expandable.gaps);

            self.elements.iter_mut().for_each(|element| element.borrow_mut().resolve(&mut inner_placement_resolver, interface_settings, theme));

            if self.open_size_constraint.height.is_flexible() {
                let final_height = inner_placement_resolver.final_height() + closed_size.y + theme.expandable.element_offset.y * *interface_settings.scaling + theme.expandable.border_size.y * *interface_settings.scaling * 2.0;
                let final_height = self.open_size_constraint.validated_height(final_height, placement_resolver.get_avalible().y, placement_resolver.get_avalible().y, *interface_settings.scaling);
                size.y = Some(final_height);
                placement_resolver.register_height(final_height);
            }
        }

        self.cached_size = size.finalize();
        self.cached_closed_size = closed_size;
        self.cached_position = position;
    }

    fn update(&mut self) -> Option<ChangeEvent> {

        if !self.expanded {
            return None;
        }

        self.elements
            .iter_mut()
            .map(|element| element.borrow_mut().update())
            .fold(None, |current, event| current.zip_with(event, ChangeEvent::combine).or(current).or(event))
    }

    fn hovered_element(&self, mouse_position: Position) -> HoverInformation {
        let absolute_position = mouse_position - self.cached_position;

        if absolute_position.x >= 0.0 && absolute_position.y >= 0.0 && absolute_position.x <= self.cached_size.x && absolute_position.y <= self.cached_size.y {
            if self.expanded {
                for element in &self.elements {
                    match element.borrow().hovered_element(absolute_position) {
                        HoverInformation::Hovered => return HoverInformation::Element(element.clone()),
                        HoverInformation::Element(element) => return HoverInformation::Element(element),
                        HoverInformation::Ignored => return HoverInformation::Ignored,
                        HoverInformation::Missed => {},
                    }
                }
            }

            match absolute_position.x <= self.cached_closed_size.x && absolute_position.y <= self.cached_closed_size.y {
                true => return HoverInformation::Hovered,
                false => return HoverInformation::Ignored,
            }
        }

        HoverInformation::Missed
    }

    fn left_click(&mut self, force_update: &mut bool) -> Option<ClickAction> {
        self.expanded = !self.expanded;
        *force_update = true;
        None
    }

    fn render(&self, render_target: &mut <InterfaceRenderer as Renderer>::Target, renderer: &InterfaceRenderer, state_provider: &StateProvider, interface_settings: &InterfaceSettings, theme: &Theme, parent_position: Position, clip_size: Size, hovered_element: Option<&dyn Element>, focused_element: Option<&dyn Element>, second_theme: bool) {
        let absolute_position = parent_position + self.cached_position;
        let clip_size = clip_size.zip(absolute_position + self.cached_size, f32::min);

        let background_color = match second_theme {
            true => *theme.expandable.second_background_color,
            false => *theme.expandable.background_color,
        };

        renderer.render_rectangle(render_target, absolute_position, self.cached_size, clip_size, *theme.expandable.border_radius * *interface_settings.scaling, background_color);
        renderer.render_expand_arrow(render_target, absolute_position + *theme.expandable.icon_offset * *interface_settings.scaling, *theme.expandable.icon_size * *interface_settings.scaling, clip_size, *theme.expandable.foreground_color, self.expanded);

        match matches!(hovered_element, Some(reference) if std::ptr::eq(reference as *const _ as *const (), self as *const _ as *const ())) {
            true => renderer.render_text(render_target, &self.display, absolute_position + *theme.expandable.text_offset * *interface_settings.scaling, clip_size, *theme.expandable.hovered_foreground_color, *theme.expandable.font_size * *interface_settings.scaling),
            false => renderer.render_text(render_target, &self.display, absolute_position + *theme.expandable.text_offset * *interface_settings.scaling, clip_size, *theme.expandable.foreground_color, *theme.expandable.font_size * *interface_settings.scaling),
        }

        if self.expanded {
            self.elements.iter().for_each(|element| element.borrow().render(render_target, renderer, state_provider, interface_settings, theme, absolute_position, clip_size, hovered_element, focused_element, !second_theme));
        }
    }
}
