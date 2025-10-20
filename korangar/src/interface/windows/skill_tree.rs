use std::cmp::Ordering;

use hashbrown::HashMap;
use korangar_components::skill_box;
use korangar_interface::element::{Element, ElementBox};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, Selector};

use crate::SkillSource;
use crate::interface::windows::WindowClass;
use crate::inventory::{LearnableSkill, LearnedSkill, LearnedSkillPath};
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

const SKILL_TREE_COLUMNS: usize = 7;

struct LearnableSkillPath<A> {
    layout_path: A,
    index: usize,
}

impl<A> LearnableSkillPath<A> {
    fn new(layout_path: A, index: usize) -> Self {
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
        // SAFETY:
        // Unwrapping is safe because of the bounds
        self.layout_path.follow(state).unwrap().get(&self.index)
    }

    fn follow_mut<'a>(&self, state: &'a mut ClientState) -> Option<&'a mut LearnableSkill> {
        // SAFETY:
        // Unwrapping is safe because of the bounds
        self.layout_path.follow_mut(state).unwrap().get_mut(&self.index)
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

struct SkillTree<A, B> {
    layout_path: A,
    skills_path: B,
    row_elements: Vec<ElementBox<ClientState>>,
}

impl<A, B> SkillTree<A, B> {
    fn new(layout_path: A, skills_path: B) -> Self {
        Self {
            layout_path,
            skills_path,
            row_elements: Vec::new(),
        }
    }
}

impl<A, B> Element<ClientState> for SkillTree<A, B>
where
    A: Path<ClientState, HashMap<usize, LearnableSkill>>,
    B: Path<ClientState, Vec<LearnedSkill>>,
{
    type LayoutInfo = ();

    fn create_layout_info(
        &mut self,
        state: &rust_state::Context<ClientState>,
        mut store: korangar_interface::element::store::ElementStoreMut<'_>,
        resolver: &mut korangar_interface::layout::Resolver<'_, ClientState>,
    ) -> Self::LayoutInfo {
        use korangar_interface::prelude::*;

        let layout = state.get(&self.layout_path);

        let row_count = 1 + layout.keys().max().copied().unwrap_or_default() / SKILL_TREE_COLUMNS;

        match self.row_elements.len().cmp(&row_count) {
            Ordering::Less => {
                for row in self.row_elements.len()..row_count {
                    self.row_elements.push(ErasedElement::new(split! {
                        gaps: theme().window().gaps(),
                        children: std::array::from_fn::<_, SKILL_TREE_COLUMNS, _>(|column| {
                            let learnable_skill_path =  LearnableSkillPath::new(self.layout_path, row * SKILL_TREE_COLUMNS + column);
                            let learned_skill_path = LearnedSkillPath::new(learnable_skill_path, self.skills_path);

                            skill_box! {
                                learnable_skill_path,
                                learned_skill_path,
                                source: SkillSource::SkillTree,
                            }
                        }),
                    }));
                }
            }
            Ordering::Greater => self.row_elements.truncate(row_count),
            Ordering::Equal => {}
        }

        self.row_elements
            .iter_mut()
            .enumerate()
            .for_each(|(index, element)| element.create_layout_info(state, store.child_store(index as u64), resolver));
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a rust_state::Context<ClientState>,
        store: korangar_interface::element::store::ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut korangar_interface::layout::WindowLayout<'a, ClientState>,
    ) {
        self.row_elements
            .iter()
            .enumerate()
            .for_each(|(index, element)| element.lay_out(state, store.child_store(index as u64), &(), layout));
    }
}

pub struct SkillTreeWindow<A, B> {
    layout_path: A,
    skills_path: B,
}

impl<A, B> SkillTreeWindow<A, B> {
    pub fn new(layout_path: A, skills_path: B) -> Self {
        Self { layout_path, skills_path }
    }
}

impl<A, B> CustomWindow<ClientState> for SkillTreeWindow<A, B>
where
    A: Path<ClientState, HashMap<usize, LearnableSkill>>,
    B: Path<ClientState, Vec<LearnedSkill>>,
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
            closable: true,
            elements: (SkillTree::new(self.layout_path, self.skills_path),),
        }
    }
}
