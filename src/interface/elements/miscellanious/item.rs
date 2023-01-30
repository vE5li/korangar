use cgmath::{Array, Vector4, Zero};
use derive_new::new;
use procedural::*;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::*;
use crate::inventory::Item;

#[derive(new)]
pub struct ItemBox {
    item: Option<Item>,
    source: ItemSource,
    highlight: Box<dyn Fn(&MouseInputMode) -> bool>,
    #[new(default)]
    state: ElementState,
}

impl Element for ItemBox {
    fn get_state(&self) -> &ElementState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state
    }

    fn is_focusable(&self) -> bool {
        self.item.is_some()
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, _theme: &Theme) {
        self.state.resolve(placement_resolver, &constraint!(30, 30));
    }

    fn hovered_element(&self, mouse_position: Position, mouse_mode: &MouseInputMode) -> HoverInformation {
        match self.item.is_some() || matches!(mouse_mode, MouseInputMode::MoveItem(..)) {
            true => self.state.hovered_element(mouse_position),
            false => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, _force_update: &mut bool) -> Option<ClickAction> {
        if let Some(item) = &self.item {
            return Some(ClickAction::MoveItem(self.source, item.clone()));
        }

        None
    }

    fn drop_item(&mut self, item_source: ItemSource, item: Item) -> Option<ItemMove> {
        Some(ItemMove {
            source: item_source,
            destination: self.source,
            item,
        })
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        _state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        theme: &Theme,
        parent_position: Position,
        clip_size: ClipSize,
        hovered_element: Option<&dyn Element>,
        focused_element: Option<&dyn Element>,
        mouse_mode: &MouseInputMode,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, interface_settings, parent_position, clip_size);

        let highlight = (self.highlight)(mouse_mode);
        let background_color = match self.is_element_self(hovered_element) || self.is_element_self(focused_element) {
            true if highlight => Color::rgba(60, 160, 160, 255),
            true if matches!(mouse_mode, MouseInputMode::None) => *theme.button.hovered_background_color,
            false if highlight => Color::rgba(160, 160, 60, 255),
            _ => *theme.button.background_color,
        };

        renderer.render_background(Vector4::from_value(5.0), background_color);

        if let Some(item) = &self.item {
            renderer.render_sprite(
                item.texture.clone(),
                Vector2::zero(),
                Vector2::from_value(30.0),
                Color::monochrome(255),
            );

            renderer.render_text(
                //&format!("{}", self.item.amount),
                "1",
                Vector2::zero(),
                *theme.button.foreground_color,
                8.0,
            );
        }
    }
}
