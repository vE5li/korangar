use cgmath::{Array, Vector4};
use procedural::*;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::*;
use crate::inventory::Item;

pub struct InventoryContainer {
    items: Remote<Vec<Item>>,
    weak_self: Option<WeakElementCell>, // TODO: maybe remove?
    state: ContainerState,
}

impl InventoryContainer {
    pub fn new(items: Remote<Vec<Item>>) -> Self {
        let elements = {
            let items = items.borrow();

            (0..40)
                .map(|index| items.get(index).cloned())
                .map(|item| ItemBox::new(item, ItemSource::Inventory, Box::new(|_| false)))
                .map(ItemBox::wrap)
                .collect()
        };

        let weak_self = None;
        let state = ContainerState::new(elements);

        Self { items, weak_self, state }
    }
}

impl Element for InventoryContainer {
    fn get_state(&self) -> &ElementState {
        &self.state.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state.state
    }

    fn link_back(&mut self, weak_self: WeakElementCell, weak_parent: Option<WeakElementCell>) {
        self.weak_self = Some(weak_self.clone());
        self.state.link_back(weak_self, weak_parent);
    }

    fn is_focusable(&self) -> bool {
        self.state.is_focusable::<false>()
    }

    fn focus_next(&self, self_cell: ElementCell, caller_cell: Option<ElementCell>, focus: Focus) -> Option<ElementCell> {
        self.state.focus_next::<false>(self_cell, caller_cell, focus)
    }

    fn restore_focus(&self, self_cell: ElementCell) -> Option<ElementCell> {
        self.state.restore_focus(self_cell)
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, interface_settings: &InterfaceSettings, theme: &Theme) {
        let size_constraint = &constraint!(100%, ?);
        self.state.resolve(
            placement_resolver,
            interface_settings,
            theme,
            size_constraint,
            Vector2::from_value(3.0),
        );
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        if self.items.consume_changed() {
            let weak_parent = self.state.state.parent_element.take();
            let weak_self = self.weak_self.take().unwrap();

            *self = Self::new(self.items.clone());
            // important: link back after creating elements, otherwise focus navigation and
            // scrolling would break
            self.link_back(weak_self, weak_parent);

            return Some(ChangeEvent::RESOLVE_WINDOW);
        }

        None
    }

    fn hovered_element(&self, mouse_position: Position, mouse_mode: &MouseInputMode) -> HoverInformation {
        match mouse_mode {
            MouseInputMode::MoveItem(..) => self.state.state.hovered_element(mouse_position),
            MouseInputMode::None => self.state.hovered_element(mouse_position, mouse_mode, false),
            _ => HoverInformation::Missed,
        }
    }

    fn drop_item(&mut self, item_source: ItemSource, item: Item) -> Option<ItemMove> {
        Some(ItemMove {
            source: item_source,
            destination: ItemSource::Inventory,
            item,
        })
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        theme: &Theme,
        parent_position: Position,
        clip_size: ClipSize,
        hovered_element: Option<&dyn Element>,
        focused_element: Option<&dyn Element>,
        mouse_mode: &MouseInputMode,
        second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .state
            .element_renderer(render_target, renderer, interface_settings, parent_position, clip_size);

        self.state.render(
            &mut renderer,
            state_provider,
            interface_settings,
            theme,
            hovered_element,
            focused_element,
            mouse_mode,
            second_theme,
        );

        if matches!(mouse_mode, MouseInputMode::MoveItem(..)) {
            match self.is_element_self(hovered_element) {
                true => renderer.render_background(Vector4::from_value(5.0), Color::rgba(60, 160, 160, 160)),
                false => renderer.render_background(Vector4::from_value(5.0), Color::rgba(160, 160, 60, 160)),
            }
        }
    }
}
