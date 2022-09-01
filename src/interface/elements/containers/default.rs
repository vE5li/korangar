use std::rc::Weak;

use procedural::*;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::interface::{Element, *};

pub struct Container {
    state: ContainerState,
    size_constraint: SizeConstraint,
}

impl Container {

    pub const DEFAULT_SIZE: SizeConstraint = constraint!(100%, ?);

    pub fn new(elements: Vec<ElementCell>, size_constraint: SizeConstraint) -> Self {

        Self {
            state: ContainerState {
                elements,
                state: Default::default(),
            },
            size_constraint,
        }
    }
}

impl Element for Container {

    fn get_state(&self) -> &ElementState {
        &self.state.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state.state
    }

    fn link_back(&mut self, weak_self: Weak<RefCell<dyn Element>>, weak_parent: Option<Weak<RefCell<dyn Element>>>) {
        self.state.link_back(weak_self, weak_parent);
    }

    fn focus_next(
        &self,
        self_cell: Rc<RefCell<dyn Element>>,
        caller_cell: Option<Rc<RefCell<dyn Element>>>,
        focus_mode: FocusMode,
    ) -> Option<Rc<RefCell<dyn Element>>> {

        if let Some(caller_cell) = caller_cell {

            let position = self
                .state
                .elements
                .iter()
                .position(|element| element.borrow().is_element_self(Some(&*caller_cell.borrow())));

            println!("{:?}", position);

            if let Some(position) = position {
                match position + 1 == self.state.elements.len() {

                    true => {

                        println!("HERE: {}", self.state.state.parent_element.is_some());
                        if let Some(parent_element) = &self.state.state.parent_element {

                            let parent_element = parent_element.upgrade().unwrap();
                            let next_element = parent_element
                                .borrow()
                                .focus_next(parent_element.clone(), Some(self_cell), focus_mode);
                            return next_element;
                        }

                        return Some(self.state.elements[0].clone());
                    }

                    false => return self.state.elements[position + 1].clone().into(),
                }
            }

            panic!("when did this happen? implement correct behavior");
        }

        // getting here means the container itself is currently being focused and we want to call
        // it's parent container focus_next (if possible) to get the next sibling element
        if let Some(parent_element) = &self.state.state.parent_element {

            let parent_element = parent_element.upgrade().unwrap();
            let next_element = parent_element
                .borrow()
                .focus_next(parent_element.clone(), Some(self_cell), focus_mode);
            return next_element;
        }

        // TODO: check if element is focusable
        Some(self.state.elements[0].clone())
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, interface_settings: &InterfaceSettings, theme: &Theme) {
        self.state
            .resolve(placement_resolver, interface_settings, theme, &self.size_constraint);
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        self.state.update()
    }

    fn hovered_element(&self, mouse_position: Position) -> HoverInformation {
        self.state.hovered_element::<false>(mouse_position)
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

        self.state.render(
            &mut renderer,
            state_provider,
            interface_settings,
            theme,
            hovered_element,
            focused_element,
            second_theme,
        );
    }
}
