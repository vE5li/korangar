use korangar_interface::application::{FontSizeTrait, SizeTraitExt};
use korangar_interface::elements::{Element, ElementState};
use korangar_interface::layout::PlacementResolver;
use korangar_interface::size_bound;
use korangar_interface::theme::ButtonTheme;
use korangar_networking::{SellItem, ShopItem};
use rust_state::Tracker;

use crate::graphics::{Color, InterfaceRenderer, Renderer, SpriteRenderer};
use crate::input::MouseInputMode;
use crate::interface::application::ThemeSelector2;
use crate::interface::layout::{CornerRadius, ScreenClip, ScreenPosition, ScreenSize};
use crate::interface::theme::InterfaceTheme;
use crate::loaders::{FontSize, ResourceMetadata, Scaling};
use crate::{GameState, GameStateFocusedElementPath, GameStateHoveredElementPath, GameStateMouseModePath};

pub trait ItemResourceProvider {
    fn get_resource_metadata(&self) -> &ResourceMetadata;
}

impl ItemResourceProvider for ShopItem<ResourceMetadata> {
    fn get_resource_metadata(&self) -> &ResourceMetadata {
        &self.metadata
    }
}

impl ItemResourceProvider for ShopItem<(ResourceMetadata, u32)> {
    fn get_resource_metadata(&self) -> &ResourceMetadata {
        &self.metadata.0
    }
}

impl ItemResourceProvider for SellItem<(ResourceMetadata, u16)> {
    fn get_resource_metadata(&self) -> &ResourceMetadata {
        &self.metadata.0
    }
}

pub struct ItemDisplay<Item, Quantity> {
    item: Item,
    get_quantity: Quantity,
    state: ElementState<GameState>,
}

impl<Item, Quantity> ItemDisplay<Item, Quantity> {
    pub fn new(item: Item, get_quantity: Quantity) -> Self {
        Self {
            item,
            get_quantity,
            state: ElementState::default(),
        }
    }
}

impl<Item, Quantity> Element<GameState> for ItemDisplay<Item, Quantity>
where
    Item: ItemResourceProvider,
    Quantity: Fn(&Item) -> Option<usize>,
{
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
        state: &Tracker<GameState>,
        theme_selector: ThemeSelector2,
        placement_resolver: &mut PlacementResolver<GameState>,
    ) {
        self.state.resolve(placement_resolver, &size_bound!(30, 30));
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        application: &Tracker<GameState>,
        theme_selector: ThemeSelector2,
        parent_position: ScreenPosition,
        screen_clip: ScreenClip,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, application, parent_position, screen_clip);

        let mouse_mode = application.get_safe(&GameStateMouseModePath::default());
        let hovered_element = application.get_safe(&GameStateHoveredElementPath::default());
        let focused_element = application.get_safe(&GameStateFocusedElementPath::default());
        let highlighted = self.is_element_self(hovered_element) || self.is_element_self(focused_element);

        let background_color = match highlighted {
            true if matches!(mouse_mode, MouseInputMode::None) => {
                application.get_safe(&ButtonTheme::hovered_background_color(theme_selector))
            }
            _ => application.get_safe(&ButtonTheme::background_color(theme_selector)),
        };

        renderer.render_background(CornerRadius::uniform(5.0), *background_color);

        renderer.renderer.render_sprite(
            renderer.render_target,
            self.item.get_resource_metadata().texture.clone(),
            renderer.position,
            ScreenSize::uniform(30.0).scaled(Scaling::new(application.get_scaling_factor())),
            renderer.clip,
            Color::monochrome_u8(255),
            false,
        );

        if let Some(quantity) = (self.get_quantity)(&self.item) {
            renderer.render_text(
                &format!("{}", quantity),
                ScreenPosition::default(),
                *application.get_safe(&ButtonTheme::foreground_color(theme_selector)),
                FontSize::new(12.0),
            );
        }
    }
}
