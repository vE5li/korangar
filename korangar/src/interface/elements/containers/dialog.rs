use korangar_interface::elements::{ButtonBuilder, ContainerState, Element, ElementCell, ElementState, ElementWrap, Text, WeakElementCell};
use korangar_interface::event::{ChangeEvent, HoverInformation};
use korangar_interface::layout::PlacementResolver;
use korangar_interface::size_bound;
use korangar_interface::state::{PlainRemote, Remote};
use ragnarok_packets::EntityId;

use crate::graphics::{Color, InterfaceRenderer, Renderer};
use crate::input::{MouseInputMode, UserEvent};
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{ScreenClip, ScreenPosition, ScreenSize};
use crate::interface::theme::InterfaceTheme;

#[derive(Clone, PartialEq, Eq)]
pub enum DialogElement {
    Text(String),
    NextButton,
    CloseButton,
    ChoiceButton(String, i8),
}

pub struct DialogContainer {
    dialog_elements: PlainRemote<Vec<DialogElement>>,
    npc_id: EntityId,
    state: ContainerState<InterfaceSettings>,
}

impl DialogContainer {
    fn to_element(dialog_element: &DialogElement, npc_id: EntityId) -> ElementCell<InterfaceSettings> {
        match dialog_element {
            DialogElement::Text(text) => Text::default()
                .with_text(text.clone())
                .with_foreground_color(|_| Color::monochrome_u8(255))
                .wrap(),
            DialogElement::NextButton => ButtonBuilder::new()
                .with_text("next")
                .with_event(UserEvent::NextDialog(npc_id))
                .build()
                .wrap(),
            DialogElement::CloseButton => ButtonBuilder::new()
                .with_text("close")
                .with_event(UserEvent::CloseDialog(npc_id))
                .build()
                .wrap(),
            DialogElement::ChoiceButton(text, index) => ButtonBuilder::new()
                .with_text(text.clone())
                .with_event(UserEvent::ChooseDialogOption(npc_id, *index))
                .build()
                .wrap(),
        }
    }

    pub fn new(dialog_elements: PlainRemote<Vec<DialogElement>>, npc_id: EntityId) -> Self {
        let elements = dialog_elements
            .get()
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

impl Element<InterfaceSettings> for DialogContainer {
    fn get_state(&self) -> &ElementState<InterfaceSettings> {
        &self.state.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<InterfaceSettings> {
        &mut self.state.state
    }

    fn link_back(&mut self, weak_self: WeakElementCell<InterfaceSettings>, weak_parent: Option<WeakElementCell<InterfaceSettings>>) {
        self.state.link_back(weak_self, weak_parent);
    }

    // TODO: focus related things

    fn resolve(
        &mut self,
        placement_resolver: &mut PlacementResolver<InterfaceSettings>,
        application: &InterfaceSettings,
        theme: &InterfaceTheme,
    ) {
        let size_bound = &size_bound!(100%, ?);
        self.state
            .resolve(placement_resolver, application, theme, size_bound, ScreenSize::uniform(3.0));
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        if self.dialog_elements.consume_changed() {
            *self = Self::new(self.dialog_elements.clone(), self.npc_id);

            // TODO: link back like in character container

            return Some(ChangeEvent::RESOLVE_WINDOW);
        }

        None
    }

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation<InterfaceSettings> {
        self.state.hovered_element(mouse_position, mouse_mode, false)
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        application: &InterfaceSettings,
        theme: &InterfaceTheme,
        parent_position: ScreenPosition,
        screen_clip: ScreenClip,
        hovered_element: Option<&dyn Element<InterfaceSettings>>,
        focused_element: Option<&dyn Element<InterfaceSettings>>,
        mouse_mode: &MouseInputMode,
        second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .state
            .element_renderer(render_target, renderer, application, parent_position, screen_clip);

        self.state.render(
            &mut renderer,
            application,
            theme,
            hovered_element,
            focused_element,
            mouse_mode,
            second_theme,
        );
    }
}
