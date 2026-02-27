use std::cmp::Ordering;
use std::fmt::Display;

use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox};
use korangar_interface::event::ClickHandler;
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use korangar_interface::window::{CustomWindow, Window};
use korangar_networking::{ItemQuantity, ShopItem};
use rust_state::{ManuallyAssertExt, Path, PathExt, Selector, State, VecIndexExt};

use super::WindowClass;
use crate::graphics::{Color, CornerDiameter, ShadowPadding};
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
    A: Path<ClientState, ShopItem<ResourceMetadata>>,
    B: Element<ClientState>,
{
    type LayoutInfo = ItemLayoutInfo<B::LayoutInfo>;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
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

            if let ItemQuantity::Fixed(count) = item.quantity {
                self.amount_string.update(count);
            }

            self.price_string.update(item.price.0);

            Self::LayoutInfo {
                area,
                texture_area,
                text_area,
                children,
            }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let item = state.get(&self.item_path);

        layout.add_rectangle(
            layout_info.area,
            CornerDiameter::uniform(4.0),
            Color::rgb_u8(80, 80, 80),
            Color::rgba_u8(0, 0, 0, 100),
            ShadowPadding::diagonal(2.0, 5.0),
        );

        if let Some(texture) = &item.metadata.texture {
            layout.add_texture(layout_info.texture_area, texture.clone(), Color::WHITE, false);

            if matches!(item.quantity, ItemQuantity::Fixed(..)) {
                layout.add_text(
                    layout_info.texture_area,
                    self.amount_string.get_str(),
                    FontSize(16.0),
                    Color::monochrome_u8(220),
                    Color::rgb_u8(255, 160, 60),
                    HorizontalAlignment::Right { offset: 3.0, border: 3.0 },
                    VerticalAlignment::Bottom { offset: 0.0 },
                    OverflowBehavior::Shrink,
                );
            }
        }

        layout.add_text(
            layout_info.text_area,
            &item.metadata.name,
            FontSize(16.0),
            Color::monochrome_u8(220),
            Color::rgb_u8(255, 160, 60),
            HorizontalAlignment::Left { offset: 3.0, border: 3.0 },
            VerticalAlignment::Center { offset: 0.0 },
            OverflowBehavior::Shrink,
        );

        layout.add_text(
            layout_info.text_area,
            self.price_string.get_str(),
            FontSize(16.0),
            Color::rgb_u8(250, 230, 130),
            Color::rgb_u8(255, 160, 60),
            HorizontalAlignment::Right { offset: 3.0, border: 3.0 },
            VerticalAlignment::Center { offset: 0.0 },
            OverflowBehavior::Shrink,
        );

        self.children.lay_out(state, store, &layout_info.children, layout);
    }
}

struct ItemList<A, B> {
    items_path: A,
    cart_path: B,
    elements: Vec<ElementBox<ClientState>>,
}

impl<A, B> ItemList<A, B> {
    fn new(items_path: A, cart_path: B) -> Self {
        Self {
            items_path,
            cart_path,
            elements: Vec::new(),
        }
    }
}

