use korangar_interface::application::{FontSizeTrait, PositionTraitExt};
use korangar_interface::elements::{Element, ElementState};
use korangar_interface::layout::PlacementResolver;
use korangar_interface::size_bound;
use korangar_interface::state::{PlainTrackedState, TrackedState};
use rust_state::View;

use crate::graphics::{Color, InterfaceRenderer, Renderer};
use crate::interface::application::ThemeSelector2;
use crate::interface::layout::{ScreenClip, ScreenPosition};
use crate::loaders::FontSize;
use crate::GameState;

pub struct CartSum {
    total_price: u32,
    state: ElementState<GameState>,
}

impl CartSum {
    pub fn new<Item>(cart: &PlainTrackedState<Vec<Item>>, get_price: impl Fn(&Item) -> u32, get_quantity: impl Fn(&Item) -> u32) -> Self
    where
        Item: 'static,
    {
        let total_price = cart.get().iter().map(|item| get_price(item) * get_quantity(item)).sum::<u32>();
        let state = ElementState::default();

        Self { total_price, state }
    }
}

impl Element<GameState> for CartSum {
    fn get_state(&self) -> &ElementState<GameState> {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<GameState> {
        &mut self.state
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn resolve(
        &mut self,
        _state: &View<GameState>,
        _theme_selector: ThemeSelector2,
        placement_resolver: &mut PlacementResolver<GameState>,
    ) {
        self.state.resolve(placement_resolver, &size_bound!(100%, 20));
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        state: &View<GameState>,
        _theme_selector: ThemeSelector2,
        parent_position: ScreenPosition,
        screen_clip: ScreenClip,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, state, parent_position, screen_clip);

        renderer.render_text(
            &format!("Total price: {}", self.total_price),
            ScreenPosition::zero(),
            Color::rgb_u8(255, 200, 200),
            FontSize::new(16.0),
        );
    }
}
