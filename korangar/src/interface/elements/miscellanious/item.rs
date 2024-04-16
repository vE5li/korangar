use derive_new::new;
use korangar_interface::application::{FontSizeTrait, SizeTraitExt};
use korangar_interface::elements::{Element, ElementState};
use korangar_interface::event::{ClickAction, HoverInformation};
use korangar_interface::layout::PlacementResolver;
use korangar_interface::size_bound;

use crate::graphics::{Color, InterfaceRenderer, Renderer, SpriteRenderer};
use crate::input::MouseInputMode;
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{CornerRadius, ScreenClip, ScreenPosition, ScreenSize};
use crate::interface::resource::{ItemSource, Move, PartialMove};
use crate::interface::theme::InterfaceTheme;
use crate::inventory::Item;
use crate::loaders::{FontSize, Scaling};

#[derive(new)]
pub struct ItemBox {
    item: Option<Item>,
    source: ItemSource,
    highlight: Box<dyn Fn(&MouseInputMode) -> bool>,
    #[new(default)]
    state: ElementState<InterfaceSettings>,
}

impl Element<InterfaceSettings> for ItemBox {
    fn get_state(&self) -> &ElementState<InterfaceSettings> {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<InterfaceSettings> {
        &mut self.state
    }

    fn is_focusable(&self) -> bool {
        self.item.is_some()
    }

    fn resolve(
        &mut self,
        placement_resolver: &mut PlacementResolver<InterfaceSettings>,
        _application: &InterfaceSettings,
        _theme: &InterfaceTheme,
    ) {
        self.state.resolve(placement_resolver, &size_bound!(30, 30));
    }

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation<InterfaceSettings> {
        match self.item.is_some() || matches!(mouse_mode, MouseInputMode::MoveItem(..)) {
            true => self.state.hovered_element(mouse_position),
            false => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, _force_update: &mut bool) -> Vec<ClickAction<InterfaceSettings>> {
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
        application: &InterfaceSettings,
        theme: &InterfaceTheme,
        parent_position: ScreenPosition,
        screen_clip: ScreenClip,
        hovered_element: Option<&dyn Element<InterfaceSettings>>,
        focused_element: Option<&dyn Element<InterfaceSettings>>,
        mouse_mode: &MouseInputMode,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, application, parent_position, screen_clip);

        let highlight = (self.highlight)(mouse_mode);
        let background_color = match self.is_element_self(hovered_element) || self.is_element_self(focused_element) {
            true if highlight => Color::rgba_u8(60, 160, 160, 255),
            true if matches!(mouse_mode, MouseInputMode::None) => theme.button.hovered_background_color.get(),
            false if highlight => Color::rgba_u8(160, 160, 60, 255),
            _ => theme.button.background_color.get(),
        };

        renderer.render_background(CornerRadius::uniform(5.0), background_color);

        if let Some(item) = &self.item {
            renderer.renderer.render_sprite(
                renderer.render_target,
                item.texture.clone(),
                renderer.position,
                ScreenSize::uniform(30.0).scaled(Scaling::new(application.get_scaling_factor())),
                renderer.clip,
                Color::monochrome_u8(255),
                false,
            );

            renderer.render_text(
                //&format!("{}", self.item.amount),
                "1",
                ScreenPosition::default(),
                theme.button.foreground_color.get(),
                FontSize::new(8.0),
            );
        }
    }
}
