use std::fmt::Display;

use korangar_interface::elements::{
    ButtonBuilder, ContainerState, Element, ElementCell, ElementState, ElementWrap, Focus, Headline, WeakElementCell,
};
use korangar_interface::event::HoverInformation;
use korangar_interface::layout::PlacementResolver;
use korangar_interface::state::PlainTrackedState;
use korangar_interface::{dimension_bound, size_bound};
use num::NumCast;
use wgpu::RenderPass;

use super::ItemResourceProvider;
use crate::graphics::{Color, InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::application::InterfaceSettings;
use crate::interface::elements::ItemDisplay;
use crate::interface::layout::{CornerRadius, ScreenClip, ScreenPosition, ScreenSize};
use crate::interface::theme::InterfaceTheme;

#[derive(Clone, Copy)]
pub enum ShopEntryOperation {
    AddToCart,
    RemoveFromCart,
}

impl std::fmt::Display for ShopEntryOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShopEntryOperation::AddToCart => write!(f, "+"),
            ShopEntryOperation::RemoveFromCart => write!(f, "+"),
        }
    }
}

pub struct ShopEntry {
    state: ContainerState<InterfaceSettings>,
    secondary_color: bool,
}

impl ShopEntry {
    fn add_button<SelfItem, CartItem, Amount>(
        item: SelfItem,
        mut cart: PlainTrackedState<Vec<CartItem>>,
        operation: ShopEntryOperation,
        amount: Amount,
        act_button_press: impl Fn(&SelfItem, &mut PlainTrackedState<Vec<CartItem>>, Amount) + 'static,
        disabled_selector: impl Fn(&SelfItem, &PlainTrackedState<Vec<CartItem>>, Amount) -> bool + 'static,
    ) -> ElementCell<InterfaceSettings>
    where
        SelfItem: Clone + 'static,
        CartItem: 'static,
        Amount: Display + Copy + 'static,
    {
        let disabled_item = item.clone();
        let disabled_cart = cart.clone();

        ButtonBuilder::new()
            .with_text(format!("{operation} {amount}"))
            .with_event(move || {
                act_button_press(&item, &mut cart, amount);
                vec![]
            })
            .with_disabled_selector(move || disabled_selector(&disabled_item, &disabled_cart, amount))
            .with_width_bound(dimension_bound!(20%))
            .build()
            .wrap()
    }

    pub fn new<SelfItem, CartItem, Amount>(
        item: SelfItem,
        mut cart: PlainTrackedState<Vec<CartItem>>,
        operation: ShopEntryOperation,
        secondary_color: bool,
        get_item_quantity: impl Fn(&SelfItem) -> Option<usize> + Clone + 'static,
        act_button_press: impl Fn(&SelfItem, &mut PlainTrackedState<Vec<CartItem>>, Amount) + Clone + 'static,
        disabled_selector: impl Fn(&SelfItem, &PlainTrackedState<Vec<CartItem>>, Amount) -> bool + Clone + 'static,
    ) -> Self
    where
        SelfItem: ItemResourceProvider + Clone + 'static,
        CartItem: Clone + 'static,
        Amount: NumCast + Display + Copy + 'static,
    {
        let mut elements = vec![
            ItemDisplay::new(item.clone(), get_item_quantity.clone()).wrap(),
            Headline::new(item.get_resource_metadata().name.clone(), size_bound!(!, 14)).wrap(),
            Self::add_button(
                item.clone(),
                cart.clone(),
                operation,
                Amount::from(1).unwrap(),
                act_button_press.clone(),
                disabled_selector.clone(),
            ),
            Self::add_button(
                item.clone(),
                cart.clone(),
                operation,
                Amount::from(10).unwrap(),
                act_button_press.clone(),
                disabled_selector.clone(),
            ),
            Self::add_button(
                item.clone(),
                cart.clone(),
                operation,
                Amount::from(100).unwrap(),
                act_button_press.clone(),
                disabled_selector.clone(),
            ),
            Self::add_button(
                item.clone(),
                cart.clone(),
                operation,
                Amount::from(1000).unwrap(),
                act_button_press.clone(),
                disabled_selector.clone(),
            ),
        ];

        if let Some(amount) = get_item_quantity(&item) {
            let disabled_item = item.clone();
            let disabled_cart = cart.clone();
            let amount = Amount::from(amount).unwrap();

            let remaining_button = ButtonBuilder::new()
                .with_text(format!("{operation} all"))
                .with_event(move || {
                    act_button_press(&item, &mut cart, amount);
                    Vec::new()
                })
                .with_disabled_selector(move || disabled_selector(&disabled_item, &disabled_cart, amount))
                .with_width_bound(dimension_bound!(20%))
                .build()
                .wrap();

            elements.push(remaining_button);
        }

        let state = ContainerState::new(elements);

        Self { state, secondary_color }
    }
}

impl Element<InterfaceSettings> for ShopEntry {
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
            .resolve(placement_resolver, application, theme, size_bound, ScreenSize::uniform(12.0));
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
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
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
        let mut renderer =
            self.state
                .state
                .element_renderer(render_target, render_pass, renderer, application, parent_position, screen_clip);

        match self.secondary_color {
            true => renderer.render_background(CornerRadius::uniform(5.0), Color::monochrome_u8(60)),
            false => renderer.render_background(CornerRadius::uniform(5.0), Color::monochrome_u8(50)),
        }

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
