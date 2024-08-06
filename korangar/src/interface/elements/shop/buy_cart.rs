use korangar_interface::application::SizeTraitExt;
use korangar_interface::elements::{
    ButtonBuilder, ContainerState, Element, ElementCell, ElementState, ElementWrap, Focus, WeakElementCell,
};
use korangar_interface::event::{ChangeEvent, ClickAction, HoverInformation};
use korangar_interface::layout::PlacementResolver;
use korangar_interface::state::{PlainRemote, PlainTrackedState, Remote, TrackedState, TrackedStateExt};
use korangar_interface::{dimension_bound, size_bound};
use korangar_networking::ShopItem;
use num::Integer;
use rust_state::{Context, View};

use super::CartSum;
use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::{MouseInputMode, UserEvent};
use crate::interface::application::ThemeSelector2;
use crate::interface::elements::{ShopEntry, ShopEntryOperation};
use crate::interface::layout::{ScreenClip, ScreenPosition, ScreenSize};
use crate::loaders::ResourceMetadata;
use crate::GameState;

pub struct BuyCartContainer {
    state: ContainerState<GameState>,
}

impl BuyCartContainer {
    pub fn new() -> Self {
        /* let mut elements = cart
            .get()
            .iter()
            .enumerate()
            .map(|(index, item)| {
                ShopEntry::new(
                    item.clone(),
                    ShopEntryOperation::RemoveFromCart,
                    index.is_odd(),
                    |item| Some(item.metadata.1 as usize),
                    |state, item, amount| {
                        /* cart.mutate(|cart| {
                            let purchase = cart.iter_mut().find(|purchase| purchase.item_id == item.item_id).unwrap();

                            purchase.metadata.1 = purchase.metadata.1.saturating_sub(amount);

                            if purchase.metadata.1 == 0 {
                                cart.retain(|purchase| purchase.item_id != item.item_id);
                            }
                        }); */
                    },
                    |state, item, amount| {
                        state
                            .get_safe(&GameState::buy_cart())
                            .iter()
                            .find(|cart_item| cart_item.item_id == item.item_id)
                            .map(|cart_item| amount.saturating_sub(cart_item.metadata.1) == 0)
                            .unwrap_or(true)
                    },
                )
            })
            .map(ElementWrap::wrap)
            .collect::<Vec<ElementCell<GameState>>>();

        elements.insert(
            0,
            ButtonBuilder::new()
                .with_text("purchase")
                .with_event(move |state: &Context<GameState>| {
                    let items = state
                        .get_safe(&GameState::buy_cart())
                        .iter()
                        .map(|item| ShopItem {
                            metadata: item.metadata.1,
                            ..item.clone()
                        })
                        .collect();

                    vec![ClickAction::Custom(UserEvent::BuyItems { items })]
                })
                .with_width_bound(dimension_bound!(50%))
                .build()
                .wrap(),
        );

        elements.insert(
            1,
            ButtonBuilder::new()
                .with_text("cancel")
                .with_event(move |_: &Context<GameState>| vec![ClickAction::Custom(UserEvent::CloseShop)])
                .with_width_bound(dimension_bound!(!))
                .build()
                .wrap(),
        );

        elements.insert(0, CartSum::new(&cart, |item| item.price.0, |item| item.metadata.1).wrap()); */

        let elements = vec![];
        let state = ContainerState::new(elements);

        Self { state }
    }
}

impl Element<GameState> for BuyCartContainer {
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

    fn resolve(&mut self, state: &View<GameState>, theme_selector: ThemeSelector2, placement_resolver: &mut PlacementResolver<GameState>) {
        let size_bound = &size_bound!(100%, ?);
        self.state
            .resolve(placement_resolver, state, theme_selector, size_bound, ScreenSize::zero());
    }

    /* fn update(&mut self) -> Option<ChangeEvent> {
        if self.cart_remote.consume_changed() {
            let weak_parent = self.state.state.parent_element.take();
            let weak_self = self.state.state.self_element.take().unwrap();

            *self = Self::new(self.cart.clone());
            // important: link back after creating elements, otherwise focus navigation and
            // scrolling would break
            self.link_back(weak_self, weak_parent);

            return Some(ChangeEvent::RESOLVE_WINDOW);
        }

        None
    } */

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
        application: &View<GameState>,
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
