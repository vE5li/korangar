use korangar_interface::application::SizeTraitExt;
use korangar_interface::elements::{ContainerState, Element, ElementCell, ElementState, ElementWrap, Focus, WeakElementCell};
use korangar_interface::event::{ChangeEvent, HoverInformation};
use korangar_interface::layout::PlacementResolver;
use korangar_interface::size_bound;
use korangar_interface::state::{PlainRemote, PlainTrackedState, Remote, TrackedState, TrackedStateExt};
use korangar_networking::SellItem;
use num::Integer;
use rust_state::Tracker;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::application::ThemeSelector2;
use crate::interface::elements::{ShopEntry, ShopEntryOperation};
use crate::interface::layout::{ScreenClip, ScreenPosition, ScreenSize};
use crate::interface::theme::InterfaceTheme;
use crate::loaders::ResourceMetadata;
use crate::GameState;

pub struct SellContainer {
    items: PlainRemote<Vec<SellItem<(ResourceMetadata, u16)>>>,
    cart: PlainTrackedState<Vec<SellItem<(ResourceMetadata, u16)>>>,
    state: ContainerState<GameState>,
}

impl SellContainer {
    pub fn new(
        items: PlainRemote<Vec<SellItem<(ResourceMetadata, u16)>>>,
        cart: PlainTrackedState<Vec<SellItem<(ResourceMetadata, u16)>>>,
    ) -> Self {
        let elements = items
            .get()
            .iter()
            .enumerate()
            .map(|(index, item)| {
                ShopEntry::new(
                    item.clone(),
                    cart.clone(),
                    ShopEntryOperation::AddToCart,
                    index.is_odd(),
                    |item| Some(item.metadata.1 as usize),
                    |item, cart, amount| {
                        cart.mutate(|cart| {
                            if let Some(purchase) = cart.iter_mut().find(|purchase| purchase.inventory_index == item.inventory_index) {
                                purchase.metadata.1 += amount;
                            } else {
                                cart.push(SellItem {
                                    metadata: (item.metadata.0.clone(), amount),
                                    inventory_index: item.inventory_index,
                                    price: item.price,
                                    overcharge_price: item.overcharge_price,
                                });
                            }
                        });
                    },
                    |item, cart, amount| {
                        let cart_quantity = cart
                            .get()
                            .iter()
                            .find(|cart_item| cart_item.inventory_index == item.inventory_index)
                            .map(|cart_item| cart_item.metadata.1)
                            .unwrap_or(0);

                        item.metadata.1.saturating_sub(cart_quantity) >= amount
                    },
                )
            })
            .map(ElementWrap::wrap)
            .collect::<Vec<ElementCell<GameState>>>();

        let state = ContainerState::new(elements);

        Self { items, cart, state }
    }
}

impl Element<GameState> for SellContainer {
    fn get_state(&self) -> &ElementState<GameState> {
        &self.state.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<GameState> {
        &mut self.state.state
    }

    fn link_back(&mut self, weak_self: WeakElementCell<GameState>, weak_parent: Option<WeakElementCell<GameState>>) {
        self.state.link_back(weak_self, weak_parent);
    }

    fn is_focusable(&self) -> bool {
        self.state.is_focusable::<false>()
    }

    fn focus_next(
        &self,
        self_cell: ElementCell<GameState>,
        caller_cell: Option<ElementCell<GameState>>,
        focus: Focus,
    ) -> Option<ElementCell<GameState>> {
        self.state.focus_next::<false>(self_cell, caller_cell, focus)
    }

    fn restore_focus(&self, self_cell: ElementCell<GameState>) -> Option<ElementCell<GameState>> {
        self.state.restore_focus(self_cell)
    }

    fn resolve(
        &mut self,
        state: &Tracker<GameState>,
        theme_selector: ThemeSelector2,
        placement_resolver: &mut PlacementResolver<GameState>,
    ) {
        let size_bound = &size_bound!(100%, ?);
        self.state
            .resolve(placement_resolver, state, theme_selector, size_bound, ScreenSize::zero());
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        if self.items.consume_changed() {
            let weak_parent = self.state.state.parent_element.take();
            let weak_self = self.state.state.self_element.take().unwrap();

            *self = Self::new(self.items.clone(), self.cart.clone());
            // important: link back after creating elements, otherwise focus navigation and
            // scrolling would break
            self.link_back(weak_self, weak_parent);

            return Some(ChangeEvent::RESOLVE_WINDOW);
        }

        None
    }

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation<GameState> {
        match mouse_mode {
            MouseInputMode::MoveItem(..) => self.state.state.hovered_element(mouse_position),
            MouseInputMode::None => self.state.hovered_element(mouse_position, mouse_mode, false),
            _ => HoverInformation::Missed,
        }
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        application: &Tracker<GameState>,
        theme_selector: ThemeSelector2,
        parent_position: ScreenPosition,
        screen_clip: ScreenClip,
        second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .state
            .element_renderer(render_target, renderer, application, parent_position, screen_clip);

        self.state.render(&mut renderer, application, theme_selector, second_theme);
    }
}
