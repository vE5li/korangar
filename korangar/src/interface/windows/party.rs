use std::cmp::Ordering;

use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox, StateElement};
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::{PartyMember, PartyMemberPathExt};
use rust_state::{ManuallyAssertExt, Path, PathExt, RustState, Selector, State, VecIndexExt};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

// TODO: These constants are duplicated troughout the code base. Unify this
// somewhere, maybe a `consts.rs` would be a good idea at this point?
const MINIMUM_NAME_LENGTH: usize = 4;
const MAXIMUM_NAME_LENGTH: usize = 24;

struct PartyMemberList<A> {
    members_path: A,
    elements: Vec<ElementBox<ClientState>>,
}

impl<A> PartyMemberList<A> {
    fn new(members_path: A) -> Self {
        Self {
            members_path,
            elements: Vec::new(),
        }
    }
}

impl<A> Element<ClientState> for PartyMemberList<A>
where
    A: Path<ClientState, Vec<PartyMember>>,
{
    type LayoutInfo = ();

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            use korangar_interface::prelude::*;

            let members = state.get(&self.members_path);

            match members.len().cmp(&self.elements.len()) {
                Ordering::Less => {
                    self.elements.truncate(members.len());
                }
                Ordering::Equal => {}
                Ordering::Greater => {
                    for index in self.elements.len()..members.len() {
                        let member_path = self.members_path.index(index).manually_asserted();
                        let name_path = member_path.name();

                        self.elements.push(ErasedElement::new(collapsible! {
                            text: name_path,
                            children: button! {
                                text: client_state().localization().party_expel_button_text(),
                                event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                    let member = state.get(&member_path);
                                    let account_id = member.account_id;
                                    let player_name = member.name.clone();

                                    queue.queue(
                                        InputEvent::ExpelPartyMember { account_id, player_name }
                                    );
                                },
                            },
                        }));
                    }
                }
            }

            self.elements.iter_mut().zip(members.iter()).for_each(|(element, member)| {
                element.create_layout_info(state, store.child_store(member.character_id.0 as u64), resolver);
            });
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let members = state.get(&self.members_path);

        self.elements.iter().zip(members.iter()).for_each(|(element, member)| {
            element.lay_out(state, store.child_store(member.character_id.0 as u64), &(), layout);
        });
    }
}

/// Selects whether the player is currently in a party, based on the party
/// name being non-empty.
struct InPartySelector<P> {
    party_name_path: P,
}

impl<P> InPartySelector<P> {
    fn new(party_name_path: P) -> Self {
        Self { party_name_path }
    }
}

impl<P> Selector<ClientState, bool> for InPartySelector<P>
where
    P: Path<ClientState, String>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a bool> {
        match self.party_name_path.follow_safe(state).is_empty() {
            true => Some(&false),
            false => Some(&true),
        }
    }
}

/// Internal state of the party window.
#[derive(Default, RustState, StateElement)]
pub struct PartyWindowState {
    currently_creating: String,
    currently_inviting: String,
}

pub struct PartyWindow<A, B, C> {
    window_state_path: A,
    party_name_path: B,
    members_path: C,
}

impl<A, B, C> PartyWindow<A, B, C> {
    pub fn new(window_state_path: A, party_name_path: B, members_path: C) -> Self {
        Self {
            window_state_path,
            party_name_path,
            members_path,
        }
    }
}

impl<A, B, C> CustomWindow<ClientState> for PartyWindow<A, B, C>
where
    A: Path<ClientState, PartyWindowState>,
    B: Path<ClientState, String>,
    C: Path<ClientState, Vec<PartyMember>>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Party)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        struct CreatePartyTextBox;
        struct InvitePlayerTextBox;

        let create_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let party_name = state.get(&self.window_state_path.currently_creating()).clone();

            // TODO: Give some sort of error if the name is too short.
            if party_name.len() >= MINIMUM_NAME_LENGTH {
                state.update_value_with(self.window_state_path.currently_creating(), |input| input.clear());
                queue.queue(InputEvent::CreateParty { party_name });
                queue.queue(Event::Unfocus);
            }
        };

        let invite_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let player_name = state.get(&self.window_state_path.currently_inviting()).clone();

            // TODO: Give some sort of error if the name is too short.
            if player_name.len() >= MINIMUM_NAME_LENGTH {
                state.update_value_with(self.window_state_path.currently_inviting(), |input| input.clear());
                queue.queue(InputEvent::InvitePlayerToParty { player_name });
                queue.queue(Event::Unfocus);
            }
        };

        window! {
            title: client_state().localization().party_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                either! {
                    selector: InPartySelector::new(self.party_name_path),
                    on_true: fragment! {
                        gaps: 4.0,
                        children: (
                            text! {
                                text: self.party_name_path,
                            },
                            text_box! {
                                ghost_text: client_state().localization().party_invite_text_box_message(),
                                state: self.window_state_path.currently_inviting(),
                                input_handler: DefaultHandler::<_, _, MAXIMUM_NAME_LENGTH>::new(self.window_state_path.currently_inviting(), invite_action),
                                focus_id: InvitePlayerTextBox,
                            },
                            PartyMemberList::new(self.members_path),
                            button! {
                                text: client_state().localization().party_leave_button_text(),
                                event: InputEvent::LeaveParty,
                            },
                        ),
                    },
                    on_false: text_box! {
                        ghost_text: client_state().localization().party_create_text_box_message(),
                        state: self.window_state_path.currently_creating(),
                        input_handler: DefaultHandler::<_, _, MAXIMUM_NAME_LENGTH>::new(self.window_state_path.currently_creating(), create_action),
                        focus_id: CreatePartyTextBox,
                    },
                },
            )
        }
    }
}
