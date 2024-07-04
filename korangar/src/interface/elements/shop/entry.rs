use std::fmt::Display;

use korangar_interface::elements::{
    ButtonBuilder, ContainerState, Element, ElementCell, ElementState, ElementWrap, Focus, Headline, WeakElementCell,
};
use korangar_interface::event::HoverInformation;
use korangar_interface::layout::PlacementResolver;
use korangar_interface::state::PlainTrackedState;
use korangar_interface::{dimension_bound, size_bound};
use num::NumCast;
use rust_state::{Context, Tracker};

use super::ItemResourceProvider;
use crate::graphics::{Color, InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::application::ThemeSelector2;
use crate::interface::elements::ItemDisplay;
use crate::interface::layout::{CornerRadius, ScreenClip, ScreenPosition, ScreenSize};
use crate::interface::theme::InterfaceTheme;
use crate::GameState;

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
    state: ContainerState<GameState>,
    secondary_color: bool,
}

impl ShopEntry {
    fn add_button<SelfItem, CartItem, Amount>(
        item: SelfItem,
        operation: ShopEntryOperation,
        amount: Amount,
        act_button_press: impl Fn(&Context<GameState>, &SelfItem, Amount) + 'static,
        disabled_selector: impl Fn(&Tracker<GameState>, &SelfItem, Amount) -> bool + 'static,
    ) -> ElementCell<GameState>
    where
        SelfItem: Clone + 'static,
        CartItem: 'static,
        Amount: Display + Copy + 'static,
    {
        let disabled_item = item.clone();

        ButtonBuilder::new()
            .with_text(format!("{operation} {amount}"))
            .with_event(move |state: &Context<GameState>| {
                act_button_press(state, &item, amount);
                vec![]
            })
            .with_disabled_selector(move |state: &Tracker<GameState>| disabled_selector(state, &disabled_item, amount))
            .with_width_bound(dimension_bound!(20%))
            .build()
            .wrap()
    }

    pub fn new<SelfItem, CartItem, Amount>(
        item: SelfItem,
        operation: ShopEntryOperation,
        secondary_color: bool,
        get_item_quantity: impl Fn(&SelfItem) -> Option<usize> + Clone + 'static,
        act_button_press: impl Fn(&Context<GameState>, &SelfItem, Amount) + Clone + 'static,
        disabled_selector: impl Fn(&Tracker<GameState>, &SelfItem, Amount) -> bool + Clone + 'static,
    ) -> Self
    where
        SelfItem: ItemResourceProvider + Clone + 'static,
        CartItem: Clone + 'static,
        Amount: NumCast + Display + Copy + 'static,
    {
        let mut elements = vec![
            /* ItemDisplay::new(item.clone(), get_item_quantity.clone()).wrap(),
            Headline::new(item.get_resource_metadata().name.clone(), size_bound!(!, 14)).wrap(),
            Self::add_button(
                item.clone(),
                operation,
                Amount::from(1).unwrap(),
                act_button_press.clone(),
                disabled_selector.clone(),
            ),
            Self::add_button(
                item.clone(),
                operation,
                Amount::from(10).unwrap(),
                act_button_press.clone(),
                disabled_selector.clone(),
            ),
            Self::add_button(
                item.clone(),
                operation,
                Amount::from(100).unwrap(),
                act_button_press.clone(),
                disabled_selector.clone(),
            ),
            Self::add_button(
                item.clone(),
                operation,
                Amount::from(1000).unwrap(),
                act_button_press.clone(),
                disabled_selector.clone(),
            ), */
        ];

        if let Some(amount) = get_item_quantity(&item) {
            let disabled_item = item.clone();
            let amount = Amount::from(amount).unwrap();

            let remaining_button = ButtonBuilder::new()
                .with_text(format!("{operation} all"))
                .with_event(move |state: &Context<GameState>| {
                    act_button_press(state, &item, amount);
                    Vec::new()
                })
                .with_disabled_selector(move |state: &Tracker<GameState>| disabled_selector(state, &disabled_item, amount))
                .with_width_bound(dimension_bound!(20%))
                .build()
                .wrap();

            elements.push(remaining_button);
        }

        let state = ContainerState::new(elements);

        Self { state, secondary_color }
    }
}

impl Element<GameState> for ShopEntry {
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
            .resolve(placement_resolver, state, theme_selector, size_bound, ScreenSize::uniform(12.0));
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

        match self.secondary_color {
            true => renderer.render_background(CornerRadius::uniform(5.0), Color::monochrome_u8(60)),
            false => renderer.render_background(CornerRadius::uniform(5.0), Color::monochrome_u8(50)),
        }

        self.state.render(&mut renderer, application, theme_selector, second_theme);
    }
}
