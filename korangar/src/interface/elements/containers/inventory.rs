use korangar_interface::elements::{ContainerState, Element, ElementCell, ElementState, ElementWrap, Focus, WeakElementCell};
use korangar_interface::event::{ChangeEvent, HoverInformation};
use korangar_interface::layout::PlacementResolver;
use korangar_interface::size_bound;
use korangar_interface::state::{PlainRemote, Remote};

use crate::graphics::{Color, InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::application::InterfaceSettings;
use crate::interface::elements::ItemBox;
use crate::interface::layout::{CornerRadius, ScreenClip, ScreenPosition, ScreenSize};
use crate::interface::resource::{ItemSource, Move, PartialMove};
use crate::interface::theme::InterfaceTheme;
use crate::inventory::Item;

pub struct InventoryContainer {
    items: PlainRemote<Vec<Item>>,
    weak_self: Option<WeakElementCell<InterfaceSettings>>, // TODO: maybe remove?
    state: ContainerState<InterfaceSettings>,
}

impl InventoryContainer {
    pub fn new(items: PlainRemote<Vec<Item>>) -> Self {
        let elements = {
            let items = items.get();

            (0..40)
                .map(|index| items.get(index).cloned())
                .map(|item| ItemBox::new(item, ItemSource::Inventory, Box::new(|_| false)))
                .map(ElementWrap::wrap)
                .collect()
        };

        let weak_self = None;
        let state = ContainerState::new(elements);

        Self { items, weak_self, state }
    }
}

impl Element<InterfaceSettings> for InventoryContainer {
    fn get_state(&self) -> &ElementState<InterfaceSettings> {
        &self.state.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<InterfaceSettings> {
        &mut self.state.state
    }

    fn link_back(&mut self, weak_self: WeakElementCell<InterfaceSettings>, weak_parent: Option<WeakElementCell<InterfaceSettings>>) {
        self.weak_self = Some(weak_self.clone());
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
            .resolve(placement_resolver, application, theme, size_bound, ScreenSize::uniform(3.0));
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

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation<InterfaceSettings> {
        match mouse_mode {
            MouseInputMode::MoveItem(..) => self.state.state.hovered_element(mouse_position),
            MouseInputMode::None => self.state.hovered_element(mouse_position, mouse_mode, false),
            _ => HoverInformation::Missed,
        }
    }

    fn drop_resource(&mut self, drop_resource: PartialMove) -> Option<Move> {
        let PartialMove::Item { source, item } = drop_resource else {
            return None;
        };

        (source != ItemSource::Inventory).then_some(Move::Item {
            source,
            destination: ItemSource::Inventory,
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
        second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .state
            .element_renderer(render_target, renderer, application, parent_position, screen_clip);

        self.state.render(
            &mut renderer,
            application,
            theme,
            hovered_element,
            focused_element,
            mouse_mode,
            second_theme,
        );

        if matches!(mouse_mode, MouseInputMode::MoveItem(..)) {
            match self.is_element_self(hovered_element) {
                true => renderer.render_background(CornerRadius::uniform(5.0), Color::rgba_u8(60, 160, 160, 160)),
                false => renderer.render_background(CornerRadius::uniform(5.0), Color::rgba_u8(160, 160, 60, 160)),
            }
        }
    }
}
