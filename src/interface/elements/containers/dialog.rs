use procedural::*;

use crate::graphics::{Color, InterfaceRenderer, Renderer};
use crate::input::{MouseInputMode, UserEvent};
use crate::interface::{Element, *};

#[derive(Clone, PartialEq, Eq)]
pub enum DialogElement {
    Text(String),
    NextButton,
    CloseButton,
    ChoiceButton(String, i8),
}

pub struct DialogContainer {
    dialog_elements: Remote<Vec<DialogElement>>,
    npc_id: EntityId,
    state: ContainerState,
}

impl DialogContainer {
    fn to_element(dialog_element: &DialogElement, npc_id: EntityId) -> ElementCell {
        match dialog_element {
            DialogElement::Text(text) => Text::default()
                .with_text(text.clone())
                .with_foreground_color(|_| Color::monochrome_u8(255))
                .wrap(),
            DialogElement::NextButton => Button::default().with_text("next").with_event(UserEvent::NextDialog(npc_id)).wrap(),
            DialogElement::CloseButton => Button::default()
                .with_text("close")
                .with_event(UserEvent::CloseDialog(npc_id))
                .wrap(),
            DialogElement::ChoiceButton(text, index) => Button::default()
                .with_text(text.clone())
                .with_event(UserEvent::ChooseDialogOption(npc_id, *index))
                .wrap(),
        }
    }

    pub fn new(dialog_elements: Remote<Vec<DialogElement>>, npc_id: EntityId) -> Self {
        let elements = dialog_elements
            .borrow()
            .iter()
            .map(|element| Self::to_element(element, npc_id))
            .collect();

        let state = ContainerState::new(elements);

        Self {
            dialog_elements,
            npc_id,
            state,
        }
    }
}

impl Element for DialogContainer {
    fn get_state(&self) -> &ElementState {
        &self.state.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state.state
    }

    fn link_back(&mut self, weak_self: WeakElementCell, weak_parent: Option<WeakElementCell>) {
        self.state.link_back(weak_self, weak_parent);
    }

    // TODO: focus related things

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, interface_settings: &InterfaceSettings, theme: &InterfaceTheme) {
        let size_constraint = &constraint!(100%, ?);
        self.state.resolve(
            placement_resolver,
            interface_settings,
            theme,
            size_constraint,
            ScreenSize::uniform(3.0),
        );
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        if self.dialog_elements.consume_changed() {
            *self = Self::new(self.dialog_elements.clone(), self.npc_id);

            // TODO: link back like in character container

            return Some(ChangeEvent::RESOLVE_WINDOW);
        }

        None
    }

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation {
        self.state.hovered_element(mouse_position, mouse_mode, false)
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

        self.state.render(
            &mut renderer,
            state_provider,
            interface_settings,
            theme,
            hovered_element,
            focused_element,
            mouse_mode,
            second_theme,
        );
    }
}
