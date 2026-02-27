use std::cmp::Ordering;

use hashbrown::HashMap;
use korangar_interface::MouseMode;
use korangar_interface::components::tabs::TabsChildren;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox};
use korangar_interface::event::{DropHandler, EventQueue};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{Resolvers, WindowLayout, with_nth_resolver, with_single_resolver};
use rust_state::{ManuallyAssertExt, Path, PathExt, State, VecIndexExt};

use crate::input::{InputEvent, MouseInputMode};
use crate::interface::resource::SkillSource;
use crate::interface::windows::SkillTreeWindowState;
use crate::interface::windows::skill_tree::slot::SkillSlot;
use crate::interface::windows::skill_tree::state::LearnableSkillPath;
use crate::state::skills::{
    LearnableSkill, LearnedSkill, LearnedSkillPath, SkillTabLayoutPathExt, SkillTreeLayout, SkillTreeLayoutPathExt,
};
use crate::state::theme::{GlobalThemePathExt, InterfaceThemePathExt};
use crate::state::{ClientState, client_theme};

const SKILL_TREE_COLUMNS: usize = 7;

pub(super) struct TabSelector<A, B> {
    selected_tab_path: A,
    layout_path: B,
    buttons: Vec<ElementBox<ClientState>>,
}

impl<A, B> TabSelector<A, B> {
    pub fn new(selected_tab_path: A, layout_path: B) -> Self {
        Self {
            selected_tab_path,
            layout_path,
            buttons: Vec::new(),
        }
    }
}

impl<A, B> Element<ClientState> for TabSelector<A, B>
where
    A: Path<ClientState, usize>,
    B: Path<ClientState, SkillTreeLayout>,
{
    type LayoutInfo = ();

    fn get_element_count(&self, state: &State<ClientState>) -> usize {
        state.get(&self.layout_path).tabs.len()
    }

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        use korangar_interface::prelude::*;

        let layout = state.get(&self.layout_path);
        let tab_count = layout.tabs.len();

        match self.buttons.len().cmp(&tab_count) {
            Ordering::Less => {
                let selected_tab_path = self.selected_tab_path;

                for tab_index in self.buttons.len()..tab_count {
                    self.buttons.push(ErasedElement::new(button! {
                    text: self.layout_path.tabs().index(tab_index).name().manually_asserted(),
                    event: move |state: &rust_state::State<ClientState>, _: &mut korangar_interface::event::EventQueue<ClientState>| {
                        state.update_value(selected_tab_path, tab_index);
                    },
                    disabled: ComputedSelector::new_default(move |state: &ClientState| *selected_tab_path.follow_safe(state) == tab_index),
                }));
                }
            }
            Ordering::Greater => self.buttons.truncate(tab_count),
            Ordering::Equal => {}
        }

        self.buttons.iter_mut().enumerate().for_each(|(index, element)| {
            with_nth_resolver(resolvers, index, |resolver| {
                element.create_layout_info(state, store.child_store(index as u64), resolver)
            })
        });
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        self.buttons
            .iter()
            .enumerate()
            .for_each(|(index, element)| element.lay_out(state, store.child_store(index as u64), &(), layout));
    }
}

pub(super) struct SkillTreeTab<A, B, C> {
    layout_path: A,
    skills_path: B,
    window_state_path: C,
    row_elements: Vec<ElementBox<ClientState>>,
}

impl<A, B, C> SkillTreeTab<A, B, C> {
    pub fn new(layout_path: A, skills_path: B, window_state_path: C) -> Self {
        Self {
            layout_path,
            skills_path,
            window_state_path,
            row_elements: Vec::new(),
        }
    }
}

