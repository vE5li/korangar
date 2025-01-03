use korangar_interface::application::SizeTraitExt;
use korangar_interface::elements::{ContainerState, Element, ElementCell, ElementState, ElementWrap, Focus, WeakElementCell};
use korangar_interface::event::{ChangeEvent, HoverInformation};
use korangar_interface::layout::PlacementResolver;
use korangar_interface::size_bound;
use korangar_interface::state::{PlainRemote, PlainTrackedState, Remote, TrackedState, TrackedStateExt};
use korangar_networking::SellItem;
use num::Integer;

use crate::input::MouseInputMode;
use crate::interface::application::InterfaceSettings;
use crate::interface::elements::{ShopEntry, ShopEntryOperation};
use crate::interface::layout::{ScreenClip, ScreenPosition, ScreenSize};
use crate::interface::theme::InterfaceTheme;
use crate::renderer::InterfaceRenderer;
use crate::world::ResourceMetadata;

pub struct SellContainer {
    items: PlainRemote<Vec<SellItem<(ResourceMetadata, u16)>>>,
    cart: PlainTrackedState<Vec<SellItem<(ResourceMetadata, u16)>>>,
    state: ContainerState<InterfaceSettings>,
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
            .collect::<Vec<ElementCell<InterfaceSettings>>>();

        let state = ContainerState::new(elements);

        Self { items, cart, state }
    }
}

impl Element<InterfaceSettings> for SellContainer {
    fn get_state(&self) -> &ElementState<InterfaceSettings> {
        &self.state.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<InterfaceSettings> {
        &mut self.state.state
    }

    fn link_back(&mut self, weak_self: WeakElementCell<InterfaceSettings>, weak_parent: Option<WeakElementCell<InterfaceSettings>>) {
        self.state.link_back(weak_self, weak_parent);
    }

    fn is_focusable(&self) -> bool {
        self.state.is_focusable::<false>()
    }

    fn focus_next(
        &self,
        self_cell: ElementCell<InterfaceSettings>,
        caller_cell: Option<ElementCell<InterfaceSettings>>,
        focus: Focus,
    ) -> Option<ElementCell<InterfaceSettings>> {
        self.state.focus_next::<false>(self_cell, caller_cell, focus)
    }

    fn restore_focus(&self, self_cell: ElementCell<InterfaceSettings>) -> Option<ElementCell<InterfaceSettings>> {
        self.state.restore_focus(self_cell)
    }

    fn resolve(
        &mut self,
        placement_resolver: &mut PlacementResolver<InterfaceSettings>,
        application: &InterfaceSettings,
        theme: &InterfaceTheme,
    ) {
        let size_bound = &size_bound!(100%, ?);
        self.state
            .resolve(placement_resolver, application, theme, size_bound, ScreenSize::zero());
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

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation<InterfaceSettings> {
        match mouse_mode {
            MouseInputMode::MoveItem(..) => self.state.state.hovered_element(mouse_position),
            MouseInputMode::None => self.state.hovered_element(mouse_position, mouse_mode, false),
            _ => HoverInformation::Missed,
        }
    }

    fn render(
        &self,
        renderer: &InterfaceRenderer,
        application: &InterfaceSettings,
        theme: &InterfaceTheme,
        parent_position: ScreenPosition,
        screen_clip: ScreenClip,
        hovered_element: Option<&dyn Element<InterfaceSettings>>,
        focused_element: Option<&dyn Element<InterfaceSettings>>,
        mouse_mode: &MouseInputMode,
        second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .state
            .element_renderer(renderer, application, parent_position, screen_clip);

        self.state.render(
            &mut renderer,
            application,
            theme,
            hovered_element,
            focused_element,
            mouse_mode,
            second_theme,
        );
    }
}
