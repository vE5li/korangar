use std::marker::PhantomData;

use rust_state::{Selector, State};

use crate::application::Application;
use crate::element::Element;
use crate::element::store::{ElementStore, ElementStoreMut};
use crate::layout::{Resolvers, WindowLayout};

pub trait TabsChildren<App: Application> {
    type Key;
    type LayoutInfo;

    fn get_tab(&self, key: &Self::Key) -> &impl Element<App, LayoutInfo = Self::LayoutInfo>;

    fn get_tab_mut(&mut self, key: &Self::Key) -> &mut impl Element<App, LayoutInfo = Self::LayoutInfo>;
}

pub struct Tabs<T, A, Children> {
    selected_tab: A,
    children: Children,
    _maker: PhantomData<T>,
}

impl<T, A, Children> Tabs<T, A, Children> {
    /// This function is supposed to be called from a component macro and not
    /// intended to be called manually.
    #[inline(always)]
    pub fn component_new(selected_tab: A, children: Children) -> Self {
        Self {
            selected_tab,
            children,
            _maker: PhantomData,
        }
    }
}

impl<App, T, A, Children> Element<App> for Tabs<T, A, Children>
where
    App: Application,
    A: Selector<App, T>,
    Children: TabsChildren<App, Key = T>,
{
    type LayoutInfo = Children::LayoutInfo;

    fn create_layout_info(&mut self, state: &State<App>, store: ElementStoreMut, resolvers: &mut dyn Resolvers<App>) -> Self::LayoutInfo {
        let selected_tab = state.get(&self.selected_tab);
        self.children.get_tab_mut(selected_tab).create_layout_info(state, store, resolvers)
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<App>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, App>,
    ) {
        let selected_tab = state.get(&self.selected_tab);
        self.children.get_tab(selected_tab).lay_out(state, store, layout_info, layout)
    }
}
