use std::cmp::Ordering;
use std::fmt::Display;

use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox, ElementSet};
use korangar_interface::event::ClickHandler;
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{Resolver, WindowLayout};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use korangar_interface::window::{CustomWindow, Window};
use korangar_networking::ShopItem;
use rust_state::{Context, ManuallyAssertExt, Path, Selector, VecIndexExt};

use super::WindowClass;
use crate::InputEvent;
use crate::graphics::{Color, CornerDiameter};
use crate::loaders::{FontSize, OverflowBehavior};
use crate::renderer::LayoutExt;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;
use crate::world::ResourceMetadata;

struct PartialEqDisplayStr<T> {
    last_value: Option<T>,
    text: String,
}

impl<T> PartialEqDisplayStr<T> {
    pub fn new() -> Self {
        Self {
            last_value: None,
            text: String::new(),
        }
    }
}

impl<T> PartialEqDisplayStr<T>
where
    T: Clone + PartialEq + Display + 'static,
{
    fn update(&mut self, value: T) {
        if self.last_value.is_none() || self.last_value.as_ref().is_some_and(|last| *last != value) {
            self.text = value.to_string();
            self.last_value = Some(value.clone());
        }
    }

    fn get_str(&self) -> &str {
        &self.text
    }
}

struct ItemLayoutInfo<A> {
    area: Area,
    texture_area: Area,
    text_area: Area,
    children: A,
}

struct ItemElement<A, B> {
    item_path: A,
    children: B,
    amount_string: PartialEqDisplayStr<u32>,
    price_string: PartialEqDisplayStr<u32>,
}

impl<A, B> ItemElement<A, B> {
    fn new(item_path: A, children: B) -> Self {
        Self {
            item_path,
            children,
            amount_string: PartialEqDisplayStr::new(),
            price_string: PartialEqDisplayStr::new(),
        }
    }
}

impl<A, B> Element<ClientState> for ItemElement<A, B>
where
    A: Path<ClientState, ShopItem<(ResourceMetadata, u32)>>,
    B: ElementSet<ClientState>,
{
    type LayoutInfo = ItemLayoutInfo<B::LayoutInfo>;

    fn create_layout_info(
        &mut self,
        state: &Context<ClientState>,
        store: ElementStoreMut<'_>,
        resolver: &mut Resolver<'_, ClientState>,
    ) -> Self::LayoutInfo {
        let (area, (texture_area, text_area, children)) = resolver.with_derived(3.0, 3.0, |resolver| {
            let area = resolver.with_height(34.0);

            let texture_area = Area {
                width: 34.0,
                height: 34.0,
                ..area
            };

            let text_area = Area {
                left: area.left + 43.0,
                width: area.width - 43.0,
                ..area
            };

            let children = self.children.create_layout_info(state, store, resolver);

            (texture_area, text_area, children)
        });

        let item = state.get(&self.item_path);

        self.amount_string.update(item.metadata.1);
        self.price_string.update(item.price.0 * item.metadata.1);

        Self::LayoutInfo {
            area,
            texture_area,
            text_area,
            children,
        }
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a Context<ClientState>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let item = state.get(&self.item_path);

        layout.add_rectangle(layout_info.area, CornerDiameter::uniform(4.0), Color::rgb_u8(80, 80, 80));

        if let Some(texture) = &item.metadata.0.texture {
            layout.add_texture(layout_info.texture_area, texture.clone(), Color::WHITE, false);

            layout.add_text(
                layout_info.texture_area,
                self.amount_string.get_str(),
                FontSize(16.0),
                Color::monochrome_u8(220),
                HorizontalAlignment::Right { offset: 3.0, border: 3.0 },
                VerticalAlignment::Bottom { offset: 0.0 },
                OverflowBehavior::Shrink,
            );
        }

        layout.add_text(
            layout_info.text_area,
            &item.metadata.0.name,
            FontSize(16.0),
            Color::monochrome_u8(220),
            HorizontalAlignment::Left { offset: 3.0, border: 3.0 },
            VerticalAlignment::Center { offset: 0.0 },
            OverflowBehavior::Shrink,
        );

        layout.add_text(
            layout_info.text_area,
            self.price_string.get_str(),
            FontSize(16.0),
            Color::rgb_u8(250, 230, 130),
            HorizontalAlignment::Right { offset: 3.0, border: 3.0 },
            VerticalAlignment::Center { offset: 0.0 },
            OverflowBehavior::Shrink,
        );

        self.children.lay_out(state, store, &layout_info.children, layout);
    }
}

struct ItemList<A> {
    cart_path: A,
    elements: Vec<ElementBox<ClientState>>,
}

impl<A> ItemList<A> {
    fn new(cart_path: A) -> Self {
        Self {
            cart_path,
            elements: Vec::new(),
        }
    }
}

