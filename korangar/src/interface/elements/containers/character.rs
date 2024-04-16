use std::cell::RefCell;
use std::rc::Weak;

use korangar_interface::application::FontSizeTrait;
use korangar_interface::elements::{ButtonBuilder, ContainerState, Element, ElementCell, ElementState, ElementWrap, Focus, Text};
use korangar_interface::event::{ChangeEvent, ClickAction, HoverInformation};
use korangar_interface::layout::PlacementResolver;
use korangar_interface::state::{PlainRemote, Remote};
use korangar_interface::{dimension_bound, size_bound};
use ragnarok_networking::CharacterInformation;

use crate::graphics::{Color, InterfaceRenderer, Renderer};
use crate::input::{MouseInputMode, UserEvent};
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{ScreenClip, ScreenPosition, ScreenSize};
use crate::interface::theme::InterfaceTheme;
use crate::loaders::FontSize;

// TODO: rework all of this
pub struct CharacterPreview {
    characters: PlainRemote<Vec<CharacterInformation>>,
    move_request: PlainRemote<Option<usize>>,
    slot: usize,
    state: ContainerState<InterfaceSettings>,
}

impl CharacterPreview {
    fn get_elements(
        characters: &PlainRemote<Vec<CharacterInformation>>,
        move_request: &PlainRemote<Option<usize>>,
        slot: usize,
    ) -> Vec<ElementCell<InterfaceSettings>> {
        if let Some(origin_slot) = *move_request.get() {
            let text = match origin_slot == slot {
                true => "Click to cancel",
                false => "Switch",
            };

            return vec![
                Text::default()
                    .with_text(text.to_owned())
                    .with_foreground_color(|_| Color::rgb_u8(200, 140, 180))
                    .wrap(),
            ];
        }

        let characters = characters.get();
        let character_information = characters.iter().find(|character| character.character_number as usize == slot);

        if let Some(character_information) = character_information {
            return vec![
                Text::default()
                    .with_text(character_information.name.clone())
                    .with_foreground_color(|_| Color::rgb_u8(220, 210, 210))
                    .with_font_size(|_| FontSize::new(18.0))
                    .wrap(),
                ButtonBuilder::new()
                    .with_text("Switch")
                    .with_event(UserEvent::RequestSwitchCharacterSlot(slot))
                    .with_background_color(|_| Color::rgb_u8(161, 141, 141))
                    .with_width_bound(dimension_bound!(50%))
                    .build()
                    .wrap(),
                ButtonBuilder::new()
                    .with_text("Delete")
                    .with_event(UserEvent::DeleteCharacter(character_information.character_id))
                    .with_background_color(|theme: &InterfaceTheme| theme.close_button.background_color.get())
                    .with_foreground_color(|theme: &InterfaceTheme| theme.close_button.foreground_color.get())
                    .with_width_bound(dimension_bound!(50%))
                    .build()
                    .wrap(),
            ];
        }

        vec![
            Text::default()
                .with_text("New character")
                .with_foreground_color(|_| Color::rgb_u8(200, 140, 180))
                .wrap(),
        ]
    }

    pub fn new(characters: PlainRemote<Vec<CharacterInformation>>, move_request: PlainRemote<Option<usize>>, slot: usize) -> Self {
        let elements = Self::get_elements(&characters, &move_request, slot);
        let state = ContainerState::new(elements);

        Self {
            characters,
            move_request,
            slot,
            state,
        }
    }

    fn has_character(&self) -> bool {
        self.state.elements.len() > 1 // TODO:
    }
}

impl Element<InterfaceSettings> for CharacterPreview {
    fn get_state(&self) -> &ElementState<InterfaceSettings> {
        &self.state.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<InterfaceSettings> {
        &mut self.state.state
    }

    fn link_back(
        &mut self,
        weak_self: Weak<RefCell<dyn Element<InterfaceSettings>>>,
        weak_parent: Option<Weak<RefCell<dyn Element<InterfaceSettings>>>>,
    ) {
        self.state.link_back(weak_self, weak_parent);
    }

    fn is_focusable(&self) -> bool {
        self.state.is_focusable::<true>()
    }

    fn focus_next(
        &self,
        self_cell: ElementCell<InterfaceSettings>,
        caller_cell: Option<ElementCell<InterfaceSettings>>,
        focus: Focus,
    ) -> Option<ElementCell<InterfaceSettings>> {
        self.state.focus_next::<true>(self_cell, caller_cell, focus)
    }

    fn resolve(
        &mut self,
        placement_resolver: &mut PlacementResolver<InterfaceSettings>,
        application: &InterfaceSettings,
        theme: &InterfaceTheme,
    ) {
        let size_bound = &size_bound!(20%, 150);
        self.state
            .resolve(placement_resolver, application, theme, size_bound, ScreenSize::uniform(4.0));
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        let characters_changed = self.characters.consume_changed();
        let move_request_changed = self.move_request.consume_changed();

        if characters_changed || move_request_changed {
            let weak_self = self.state.state.self_element.take().unwrap();
            let weak_parent = self.state.state.parent_element.clone();

            *self = Self::new(self.characters.clone(), self.move_request.clone(), self.slot);

            // important: link back after creating elements, otherwise focus navigation and
            // scrolling would break
            self.state.link_back(weak_self, weak_parent);

            return Some(ChangeEvent::RESOLVE_WINDOW);
        }

        None
    }

    fn left_click(&mut self, _update: &mut bool) -> Vec<ClickAction<InterfaceSettings>> {
        if let Some(origin_slot) = *self.move_request.get() {
            let event = match origin_slot == self.slot {
                true => UserEvent::CancelSwitchCharacterSlot,
                false => UserEvent::SwitchCharacterSlot(self.slot),
            };

            return vec![ClickAction::Custom(event)];
        }

        let event = match self.has_character() {
            true => UserEvent::SelectCharacter(self.slot),
            false => UserEvent::OpenCharacterCreationWindow(self.slot),
        };

        vec![ClickAction::Custom(event)]
    }

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation<InterfaceSettings> {
        match mouse_mode {
            MouseInputMode::None => self.state.hovered_element(mouse_position, mouse_mode, true),
            _ => HoverInformation::Missed,
        }
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

        let background_color = match self.is_element_self(hovered_element) || self.is_element_self(focused_element) {
            true => theme.button.hovered_background_color.get(),
            false => theme.button.background_color.get(),
        };

        renderer.render_background(theme.button.corner_radius.get(), background_color);

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
