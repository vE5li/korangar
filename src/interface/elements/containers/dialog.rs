use procedural::*;
use num::Zero;
use std::rc::Rc;
use std::cell::RefCell;

use crate::interface::traits::Element;
use crate::interface::types::*;
use crate::interface::elements::*;
use crate::graphics::{Renderer, InterfaceRenderer, Color};

pub struct DialogContainer {
    dialog_elements: Rc<RefCell<Vec<DialogElement>>>,
    changed: Rc<RefCell<bool>>,
    npc_id: u32,
    elements: Vec<ElementCell>,
    cached_size: Size,
    cached_position: Position,
}

impl DialogContainer {

    fn to_element(dialog_element: &DialogElement, npc_id: u32) -> ElementCell {
        match dialog_element {
            DialogElement::Text(text) => cell!(Text::new(text.clone(), Color::monochrome(255), 14.0, constraint!(100%, 14))),
            DialogElement::NextButton => cell!(Button::new("next", crate::input::UserEvent::NextDialog(npc_id), false)),
            DialogElement::CloseButton => cell!(Button::new("close", crate::input::UserEvent::CloseDialog(npc_id), false)),
            DialogElement::ChoiceButton(text, index)=> cell!(EventButton::new(text.clone(), crate::input::UserEvent::ChooseDialogOption(npc_id, *index))),
        }
    }

    pub fn new(dialog_elements: Rc<RefCell<Vec<DialogElement>>>, changed: Rc<RefCell<bool>>, npc_id: u32) -> Self {

        let elements = dialog_elements
            .borrow()
            .iter()
            .map(|element| Self::to_element(element, npc_id))
            .collect();

        let cached_size = Size::zero();
        let cached_position = Position::zero();

        Self {
            dialog_elements,
            changed,
            npc_id,
            elements,
            cached_size,
            cached_position,
        }
    }
}

impl Element for DialogContainer {

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, interface_settings: &InterfaceSettings, theme: &Theme) {

        let size_constraint = &constraint!(100%, ?);

        let (mut size, position) = placement_resolver.allocate(&size_constraint);
        let mut inner_placement_resolver = placement_resolver.derive(Position::zero(), Size::zero());
        inner_placement_resolver.set_gaps(Size::new(5.0, 3.0));

        self.elements
            .iter_mut()
            .for_each(|element| element.borrow_mut().resolve(&mut inner_placement_resolver, interface_settings, theme));

        if size_constraint.height.is_flexible() {
            let final_height = inner_placement_resolver.final_height();
            let final_height = size_constraint.validated_height(final_height, placement_resolver.get_avalible().y, placement_resolver.get_avalible().y, *interface_settings.scaling);
            size.y = Some(final_height);
            placement_resolver.register_height(final_height);
        }

        self.cached_size = size.finalize();
        self.cached_position = position;
    }

    fn update(&mut self) -> Option<ChangeEvent> {

        if !*self.changed.borrow() {
            return None;
        }

        *self = Self::new(self.dialog_elements.clone(), self.changed.clone(), self.npc_id);
        *self.changed.borrow_mut() = false;

        ChangeEvent::Reresolve.into()
    }

    fn hovered_element(&self, mouse_position: Position) -> HoverInformation {
        let absolute_position = mouse_position - self.cached_position;

        if absolute_position.x >= 0.0 && absolute_position.y >= 0.0 && absolute_position.x <= self.cached_size.x && absolute_position.y <= self.cached_size.y {
            for element in &self.elements {
                match element.borrow().hovered_element(absolute_position) {
                    HoverInformation::Hovered => return HoverInformation::Element(element.clone()),
                    HoverInformation::Element(element) => return HoverInformation::Element(element),
                    HoverInformation::Ignored => return HoverInformation::Ignored,
                    HoverInformation::Missed => {},
                }
            }
        }

        HoverInformation::Missed
    }

    fn render(&self, render_target: &mut <InterfaceRenderer as Renderer>::Target, renderer: &InterfaceRenderer, state_provider: &StateProvider, interface_settings: &InterfaceSettings, theme: &Theme, parent_position: Position, clip_size: Size, hovered_element: Option<&dyn Element>, focused_element: Option<&dyn Element>, second_theme: bool) {
        let absolute_position = parent_position + self.cached_position;
        let clip_size = clip_size.zip(absolute_position + self.cached_size, f32::min);
        self.elements
            .iter()
            .for_each(|element| element.borrow().render(render_target, renderer, state_provider, interface_settings, theme, absolute_position, clip_size, hovered_element, focused_element, second_theme));
    }
}
