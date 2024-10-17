use korangar_interface::application::{FontSizeTrait, SizeTraitExt};
use korangar_interface::elements::{Element, ElementState};
use korangar_interface::layout::PlacementResolver;
use korangar_interface::size_bound;
use korangar_networking::{SellItem, ShopItem};

use crate::graphics::Color;
use crate::input::MouseInputMode;
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{CornerRadius, ScreenClip, ScreenPosition, ScreenSize};
use crate::interface::theme::InterfaceTheme;
use crate::loaders::{FontSize, ResourceMetadata, Scaling};
use crate::renderer::{InterfaceRenderer, SpriteRenderer};

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
    state: ElementState<InterfaceSettings>,
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

impl<Item, Quantity> Element<InterfaceSettings> for ItemDisplay<Item, Quantity>
where
    Item: ItemResourceProvider,
    Quantity: Fn(&Item) -> Option<usize>,
{
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
        self.state.resolve(placement_resolver, &size_bound!(30, 30));
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
        _second_theme: bool,
    ) {
        let mut renderer = self.state.element_renderer(renderer, application, parent_position, screen_clip);

        let background_color = match self.is_element_self(hovered_element) || self.is_element_self(focused_element) {
            true if matches!(mouse_mode, MouseInputMode::None) => theme.button.hovered_background_color.get(),
            _ => theme.button.background_color.get(),
        };

        renderer.render_background(CornerRadius::uniform(5.0), background_color);

        renderer.renderer.render_sprite(
            self.item.get_resource_metadata().texture.clone(),
            renderer.position,
            ScreenSize::uniform(30.0).scaled(Scaling::new(application.get_scaling_factor())),
            renderer.clip,
            Color::WHITE,
            false,
        );

        if let Some(quantity) = (self.get_quantity)(&self.item) {
            renderer.render_text(
                &format!("{}", quantity),
                ScreenPosition::default(),
                theme.button.foreground_color.get(),
                FontSize::new(12.0),
            );
        }
    }
}
