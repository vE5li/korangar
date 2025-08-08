use std::cmp::Ordering;

use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox};
use korangar_interface::layout::{Layout, Resolver};
use korangar_interface::window::{CustomWindow, Window};
use korangar_networking::ShopItem;
use rust_state::{Context, ManuallyAssertExt, Path, VecIndexExt};

use super::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;
use crate::world::ResourceMetadata;

struct ItemList<A> {
    items_path: A,
    elements: Vec<ElementBox<ClientState>>,
}

impl<A> ItemList<A> {
    fn new(items_path: A) -> Self {
        Self {
            items_path,
            elements: Vec::new(),
        }
    }
}

impl<A> Element<ClientState> for ItemList<A>
where
    A: Path<ClientState, Vec<ShopItem<ResourceMetadata>>>,
{
    type LayoutInfo = ();

    fn create_layout_info(
        &mut self,
        state: &Context<ClientState>,
        mut store: ElementStoreMut<'_>,
        resolver: &mut Resolver<'_, ClientState>,
    ) -> Self::LayoutInfo {
        use korangar_interface::prelude::*;

        let items = state.get(&self.items_path);

        match items.len().cmp(&self.elements.len()) {
            Ordering::Less => {
                self.elements.truncate(items.len());
            }
            Ordering::Equal => {}
            Ordering::Greater => {
                for index in self.elements.len()..items.len() {
                    let item_path = self.items_path.index(index).manually_asserted();

                    self.elements.push(ErasedElement::new(button! {
                        text: "Some item",
                        event: move |_: &Context<ClientState>, _: &mut EventQueue<ClientState>| {
                        },
                    }));
                }
            }
        }

        self.elements.iter_mut().enumerate().for_each(|(index, element)| {
            element.create_layout_info(state, store.child_store(index as u64), resolver);
        });
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a Context<ClientState>,
        store: ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut Layout<'a, ClientState>,
    ) {
        let items = state.get(&self.items_path);

        self.elements.iter().enumerate().for_each(|(index, element)| {
            element.lay_out(state, store.child_store(index as u64), &(), layout);
        });
    }
}

pub struct BuyWindow<A, B> {
    items_path: A,
    cart_path: B,
}

impl<A, B> BuyWindow<A, B> {
    pub fn new(items_path: A, cart_path: B) -> Self {
        Self { items_path, cart_path }
    }
}

impl<A, B> CustomWindow<ClientState> for BuyWindow<A, B>
where
    A: Path<ClientState, Vec<ShopItem<ResourceMetadata>>>,
    B: Path<ClientState, Vec<ShopItem<(ResourceMetadata, u32)>>>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Buy)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Buy",
            class: Self::window_class(),
            theme: InterfaceThemeType::Game,
            elements: (
                ItemList::new(self.items_path),
            ),
        }
    }
}
