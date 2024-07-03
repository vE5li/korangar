use derive_new::new;
use korangar_interface::application::{FontSizeTrait, SizeTraitExt};
use korangar_interface::elements::{Element, ElementState};
use korangar_interface::event::{ClickAction, HoverInformation};
use korangar_interface::layout::PlacementResolver;
use korangar_interface::size_bound;
use korangar_interface::theme::ButtonTheme;
use korangar_networking::{InventoryItem, InventoryItemDetails};
use rust_state::{Context, Tracker};

use crate::graphics::{Color, InterfaceRenderer, Renderer, SpriteRenderer};
use crate::input::MouseInputMode;
use crate::interface::application::ThemeSelector2;
use crate::interface::layout::{CornerRadius, ScreenClip, ScreenPosition, ScreenSize};
use crate::interface::resource::{ItemSource, Move, PartialMove};
use crate::interface::theme::InterfaceTheme;
use crate::loaders::{FontSize, ResourceMetadata, Scaling};
use crate::{GameState, GameStateFocusedElementPath, GameStateHoveredElementPath, GameStateMouseModePath, GameStateScalePath};

#[derive(new)]
pub struct ItemBox {
    item: Option<InventoryItem<ResourceMetadata>>,
    source: ItemSource,
    highlight: Box<dyn Fn(&MouseInputMode) -> bool>,
    #[new(default)]
    state: ElementState<GameState>,
}

impl Element<GameState> for ItemBox {
    fn get_state(&self) -> &ElementState<GameState> {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<GameState> {
        &mut self.state
    }

    fn is_focusable(&self) -> bool {
        self.item.is_some()
    }

    fn resolve(
        &mut self,
        _state: &Tracker<GameState>,
        _theme_selector: ThemeSelector2,
        placement_resolver: &mut PlacementResolver<GameState>,
    ) {
        self.state.resolve(placement_resolver, &size_bound!(30, 30));
    }

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation<GameState> {
        match self.item.is_some() || matches!(mouse_mode, MouseInputMode::MoveItem(..)) {
            true => self.state.hovered_element(mouse_position),
            false => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, _state: &Context<GameState>, _force_update: &mut bool) -> Vec<ClickAction<GameState>> {
        if let Some(item) = &self.item {
            return vec![ClickAction::Move(PartialMove::Item {
                source: self.source,
                item: item.clone(),
            })];
        }

        Vec::new()
    }

    fn drop_resource(&mut self, drop_resource: PartialMove) -> Option<Move> {
        let PartialMove::Item { source, item } = drop_resource else {
            return None;
        };

        (source != self.source).then_some(Move::Item {
            source,
            destination: self.source,
            item,
        })
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        state: &Tracker<GameState>,
        theme_selector: ThemeSelector2,
        parent_position: ScreenPosition,
        screen_clip: ScreenClip,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, state, parent_position, screen_clip);

        let mouse_mode = state.get_safe(&GameStateMouseModePath::default());
        let hovered_element = state.get_safe(&GameStateHoveredElementPath::default());
        let focused_element = state.get_safe(&GameStateFocusedElementPath::default());
        let highlighted = self.is_element_self(hovered_element) || self.is_element_self(focused_element);

        let highlight = (self.highlight)(mouse_mode);
        let background_color = match highlighted {
            true if highlight => Color::rgba_u8(60, 160, 160, 255),
            true if matches!(mouse_mode, MouseInputMode::None) => *state.get_safe(&ButtonTheme::hovered_background_color(theme_selector)),
            false if highlight => Color::rgba_u8(160, 160, 60, 255),
            _ => *state.get_safe(&ButtonTheme::background_color(theme_selector)),
        };

        renderer.render_background(CornerRadius::uniform(5.0), background_color);

        if let Some(item) = &self.item {
            renderer.renderer.render_sprite(
                renderer.render_target,
                item.metadata.texture.clone(),
                renderer.position,
                ScreenSize::uniform(30.0).scaled(*state.get_safe(&GameStateScalePath::default())),
                renderer.clip,
                Color::monochrome_u8(255),
                false,
            );

            if let InventoryItemDetails::Regular { amount, .. } = &item.details {
                renderer.render_text(
                    &format!("{}", amount),
                    ScreenPosition::default(),
                    *state.get_safe(&ButtonTheme::foreground_color(theme_selector)),
                    FontSize::new(12.0),
                );
            }
        }
    }
}
