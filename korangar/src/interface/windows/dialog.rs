use std::cell::UnsafeCell;

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

        self.elements.retain(|element| !element.is_next_button);

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

        self.elements.retain(|element| !element.is_next_button);

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
