use korangar_interface::application::{FontSizeTrait, PositionTraitExt};
use korangar_interface::elements::{Element, ElementState};
use korangar_interface::layout::PlacementResolver;
use korangar_interface::size_bound;
use korangar_interface::state::{PlainTrackedState, TrackedState};

use crate::graphics::{Color, InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{ScreenClip, ScreenPosition};
use crate::interface::theme::InterfaceTheme;
use crate::loaders::FontSize;

pub struct CartSum {
    total_price: u32,
    state: ElementState<InterfaceSettings>,
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

impl Element<InterfaceSettings> for CartSum {
    fn get_state(&self) -> &ElementState<InterfaceSettings> {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<InterfaceSettings> {
        &mut self.state
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn resolve(
        &mut self,
        placement_resolver: &mut PlacementResolver<InterfaceSettings>,
        _application: &InterfaceSettings,
        _theme: &InterfaceTheme,
    ) {
        self.state.resolve(placement_resolver, &size_bound!(100%, 20));
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        application: &InterfaceSettings,
        _theme: &InterfaceTheme,
        parent_position: ScreenPosition,
        screen_clip: ScreenClip,
        _hovered_element: Option<&dyn Element<InterfaceSettings>>,
        _focused_element: Option<&dyn Element<InterfaceSettings>>,
        _mouse_mode: &MouseInputMode,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, application, parent_position, screen_clip);

        renderer.render_text(
            &format!("Total price: {}", self.total_price),
            ScreenPosition::zero(),
            Color::rgb_u8(255, 200, 200),
            FontSize::new(16.0),
        );
    }
}
