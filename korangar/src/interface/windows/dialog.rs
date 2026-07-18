use std::cell::UnsafeCell;

use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::store::ElementStoreMut;
use korangar_interface::element::{Element, ElementBox, ErasedElement, StateElement};
use korangar_interface::layout::{Resolvers, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::EntityId;
use rust_state::{Path, RustState, State};

use super::WindowClass;
use crate::input::InputEvent;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

/// Maximum number of characters for a dialog input. The server caps text
/// inputs at rAthena's `CHATBOX_SIZE` (70).
const MAXIMUM_INPUT_LENGTH: usize = 70;

fn parse_dialog_number(text: &str) -> Option<i32> {
    text.trim().parse().ok()
}

/// A small wrapper struct that serves two purposes:
/// - Making the elements nicer to construct by putting the [`UnsafeCell::new`]
///   and [`Box::new`] behind a function call.
/// - Storing information about which elements are next buttons since we need to
///   be able to remove those individually.
#[derive(RustState, StateElement)]
pub struct DialogElement {
    /// Stores the UI element.
    // TODO: Unfortunately this has to be an unsafe cell as of now. Ideally this can be changed
    // later.
    #[hidden_element]
    element: UnsafeCell<ElementBox<ClientState>>,
    is_next_button: bool,
    /// Marks the elements of an input row so they can be removed once the
    /// input is submitted.
    is_input: bool,
}

impl DialogElement {
    /// Creates a new dialog element.
    #[inline(always)]
    fn new<E>(element: E, is_next_button: bool) -> Self
    where
        E: Element<ClientState> + 'static,
    {
        Self {
            element: UnsafeCell::new(ErasedElement::new(element)),
            is_next_button,
            is_input: false,
        }
    }

    /// Creates a new dialog element that is part of an input row.
    #[inline(always)]
    fn new_input<E>(element: E) -> Self
    where
        E: Element<ClientState> + 'static,
    {
        Self {
            element: UnsafeCell::new(ErasedElement::new(element)),
            is_next_button: false,
            is_input: true,
        }
    }
}

/// Internal state of the dialog window.
#[derive(RustState, StateElement)]
pub struct DialogWindowState {
    /// All current dialog elements.
    elements: Vec<DialogElement>,
    /// The entity id of the NPC the player is talking to.
    npc_id: EntityId,
    /// Whether or not the elements should be cleared the next time
    /// [`start`](Self::start) is called.
    clear_next: bool,
    /// Backing store for the text box of an input row.
    input_buffer: String,
}

impl DialogWindowState {
    /// Initialize the dialog. This is important so we have the correct entity
    /// id when sending packets to the server.
    pub fn initialize(&mut self, npc_id: EntityId) -> &mut Self {
        self.npc_id = npc_id;
        self
    }

    /// Add text to the dialog.
    pub fn add_text(&mut self, text: String) {
        use korangar_interface::prelude::*;

        if self.clear_next {
            self.elements.clear();
            self.clear_next = false;
        }

        self.elements.push(DialogElement::new(
            text! {
                text: text,
            },
            false,
        ));
    }

    /// Add add next button to the dialog.
    ///
    /// This also sets the internal state to clear the dialog the next time
    /// [`start`](Self::start) is called.
    pub fn add_next_button(&mut self) {
        use korangar_interface::prelude::*;

        let npc_id = self.npc_id;

        self.elements.push(DialogElement::new(
            button! {
                text: client_state().localization().next_button_text(),
                event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                    queue.queue(InputEvent::NextDialog { npc_id });
                },
            },
            true,
        ));

        self.clear_next = true;
    }

    /// Add a close button to the dialog.
    ///
    /// This also removes any existing "Next"-buttons.
    ///
    /// I am unsure why that's the behavior of the official client.
    pub fn add_close_button(&mut self) {
        use korangar_interface::prelude::*;

        self.elements.retain(|element| !element.is_next_button && !element.is_input);

        let npc_id = self.npc_id;

        self.elements.push(DialogElement::new(
            button! {
                text: client_state().localization().close_button_text(),
                event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                    queue.queue(InputEvent::CloseDialog { npc_id });
                },
            },
            false,
        ));
    }

    /// Add multiple buttons, one for each choice.
    ///
    /// This also removes any existing "Next"-buttons.
    ///
    /// I am unsure why that's the behavior of the official client.
    pub fn add_choice_buttons(&mut self, choices: Vec<String>) {
        use korangar_interface::prelude::*;

        self.elements.retain(|element| !element.is_next_button && !element.is_input);

        let npc_id = self.npc_id;

        choices.into_iter().enumerate().for_each(|(index, text)| {
            self.elements.push(DialogElement::new(
                button! {
                    text: text,
                    event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                        queue.queue(InputEvent::ChooseDialogOption { npc_id, option: index as i8 + 1 });
                    },
                },
                false,
            ))
        });
    }

    /// Add a number input row to the dialog.
    pub fn add_number_input(&mut self) {
        self.add_input(true);
    }

    /// Add a text input row to the dialog.
    pub fn add_text_input(&mut self) {
        self.add_input(false);
    }

    /// Add an input row (text box and an "OK"-button) to the dialog.
    fn add_input(&mut self, numbers_only: bool) {
        use korangar_interface::prelude::*;

        if self.clear_next {
            // An input request may be the first packet after advancing a dialog page.
            // In that case, clear the previous page just like add_text() does.
            self.elements.clear();
            self.clear_next = false;
        } else {
            // The server should only ever request one input at a time. Also remove
            // any stale Next button: the server is now waiting for an input packet.
            self.elements.retain(|element| !element.is_input && !element.is_next_button);
        }
        self.input_buffer.clear();

        let npc_id = self.npc_id;
        let input_path = client_state().dialog_window().input_buffer();

        struct DialogInputTextBox;

        let submit_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let text = state.get(&input_path).clone();

            match numbers_only {
                true => {
                    let Some(value) = parse_dialog_number(&text) else {
                        return;
                    };
                    queue.queue(InputEvent::SubmitDialogNumberInput { npc_id, value });
                }
                false => queue.queue(InputEvent::SubmitDialogTextInput { npc_id, text }),
            }
        };

        self.elements.push(DialogElement::new_input(text_box! {
            ghost_text: client_state().localization().dialog_text_box_message(),
            state: input_path,
            input_handler: DefaultHandler::<_, _, MAXIMUM_INPUT_LENGTH>::new(input_path, submit_action),
            focus_id: DialogInputTextBox,
        }));

        self.elements.push(DialogElement::new_input(button! {
            text: client_state().localization().okay_button_text(),
            event: submit_action,
        }));
    }

    /// Remove the input row after the input has been submitted.
    pub fn input_submitted(&mut self) {
        self.elements.retain(|element| !element.is_input);
        self.input_buffer.clear();
    }

    /// End the dialog.
    ///
    /// This has no side effects.
    pub fn end(&mut self) {
        self.elements.clear();
        self.clear_next = false;
    }
}

