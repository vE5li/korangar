use procedural::*;
use std::rc::Rc;
use std::cell::RefCell;
use num::Zero;

use crate::input::UserEvent;
use crate::interface::traits::Element;
use crate::interface::types::*;
use crate::interface::elements::*;
use crate::graphics::{ Renderer, Color };
use crate::network::CharacterInformation;

pub struct Chat {
    characters: Rc<RefCell<Vec<CharacterInformation>>>,
    move_request: Rc<RefCell<Option<usize>>>,
    changed: Rc<RefCell<bool>>,
    slot: usize,
    elements: Vec<ElementCell>,
    cached_size: Size,
    cached_position: Position,
}

impl Element for Chat {

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, interface_settings: &InterfaceSettings, theme: &Theme) {

        let size_constraint = &constraint!(20%, 150);

        let (mut size, position) = placement_resolver.allocate(size_constraint);
        let mut inner_placement_resolver = placement_resolver.derive(Position::zero(), Size::new(3.0, 3.0));
        inner_placement_resolver.set_gaps(Size::new(10.0, 3.0));

        self.elements.iter_mut().for_each(|element| element.borrow_mut().resolve(&mut inner_placement_resolver, interface_settings, theme));

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

        *self = Self::new(self.characters.clone(), self.move_request.clone(), self.changed.clone(), self.slot);

        ChangeEvent::Reresolve.into()
    }

    fn left_click(&mut self, _update: &mut bool) -> Option<ClickAction> {

        if let Some(origin_slot) = *self.move_request.borrow() {

            let event = match origin_slot == self.slot {
                true => UserEvent::CancelSwitchCharacterSlot,
                false => UserEvent::SwitchCharacterSlot(self.slot),
            };

            return Some(ClickAction::Event(event));
        }

        let event = match self.has_character() {
            true => UserEvent::SelectCharacter(self.slot),
            false => UserEvent::CreateCharacter(self.slot),
        };

        Some(ClickAction::Event(event))
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

            return HoverInformation::Hovered;
        }

        HoverInformation::Missed
    }

    fn render(&self, renderer: &mut Renderer, state_provider: &StateProvider, interface_settings: &InterfaceSettings, theme: &Theme, parent_position: Position, clip_size: Size, hovered_element: Option<&dyn Element>, second_theme: bool) {
        let absolute_position = parent_position + self.cached_position;

        match matches!(hovered_element, Some(reference) if std::ptr::eq(reference as *const _ as *const (), self as *const _ as *const ())) {
            true => renderer.render_rectangle(absolute_position, self.cached_size, clip_size, *theme.button.border_radius * *interface_settings.scaling, *theme.button.hovered_background_color),
            false => renderer.render_rectangle(absolute_position, self.cached_size, clip_size, *theme.button.border_radius * *interface_settings.scaling, *theme.button.background_color),
        }

        let clip_size = clip_size.zip(absolute_position + self.cached_size, f32::min);
        self.elements.iter().for_each(|element| element.borrow().render(renderer, state_provider, interface_settings, theme, absolute_position, clip_size, hovered_element, second_theme));
    }
}