impl<A, B, C> Element<ClientState> for SkillTreeTab<A, B, C>
where
    A: Path<ClientState, HashMap<usize, LearnableSkill>>,
    B: Path<ClientState, Vec<LearnedSkill>>,
    C: Path<ClientState, SkillTreeWindowState>,
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

            let layout = state.get(&self.layout_path);

            let row_count = 1 + layout.keys().max().copied().unwrap_or_default() / SKILL_TREE_COLUMNS;

            match self.row_elements.len().cmp(&row_count) {
                Ordering::Less => {
                    for row in self.row_elements.len()..row_count {
                        self.row_elements.push(ErasedElement::new(split! {
                            gaps: theme().window().gaps(),
                            children: std::array::from_fn::<_, SKILL_TREE_COLUMNS, _>(|column| {
                                let learnable_skill_path =  LearnableSkillPath::new(
                                    self.layout_path,
                                    row * SKILL_TREE_COLUMNS + column,
                                );
                                let learned_skill_path = LearnedSkillPath::new(learnable_skill_path, self.skills_path);

                                SkillSlot::new(
                                    learnable_skill_path,
                                    learned_skill_path,
                                    self.window_state_path,
                                    SkillSource::SkillTree,
                                )
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
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        self.row_elements
            .iter()
            .enumerate()
            .for_each(|(index, element)| element.lay_out(state, store.child_store(index as u64), &(), layout));
    }
}

pub(super) struct DropSkillWrapper<Children> {
    children: Children,
}

impl<Children> DropSkillWrapper<Children> {
    pub fn new(children: Children) -> Self {
        Self { children }
    }
}

impl<Children> DropHandler<ClientState> for DropSkillWrapper<Children> {
    fn handle_drop(&self, _: &State<ClientState>, queue: &mut EventQueue<ClientState>, mouse_mode: &MouseMode<ClientState>) {
        if let MouseMode::Custom {
            mode: MouseInputMode::MoveSkill { source, skill },
        } = mouse_mode
        {
            queue.queue(InputEvent::MoveSkill {
                source: *source,
                destination: SkillSource::SkillTree,
                skill: skill.clone(),
            });
        }
    }
}

impl<Children> Element<ClientState> for DropSkillWrapper<Children>
where
    Children: Element<ClientState>,
{
    type LayoutInfo = (Area, Children::LayoutInfo);

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            resolver.with_derived_unchanged(|resolver| self.children.create_layout_info(state, store, resolver))
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        use korangar_interface::prelude::*;

        if let MouseMode::Custom {
            mode: MouseInputMode::MoveSkill { source, .. },
        } = layout.get_mouse_mode()
            && *source != SkillSource::SkillTree
        {
            let is_hovered = layout_info.0.check().any_mouse_mode().run(layout);
            let color = match is_hovered {
                true => *state.get(&client_theme().global().hovered_drop_area_color()),
                false => *state.get(&client_theme().global().drop_area_color()),
            };

            layout.add_rectangle(
                layout_info.0,
                *state.get(&client_theme().window().corner_diameter()),
                color.multiply_alpha(*state.get(&client_theme().global().fill_alpha())),
                color,
                *state.get(&client_theme().global().drop_area_outline()),
            );

            if is_hovered {
                // Since we are not in default mouse mode we need to mark the window as
                // hovered.
                layout.set_hovered();

                layout.register_drop_handler(self);
            }
        }

        self.children.lay_out(state, store, &layout_info.1, layout);
    }
}

pub(super) struct DynamicTabs<F, Children> {
    new_tab: F,
    tabs: Vec<Children>,
}

impl<F, Children> DynamicTabs<F, Children>
where
    F: Fn(usize) -> Children,
{
    pub fn new(new_tab: F) -> DynamicTabs<F, Children> {
        Self { new_tab, tabs: Vec::new() }
    }
}

impl<F, Children> TabsChildren<ClientState> for DynamicTabs<F, Children>
where
    F: Fn(usize) -> Children,
    Children: Element<ClientState, LayoutInfo = ()>,
{
    type Key = usize;
    type LayoutInfo = ();

    fn get_tab(&self, key: &Self::Key) -> &impl Element<ClientState, LayoutInfo = Self::LayoutInfo> {
        &self.tabs[*key]
    }

    fn get_tab_mut(&mut self, key: &Self::Key) -> &mut impl Element<ClientState, LayoutInfo = Self::LayoutInfo> {
        while *key >= self.tabs.len() {
            let new_tab = (self.new_tab)(self.tabs.len());
            self.tabs.push(new_tab);
        }

        &mut self.tabs[*key]
    }
}
