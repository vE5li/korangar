use std::cell::{Cell, UnsafeCell};

use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::StatUpType;
use rust_state::{Path, Selector};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};
use crate::world::{Player, PlayerPathExt};

struct StatTextSelector<A> {
    bonus_path: A,
    last_value: Cell<Option<i32>>,
    text: UnsafeCell<String>,
}

impl<A> StatTextSelector<A> {
    pub fn new(bonus_path: A) -> Self {
        Self {
            bonus_path,
            last_value: Cell::default(),
            text: UnsafeCell::default(),
        }
    }
}

impl<A> Selector<ClientState, String> for StatTextSelector<A>
where
    A: Path<ClientState, i32>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a String> {
        // SAFETY
        // `unnwrap` is safe here because the bound of `A` specifies a safe path.
        let bonus_value = self.bonus_path.follow(state).unwrap();

        unsafe {
            let last_value = self.last_value.get();

            if last_value.is_none() || last_value.as_ref().is_some_and(|last| *last != *bonus_value) {
                *self.text.get() = format!("^000001{bonus_value:+}^000000");
                self.last_value.set(Some(*bonus_value));
            }
        }

        unsafe { Some(self.text.as_ref_unchecked()) }
    }
}

struct CostTextSelector<A> {
    cost_path: A,
    last_value: UnsafeCell<Option<u8>>,
    text: UnsafeCell<String>,
}

impl<A> CostTextSelector<A> {
    pub fn new(cost_path: A) -> Self {
        Self {
            cost_path,
            last_value: UnsafeCell::default(),
            text: UnsafeCell::default(),
        }
    }
}

impl<A> Selector<ClientState, String> for CostTextSelector<A>
where
    A: Path<ClientState, u8>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a String> {
        // SAFETY
        // `unnwrap` is safe here because the bound of `A` specifies a safe path.
        let cost = self.cost_path.follow(state).unwrap();

        unsafe {
            let last_value = &mut *self.last_value.get();

            if last_value.is_none() || last_value.as_ref().is_some_and(|last| *last != *cost) {
                *self.text.get() = match *cost {
                    0 => "^000001max^000000".to_string(),
                    cost => format!("+1 (^000001{cost}^000000)"),
                };

                *last_value = Some(*cost);
            }
        }

        unsafe { Some(self.text.as_ref_unchecked()) }
    }
}

#[derive(Default)]
pub struct StatsWindow<A> {
    player_path: A,
}

impl<A> StatsWindow<A> {
    pub fn new(player_path: A) -> Self {
        Self { player_path }
    }
}

impl<A> CustomWindow<ClientState> for StatsWindow<A>
where
    A: Path<ClientState, Player>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Stats)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        fn disabled_cutoff<A, B>(stat_points_path: A, cost_path: B) -> impl Selector<ClientState, bool>
        where
            A: Path<ClientState, u32>,
            B: Path<ClientState, u8>,
        {
            ComputedSelector::new_default(move |state: &ClientState| {
                // SAFETY:
                //
                // Unwrap is safe here because of the bounds.
                let stat_points = stat_points_path.follow(state).unwrap();

                // SAFETY:
                //
                // Unwrap is safe here because of the bounds.
                let cost = cost_path.follow(state).unwrap();

                // The cost is 0 if the the player is at the maximum level.
                *cost == 0 || *stat_points < *cost as u32
            })
        }

        macro_rules! stat_row {
            ($text_name:expr, $name:ident, $bonus_name:ident, $cost_name:ident, $variant_name:ident) => {
                split! {
                    children: (
                        text! {
                            text: client_state().localization().$text_name(),
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        split! {
                            children: (
                                text! {
                                    text: PartialEqDisplaySelector::new(self.player_path.$name()),
                                    horizontal_alignment: HorizontalAlignment::Right { offset: 5.0, border: 5.0 },
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                                text! {
                                    text: StatTextSelector::new(self.player_path.$bonus_name()),
                                    horizontal_alignment: HorizontalAlignment::Left { offset: 5.0, border: 5.0 },
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                            ),
                        },
                        button! {
                            text: CostTextSelector::new(self.player_path.$cost_name()),
                            disabled: disabled_cutoff(self.player_path.stat_points(), self.player_path.$cost_name()),
                            event: InputEvent::StatUp { stat_type: StatUpType::$variant_name { amount: 1 } },
                        },
                    ),
                }
            };
        }

        window! {
            title: client_state().localization().stats_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                split! {
                    children: (
                        text! {
                            text: client_state().localization().available_stat_points_text(),
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        text! {
                            text: PartialEqDisplaySelector::new(self.player_path.stat_points()),
                            horizontal_alignment: HorizontalAlignment::Right { offset: 5.0, border: 5.0 },
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                    ),
                },
                stat_row!(strength_text, strength, bonus_strength, strength_stat_points_cost, Strength),
                stat_row!(agility_text, agility, bonus_agility, agility_stat_points_cost, Agility),
                stat_row!(vitality_text, vitality, bonus_vitality, vitality_stat_points_cost, Vitality),
                stat_row!(intelligence_text, intelligence, bonus_intelligence, intelligence_stat_points_cost, Intelligence),
                stat_row!(dexterity_text, dexterity, bonus_dexterity, dexterity_stat_points_cost, Dexterity),
                stat_row!(luck_text, luck, bonus_luck, luck_stat_points_cost, Luck),
            ),
        }
    }
}