impl<A> Element<ClientState> for ItemList<A>
where
    A: Path<ClientState, Vec<ShopItem<(ResourceMetadata, u32)>>>,
{
    type LayoutInfo = ();

    fn create_layout_info(
        &mut self,
        state: &Context<ClientState>,
        mut store: ElementStoreMut<'_>,
        resolver: &mut Resolver<'_, ClientState>,
    ) -> Self::LayoutInfo {
        use korangar_interface::prelude::*;

        let cart = state.get(&self.cart_path);

        match cart.len().cmp(&self.elements.len()) {
            Ordering::Less => {
                self.elements.truncate(cart.len());
            }
            Ordering::Equal => {}
            Ordering::Greater => {
                for index in self.elements.len()..cart.len() {
                    let item_path = self.cart_path.index(index).manually_asserted();

                    fn disabled_cutoff<A>(item_path: A, amount: u32) -> impl Selector<ClientState, bool>
                    where
                        A: Path<ClientState, ShopItem<(ResourceMetadata, u32)>>,
                    {
                        ComputedSelector::new_default(move |state: &ClientState| {
                            // SAFETY:
                            //
                            // Unwrap is safe here because of the bounds.
                            let item = item_path.follow(state).unwrap();

                            item.metadata.1 < amount
                        })
                    }

                    struct RemoveAction<A, B> {
                        item_path: A,
                        cart_path: B,
                        amount: u32,
                    }

                    impl<A, B> RemoveAction<A, B> {
                        fn new(item_path: A, cart_path: B, amount: u32) -> Self {
                            Self {
                                item_path,
                                cart_path,
                                amount,
                            }
                        }
                    }

                    impl<A, B> ClickHandler<ClientState> for RemoveAction<A, B>
                    where
                        A: Path<ClientState, ShopItem<(ResourceMetadata, u32)>>,
                        B: Path<ClientState, Vec<ShopItem<(ResourceMetadata, u32)>>>,
                    {
                        fn execute(&self, state: &Context<ClientState>, _: &mut EventQueue<ClientState>) {
                            let item_id = state.get(&self.item_path).item_id;
                            let amount = self.amount;

                            state.update_value_with(self.cart_path, move |cart| {
                                if let Some(index) = cart.iter_mut().position(|purchase| purchase.item_id == item_id) {
                                    if cart[index].metadata.1 > amount {
                                        cart[index].metadata.1 -= amount;
                                    } else {
                                        cart.remove(index);
                                    }
                                }
                            });
                        }
                    }

                    let buttons = (split! {
                        gaps: theme().window().gaps(),
                        children: (
                            button! {
                                text: "-1",
                                disabled: disabled_cutoff(item_path, 1),
                                event: RemoveAction::new(item_path, self.cart_path, 1),
                            },
                            button! {
                                text: "-10",
                                disabled: disabled_cutoff(item_path, 10),
                                event: RemoveAction::new(item_path, self.cart_path, 10),
                            },
                            button! {
                                text: "-100",
                                disabled: disabled_cutoff(item_path, 100),
                                event: RemoveAction::new(item_path, self.cart_path, 100),
                            },
                            button! {
                                text: "-All",
                                event: RemoveAction::new(item_path, self.cart_path, u32::MAX),
                            },
                        ),
                    },);

                    self.elements.push(ErasedElement::new(ItemElement::new(item_path, buttons)));
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
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        self.elements.iter().enumerate().for_each(|(index, element)| {
            element.lay_out(state, store.child_store(index as u64), &(), layout);
        });
    }
}

pub struct BuyCartWindow<A> {
    cart_path: A,
}

impl<A> BuyCartWindow<A> {
    pub fn new(cart_path: A) -> Self {
        Self { cart_path }
    }
}

impl<A> CustomWindow<ClientState> for BuyCartWindow<A>
where
    A: Path<ClientState, Vec<ShopItem<(ResourceMetadata, u32)>>>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::BuyCart)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Cart",
            class: Self::window_class(),
            theme: InterfaceThemeType::Game,
            resizable: true,
            elements: (
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Buy",
                            event: move |state: &Context<ClientState>, queue: &mut EventQueue<ClientState>| {
                                let cart = state.get(&self.cart_path);
                                let items = cart
                                    .iter()
                                    .map(|item| ShopItem {
                                        metadata: item.metadata.1,
                                        ..item.clone()
                                    })
                                    .collect();

                                queue.queue(InputEvent::BuyItems { items });
                            }
                        },
                        button! {
                            text: "Cancel",
                            event: InputEvent::CloseShop,
                        },
                    ),
                },
                scroll_view! {
                    children: (
                        ItemList::new(self.cart_path),
                    ),
                },
            ),
        }
    }
}
