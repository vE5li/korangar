mod slot;
mod state;
mod tabs;

use hashbrown::HashMap;
use korangar_interface::element::StateElement;
use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::{SkillId, SkillLevel};
use rust_state::{ManuallyAssertExt, Path, PathExt, RustState, State, VecIndexExt};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::interface::windows::skill_tree::state::AvailablePointsDisplay;
use crate::interface::windows::skill_tree::tabs::{DropSkillWrapper, DynamicTabs, SkillTreeTab, TabSelector};
use crate::state::localization::LocalizationPathExt;
use crate::state::skills::{LearnedSkill, SkillTabLayoutPathExt, SkillTreeLayout, SkillTreeLayoutPathExt};
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

/// Internal state of the skill tree window.
#[derive(Default, RustState, StateElement)]
pub struct SkillTreeWindowState {
    currently_skilling: bool,
    /// List of pending skill points. Each new point gets appended to this list,
    /// meaning the same skill id can be in the list multiple times. The
    /// total count of each skill id defines how many points to raise the
    /// skill level by.
    pending_skill_points: Vec<SkillId>,
    selected_tab: usize,
    highlighted_skill: Option<SkillId>,
    /// Map of skills that are not selected at their highest learned level. Only
    /// contains skills that have this property.
    #[hidden_element]
    chosen_skill_level: HashMap<SkillId, SkillLevel>,
}

pub struct SkillTreeWindow<A, B, C, D> {
    window_state_path: A,
    layout_path: B,
    skills_path: C,
    available_skill_points_path: D,
}

impl<A, B, C, D> SkillTreeWindow<A, B, C, D> {
    pub fn new(window_state_path: A, layout_path: B, skills_path: C, available_skill_points_path: D) -> Self {
        Self {
            window_state_path,
            layout_path,
            skills_path,
            available_skill_points_path,
        }
    }
}

impl<A, B, C, D> CustomWindow<ClientState> for SkillTreeWindow<A, B, C, D>
where
    A: Path<ClientState, SkillTreeWindowState>,
    B: Path<ClientState, SkillTreeLayout>,
    C: Path<ClientState, Vec<LearnedSkill>>,
    D: Path<ClientState, u32>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::SkillTree)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: client_state().localization().skill_tree_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            minimum_width: 550.0,
            maximum_width: 800.0,
            closable: true,
            elements: (
                split! {
                    gaps: theme().window().gaps(),
                    children: TabSelector::new(self.window_state_path.selected_tab(), self.layout_path),
                },
                DropSkillWrapper::new(
                    tabs! {
                        selected_tab: self.window_state_path.selected_tab(),
                        children: DynamicTabs::new(move |index|
                            SkillTreeTab::new(
                                self.layout_path.tabs().index(index).skills().manually_asserted(),
                                self.skills_path,
                                self.window_state_path,
                            )
                        )
                    }
                ),
                split! {
                    children: (
                        text! {
                            text: AvailablePointsDisplay::new(
                                client_state().localization().available_skill_points_text(),
                                self.available_skill_points_path,
                                self.window_state_path.pending_skill_points(),
                            ),
                            height: 20.0,
                        },
                        either! {
                            selector: self.window_state_path.currently_skilling(),
                            on_true: split! {
                                gaps: theme().window().gaps(),
                                children: (
                                    button! {
                                        text: client_state().localization().reset_skill_points_button_text(),
                                        event: move |state: &State<ClientState>, _: &mut EventQueue<ClientState>| {
                                            state.update_value_with(self.window_state_path.pending_skill_points(), Vec::clear);
                                        },
                                    },
                                    button! {
                                        text: client_state().localization().cancel_skill_points_button_text(),
                                        event: move |state: &State<ClientState>, _: &mut EventQueue<ClientState>| {
                                            state.update_value_with(self.window_state_path.pending_skill_points(), Vec::clear);
                                            state.update_value(self.window_state_path.currently_skilling(), false);
                                        },
                                    },
                                    button! {
                                        text: client_state().localization().apply_skill_points_button_text(),
                                        disabled: ComputedSelector::new_default(move |state: &ClientState| {
                                            self.window_state_path.pending_skill_points().follow_safe(state).is_empty()
                                        }),
                                        event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                            let pending_skill_points_path = self.window_state_path.pending_skill_points();
                                            let pending_skill_points = state.get(&pending_skill_points_path);
                                            let skill_ids = pending_skill_points.clone();

                                            state.update_value(self.window_state_path.currently_skilling(), false);
                                            state.update_value_with(pending_skill_points_path, Vec::clear);

                                            queue.queue(InputEvent::LevelUpSkills { skill_ids });
                                        },
                                    },
                                ),
                            },
                            on_false: button! {
                                text: client_state().localization().distribute_skill_points_button_text(),
                                disabled: ComputedSelector::new_default(move |state: &ClientState| {
                                    *self.available_skill_points_path.follow_safe(state) == 0
                                }),
                                event: SetToTrue(self.window_state_path.currently_skilling()),
                            },
                        },
                    )
                }
            ),
        }
    }
}
