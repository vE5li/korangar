use std::cell::UnsafeCell;

use hashbrown::HashMap;
use ragnarok_packets::SkillId;
use rust_state::{Path, PathExt, Selector, SelectorExt};

use crate::state::ClientState;
use crate::state::skills::LearnableSkill;

#[derive(Default)]
struct AvailablePointsDisplayState {
    available_skill_points: Option<u32>,
    string: String,
}

pub(super) struct AvailablePointsDisplay<A, B, C> {
    label_path: A,
    available_skill_points_path: B,
    pending_skill_points_path: C,
    state: UnsafeCell<AvailablePointsDisplayState>,
}

impl<A, B, C> AvailablePointsDisplay<A, B, C> {
    pub fn new(label_path: A, available_skill_points_path: B, pending_skill_points_path: C) -> Self {
        Self {
            label_path,
            available_skill_points_path,
            pending_skill_points_path,
            state: UnsafeCell::default(),
        }
    }
}

impl<A, B, C> Selector<ClientState, String> for AvailablePointsDisplay<A, B, C>
where
    A: Selector<ClientState, String>,
    B: Selector<ClientState, u32>,
    C: Selector<ClientState, Vec<SkillId>>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a String> {
        let label = self.label_path.select_safe(state);
        let available_skill_points = self
            .available_skill_points_path
            .select_safe(state)
            .saturating_sub(self.pending_skill_points_path.select_safe(state).len() as u32);

        // SAFETY:
        //
        // We are only creating an immutable reference to check if the state is in sync.
        // This reference is dropped before creating any mutable reference.
        let state = unsafe { &*self.state.get() };

        // Technically, this will lead to buggy behavior when changing the prefix to a
        // shorter version of the previous prefix. E.g. "foobar" -> "foob" will
        // not trigger the string to be recreated.
        //
        // I don't think this edge case will ever be observed by a user.
        if state.available_skill_points.is_none_or(|cached| cached != available_skill_points) || !state.string.starts_with(label) {
            // Make sure the reference is invalidated.
            let _ = state;

            // SAFETY:
            //
            // We only ever modify the state here and only if the state is out-of-sync with
            // the global state. Since the global state can't change during the building
            // or rendering of the interface, only the first call to `select, will get to
            // this scope, thus no immutable references have been handed out yet and it is
            // sound to create a mutable references.
            let state_mut = unsafe { &mut *self.state.get() };

            state_mut.string = match available_skill_points > 0 {
                true => format!("{label}: ^000001{available_skill_points}"),
                false => format!("{label}: {available_skill_points}"),
            };

            state_mut.available_skill_points = Some(available_skill_points);
        }

        // SAFETY:
        //
        // No further mutation will happen on this immutable reference, so it's sound to
        // return it.
        let state = unsafe { &*self.state.get() };

        Some(&state.string)
    }
}

pub(super) struct LearnableSkillPath<A> {
    layout_path: A,
    index: usize,
}

impl<A> LearnableSkillPath<A> {
    pub fn new(layout_path: A, index: usize) -> Self {
        Self { layout_path, index }
    }
}

impl<A> Copy for LearnableSkillPath<A> where A: Copy {}

impl<A> Clone for LearnableSkillPath<A>
where
    A: Clone,
{
    fn clone(&self) -> Self {
        Self {
            layout_path: self.layout_path.clone(),
            index: self.index,
        }
    }
}

impl<A> Path<ClientState, LearnableSkill, false> for LearnableSkillPath<A>
where
    A: Path<ClientState, HashMap<usize, LearnableSkill>>,
{
    fn follow<'a>(&self, state: &'a ClientState) -> Option<&'a LearnableSkill> {
        self.layout_path.follow_safe(state).get(&self.index)
    }

    fn follow_mut<'a>(&self, state: &'a mut ClientState) -> Option<&'a mut LearnableSkill> {
        self.layout_path.follow_mut_safe(state).get_mut(&self.index)
    }
}

impl<A> Selector<ClientState, LearnableSkill, false> for LearnableSkillPath<A>
where
    A: Path<ClientState, HashMap<usize, LearnableSkill>>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a LearnableSkill> {
        self.follow(state)
    }
}
