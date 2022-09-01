mod character;
mod default;
mod dialog;
mod expandable;
mod scroll;

use std::rc::Weak;

use cgmath::Zero;
use derive_new::new;

pub use self::character::CharacterPreview;
pub use self::default::Container;
pub use self::dialog::{DialogContainer, DialogElement};
pub use self::expandable::Expandable;
pub use self::scroll::ScrollView;
use crate::interface::*;

#[derive(new)]
pub struct ContainerState {
    elements: Vec<ElementCell>,
    #[new(default)]
    state: ElementState,
}

impl ContainerState {

    pub fn link_back(&mut self, weak_self: Weak<RefCell<dyn Element>>, weak_parent: Option<Weak<RefCell<dyn Element>>>) {

        self.state.link_back(weak_parent);
        self.elements.iter().for_each(|element| {

            let weak_element = Rc::downgrade(element);
            element.borrow_mut().link_back(weak_element, Some(weak_self.clone()));
        });
    }

    pub fn resolve(
        &mut self,
        placement_resolver: &mut PlacementResolver,
        interface_settings: &InterfaceSettings,
        theme: &Theme,
        size_constraint: &SizeConstraint,
    ) {

        let (mut size, position) = placement_resolver.allocate(&size_constraint);
        let mut inner_placement_resolver = placement_resolver.derive(Position::zero(), Size::zero());
        inner_placement_resolver.set_gaps(Size::new(5.0, 3.0));

        self.elements.iter_mut().for_each(|element| {

            element
                .borrow_mut()
                .resolve(&mut inner_placement_resolver, interface_settings, theme)
        });

        if size_constraint.height.is_flexible() {

            let final_height = inner_placement_resolver.final_height();
            let final_height = size_constraint.validated_height(
                final_height,
                placement_resolver.get_avalible().y,
                placement_resolver.get_avalible().y,
                *interface_settings.scaling,
            );
            size.y = Some(final_height);
            placement_resolver.register_height(final_height);
        }

        self.state.cached_size = size.finalize();
        self.state.cached_position = position;
    }

    pub fn update(&mut self) -> Option<ChangeEvent> {

        self.elements
            .iter_mut()
            .map(|element| element.borrow_mut().update())
            .fold(None, |current, other| {
                current.zip_with(other, ChangeEvent::combine).or(current).or(other)
            })
    }

    pub fn hovered_element<const HOVERABLE: bool>(&self, mouse_position: Position) -> HoverInformation {

        let absolute_position = mouse_position - self.state.cached_position;

        if absolute_position.x >= 0.0
            && absolute_position.y >= 0.0
            && absolute_position.x <= self.state.cached_size.x
            && absolute_position.y <= self.state.cached_size.y
        {

            for element in &self.elements {
                match element.borrow().hovered_element(absolute_position) {
                    HoverInformation::Hovered => return HoverInformation::Element(element.clone()),
                    HoverInformation::Missed => {}
                    hover_information => return hover_information,
                }
            }

            if HOVERABLE {
                return HoverInformation::Hovered;
            }
        }

        HoverInformation::Missed
    }

    pub fn render(
        &self,
        renderer: &mut ElementRenderer,
        state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        theme: &Theme,
        hovered_element: Option<&dyn Element>,
        focused_element: Option<&dyn Element>,
        second_theme: bool,
    ) {

        self.elements.iter().for_each(|element| {

            renderer.render_element(
                &*element.borrow(),
                state_provider,
                interface_settings,
                theme,
                hovered_element,
                focused_element,
                second_theme,
            )
        });
    }
}