impl<A, B> Element<ClientState> for ItemList<A, B>
where
    A: Path<ClientState, Vec<ShopItem<ResourceMetadata>>>,
    B: Path<ClientState, Vec<ShopItem<(ResourceMetadata, u32)>>>,
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

            let items = state.get(&self.items_path);

            match items.len().cmp(&self.elements.len()) {
                Ordering::Less => {
                    self.elements.truncate(items.len());
                }
                Ordering::Equal => {}
                Ordering::Greater => {
                    for index in self.elements.len()..items.len() {
                        let item_path = self.items_path.index(index).manually_asserted();
                        let cart_path = self.cart_path;

                        fn disabled_cutoff<A, B>(item_path: A, cart_path: B, amount: u32) -> impl Selector<ClientState, bool>
                        where
                            A: Path<ClientState, ShopItem<ResourceMetadata>>,
                            B: Path<ClientState, Vec<ShopItem<(ResourceMetadata, u32)>>>,
                        {
                            ComputedSelector::new_default(move |state: &ClientState| {
                                let item = item_path.follow_safe(state);
                                let cart = cart_path.follow_safe(state);

                                cart.iter()
                                    .find(|purchase| purchase.item_id == item.item_id)
                                    .map(|purchase| matches!(item.quantity, ItemQuantity::Fixed(quantity) if quantity - purchase.metadata.1 < amount))
                                    .unwrap_or_else(|| matches!(item.quantity, ItemQuantity::Fixed(quantity) if quantity < amount))
                            })
                        }

                        fn resolve_amount(
                            amount: ItemQuantity,
                            _item: &ShopItem<ResourceMetadata>,
                            _cart: &[ShopItem<(ResourceMetadata, u32)>],
                        ) -> u32 {
                            match amount {
                                ItemQuantity::Fixed(count) => count,
                                ItemQuantity::Infinite => todo!(),
                            }
                        }

                        struct AddAction<A, B> {
                            item_path: A,
                            cart_path: B,
                            amount: ItemQuantity,
                        }

                        impl<A, B> AddAction<A, B> {
                            fn new(item_path: A, cart_path: B, amount: ItemQuantity) -> Self {
                                Self {
                                    item_path,
                                    cart_path,
                                    amount,
                                }
                            }
                        }

                        impl<A, B> ClickHandler<ClientState> for AddAction<A, B>
                        where
                            A: Path<ClientState, ShopItem<ResourceMetadata>>,
                            B: Path<ClientState, Vec<ShopItem<(ResourceMetadata, u32)>>>,
                        {
                            fn handle_click(&self, state: &State<ClientState>, _: &mut EventQueue<ClientState>) {
                                let item = state.get(&self.item_path).clone();
                                let amount = self.amount;

                                state.update_value_with(self.cart_path, move |cart| {
                                    let amount = resolve_amount(amount, &item, cart);

                                    if let Some(purchase) = cart.iter_mut().find(|purchase| purchase.item_id == item.item_id) {
                                        purchase.metadata.1 += amount;
                                    } else {
                                        cart.push(ShopItem {
                                            metadata: (item.metadata.clone(), amount),
                                            item_id: item.item_id,
                                            item_type: item.item_type,
                                            price: item.price,
                                            quantity: item.quantity,
                                            weight: item.weight,
                                            location: item.location,
                                        });
                                    }
                                });
                            }
                        }

                        let buttons = (split! {
                            gaps: theme().window().gaps(),
                            children: (
                                button! {
                                    text: "+1",
                                    disabled: disabled_cutoff(item_path, cart_path, 1),
                                    event: AddAction::new(item_path, cart_path, ItemQuantity::Fixed(1)),
                                },
                                button! {
                                    text: "+10",
                                    disabled: disabled_cutoff(item_path, cart_path, 10),
                                    event: AddAction::new(item_path, cart_path, ItemQuantity::Fixed(10)),
                                },
                                button! {
                                    text: "+100",
                                    disabled: disabled_cutoff(item_path, cart_path, 100),
                                    event: AddAction::new(item_path, cart_path, ItemQuantity::Fixed(100)),
                                },
                                // TODO: Needs special treatment. Should be +All or +1000 depending on
                                // the available quantity.
                                // button! {
                                //     text: "+All",
                                //     disabled: DisabledCutoff::new(item_path, cart_path, 1),
                                //     event: AddAction::new(item_path, cart_path, ItemQuantity::Infinite),
                                // },
                            ),
                        },);

                        self.elements.push(ErasedElement::new(ItemElement::new(item_path, buttons)));
                    }
                }
            }

            self.elements.iter_mut().enumerate().for_each(|(index, element)| {
                element.create_layout_info(state, store.child_store(index as u64), resolver);
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
            theme: InterfaceThemeType::InGame,
            resizable: true,
            elements: (
                scroll_view! {
                    children: ItemList::new(self.items_path, self.cart_path),
                },
            ),
        }
    }
}
