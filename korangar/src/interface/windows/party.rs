use std::cmp::Ordering;

use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox, StateElement};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{ManuallyAssertExt, Path, PathExt, RustState, Selector, State, VecIndexExt};

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, PartyMemberState, PartyMemberStatePathExt, client_state, this_player};

// TODO: These constants are duplicated troughout the code base. Unify this
// somewhere, maybe a `consts.rs` would be a good idea at this point?
const MINIMUM_NAME_LENGTH: usize = 4;
const MAXIMUM_NAME_LENGTH: usize = 24;

const HEALTH_BAR_HEIGHT: f32 = 14.0;

/// Returns the health of a party member. The server does not send us health
/// updates about our own character through the party packets, so the health
/// of the local player is taken from the player entity instead.
fn member_health(state: &State<ClientState>, member: &PartyMemberState) -> (u32, u32) {
    if let Some(player) = state.try_follow(this_player()) {
        let common = player.get_common();

        if common.entity_id.0 == member.account_id.0 {
            return (common.health_points as u32, common.maximum_health_points as u32);
        }
    }

    (member.health_points, member.maximum_health_points)
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

/// Selects whether the expel button of a party member should be shown. Only
/// the party leader can expel members, and members cannot expel themselves.
struct CanExpelSelector<P> {
    members_path: P,
    member_index: usize,
}

impl<P> CanExpelSelector<P> {
    fn new(members_path: P, member_index: usize) -> Self {
        Self {
            members_path,
            member_index,
        }
    }
}

impl<P> Selector<ClientState, bool> for CanExpelSelector<P>
where
    P: Path<ClientState, Vec<PartyMemberState>>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a bool> {
        let members = self.members_path.follow(state)?;
        let member = members.get(self.member_index)?;

        let local_player_id = this_player().follow(state).map(|player| player.get_common().entity_id.0);

        let local_player_is_leader =
            local_player_id.is_some_and(|player_id| members.iter().any(|member| member.is_leader && member.account_id.0 == player_id));
        let member_is_local_player = local_player_id.is_some_and(|player_id| member.account_id.0 == player_id);

        match local_player_is_leader && !member_is_local_player {
            true => Some(&true),
            false => Some(&false),
        }
    }
}

struct HealthBarLayoutInfo {
    area: Area,
    fill_ratio: f32,
    health_known: bool,
    is_online: bool,
}

/// A bar displaying the health of a party member.
struct HealthBar<A> {
    member_path: A,
}

impl<A> HealthBar<A> {
    fn new(member_path: A) -> Self {
        Self { member_path }
    }
}

impl<A> Element<ClientState> for HealthBar<A>
where
    A: Path<ClientState, PartyMemberState>,
{
    type LayoutInfo = HealthBarLayoutInfo;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let member = state.get(&self.member_path);
            let (health_points, maximum_health_points) = member_health(state, member);

            let health_known = maximum_health_points > 0;
            let fill_ratio = match health_known {
                true => (health_points as f32 / maximum_health_points as f32).clamp(0.0, 1.0),
                false => 0.0,
            };

            HealthBarLayoutInfo {
                area: resolver.with_height(HEALTH_BAR_HEIGHT),
                fill_ratio,
                health_known,
                is_online: member.is_online,
            }
        })
    }

    fn lay_out<'a>(
        &'a self,
        _: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        layout.add_rectangle(
            layout_info.area,
            CornerDiameter::uniform(4.0),
            Color::rgb_u8(40, 40, 40),
            Color::rgba_u8(0, 0, 0, 100),
            ShadowPadding::diagonal(1.0, 2.0),
        );

        if layout_info.health_known && layout_info.is_online && layout_info.fill_ratio > 0.0 {
            let fill_area = Area {
                left: layout_info.area.left + 1.0,
                top: layout_info.area.top + 1.0,
                width: (layout_info.area.width - 2.0) * layout_info.fill_ratio,
                height: layout_info.area.height - 2.0,
            };

            let fill_color = match layout_info.fill_ratio {
                ratio if ratio > 0.5 => Color::rgb_u8(70, 180, 90),
                ratio if ratio > 0.25 => Color::rgb_u8(220, 160, 40),
                _ => Color::rgb_u8(200, 60, 50),
            };

            layout.add_rectangle(
                fill_area,
                CornerDiameter::uniform(3.0),
                fill_color,
                Color::rgba_u8(0, 0, 0, 0),
                ShadowPadding::uniform(0.0),
            );
        }
    }
}

/// A cached text line with the level, health, and position of a party member.
struct MemberInfoText<A> {
    member_path: A,
    cached_values: Option<(u16, u32, u32, u16, u16, bool)>,
    text: String,
}

impl<A> MemberInfoText<A> {
    fn new(member_path: A) -> Self {
        Self {
            member_path,
            cached_values: None,
            text: String::new(),
        }
    }
}

impl<A> Element<ClientState> for MemberInfoText<A>
where
    A: Path<ClientState, PartyMemberState>,
{
    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        use korangar_interface::prelude::*;

        with_single_resolver(resolvers, |resolver| {
            let member = state.get(&self.member_path);
            let (health_points, maximum_health_points) = member_health(state, member);

            let values = (
                member.level,
                health_points,
                maximum_health_points,
                member.x,
                member.y,
                member.is_online,
            );

            if self.cached_values != Some(values) {
                self.text = match member.is_online {
                    false => format!("Lv {} - {} - Offline", member.level, member.map_name),
                    true => {
                        let health_text = match maximum_health_points > 0 {
                            true => format!("{health_points} / {maximum_health_points} HP"),
                            false => "HP unknown".to_owned(),
                        };

                        let position_text = match member.x > 0 || member.y > 0 {
                            true => format!(" - {} ({}, {})", member.map_name, member.x, member.y),
                            false => format!(" - {}", member.map_name),
                        };

                        format!("Lv {} - {health_text}{position_text}", member.level)
                    }
                };
                self.cached_values = Some(values);
            }

            let height = *state.get(&theme().text().height());
            let font_size = *state.get(&theme().text().font_size());
            let color = *state.get(&theme().text().color());
            let highlight_color = *state.get(&theme().text().highlight_color());
            let horizontal_alignment = *state.get(&theme().text().horizontal_alignment());
            let overflow_behavior = *state.get(&theme().text().overflow_behavior());

            let (size, font_size) = resolver.get_text_dimensions(
                &self.text,
                color,
                highlight_color,
                font_size,
                horizontal_alignment,
                overflow_behavior,
            );
            let area = resolver.with_height(height.max(size.height));

            Self::LayoutInfo { area, font_size }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        use korangar_interface::prelude::*;

        layout.add_text(
            layout_info.area,
            &self.text,
            layout_info.font_size,
            *state.get(&theme().text().color()),
            *state.get(&theme().text().highlight_color()),
            *state.get(&theme().text().horizontal_alignment()),
            *state.get(&theme().text().vertical_alignment()),
            *state.get(&theme().text().overflow_behavior()),
        );
    }
}

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
    A: Path<ClientState, Vec<PartyMemberState>>,
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
                            children: (
                                HealthBar::new(member_path),
                                MemberInfoText::new(member_path),
                                either! {
                                    selector: CanExpelSelector::new(self.members_path, index),
                                    on_true: button! {
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
                                    on_false: (),
                                },
                            ),
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
    C: Path<ClientState, Vec<PartyMemberState>>,
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