impl Default for DialogWindowState {
    fn default() -> Self {
        Self {
            elements: Default::default(),
            // Arguably not very clean but avoids using an Option.
            npc_id: EntityId(0),
            clear_next: false,
            input_buffer: Default::default(),
        }
    }
}

/// Wrapper struct for collecting all [`DialogElement::element`]s into a single
/// element.
struct InnerElement<A> {
    dialog_elements_path: A,
}

impl<A> Element<ClientState> for InnerElement<A>
where
    A: Path<ClientState, Vec<DialogElement>>,
{
    type LayoutInfo = ();

    fn create_layout_info(&mut self, state: &State<ClientState>, mut store: ElementStoreMut, resolvers: &mut dyn Resolvers<ClientState>) {
        with_single_resolver(resolvers, |resolver| {
            state
                .get(&self.dialog_elements_path)
                .iter()
                .enumerate()
                .for_each(|(index, dialog_element)| {
                    // We only create this mutable reference for the lifetime of this scope, and
                    // since nothing is captured from the element this is safe.
                    let element = unsafe { &mut *dialog_element.element.get() };

                    element.create_layout_info(state, store.child_store(index as u64), resolver)
                });
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: korangar_interface::element::store::ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut korangar_interface::layout::WindowLayout<'a, ClientState>,
    ) {
        state
            .get(&self.dialog_elements_path)
            .iter()
            .enumerate()
            .for_each(|(index, dialog_element)| {
                // There are no mutable references at this point in time and the immutable
                // reference will be dropped after the interface is rendered, making this safe.
                let element = unsafe { &*dialog_element.element.get() };

                element.lay_out(state, store.child_store(index as u64), &(), layout)
            });
    }
}

/// A window representing a dialog with an NPC.
pub struct DialogWindow<A> {
    /// Path to the [`DialogWindowState`].
    window_state_path: A,
}

impl<A> DialogWindow<A> {
    /// Creates a new dialog window.
    ///
    /// This does not modify the [`DialogWindowState`].
    pub fn new(window_state_path: A) -> Self {
        Self { window_state_path }
    }
}

impl<A> CustomWindow<ClientState> for DialogWindow<A>
where
    A: Path<ClientState, DialogWindowState>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Dialog)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: client_state().localization().dialog_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            elements: (
                InnerElement {
                    dialog_elements_path: self.window_state_path.elements(),
                },
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DialogWindowState, parse_dialog_number};

    #[test]
    fn dialog_numbers_require_valid_i32_input() {
        assert_eq!(parse_dialog_number(" -42 "), Some(-42));
        assert_eq!(parse_dialog_number(""), None);
        assert_eq!(parse_dialog_number("-"), None);
        assert_eq!(parse_dialog_number("12x"), None);
        assert_eq!(parse_dialog_number("2147483648"), None);
    }

    #[test]
    fn input_after_next_starts_a_new_dialog_page() {
        let mut state = DialogWindowState::default();
        state.add_next_button();
        assert!(state.elements.iter().any(|element| element.is_next_button));

        state.add_number_input();

        assert!(!state.clear_next);
        assert!(!state.elements.iter().any(|element| element.is_next_button));
        assert!(state.elements.iter().all(|element| element.is_input));
    }
}
