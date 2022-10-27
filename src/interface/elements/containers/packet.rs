
use std::fmt::{Display, Formatter, Result};
use std::rc::Weak;

use cgmath::Zero;
use procedural::*;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::{Element, *};

enum Direction {
    Incoming,
    Outgoing,
}

impl Display for Direction {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Direction::Incoming => write!(f, "[in]"),
            Direction::Outgoing => write!(f, "[out]"),
        }
    }
}

pub struct PacketEntry {
    element: Box<dyn PrototypeElement>,
    name: &'static str,
    is_ping: bool,
    direction: Direction,
}

impl PacketEntry {
    pub fn new_incoming(element: &(impl PrototypeElement + Clone + 'static), name: &'static str, is_ping: bool) -> Self {
        Self {
            element: Box::new(element.clone()),
            name,
            is_ping,
            direction: Direction::Incoming,
        }
    }

    pub fn new_outgoing(element: &(impl PrototypeElement + Clone + 'static), name: &'static str, is_ping: bool) -> Self {
        Self {
            element: Box::new(element.clone()),
            name,
            is_ping,
            direction: Direction::Outgoing,
        }
    }

    fn is_ping(&self) -> bool {
        self.is_ping
    }

    fn to_element(&self) -> ElementCell {
        self.element.to_element(format!("{} {}", self.direction, self.name))
    }
}

pub struct PacketView {
    packets: TrackedState<Vec<PacketEntry>>,
    cleared: Remote<()>,
    show_pings: Remote<bool>,
    update: Remote<bool>,
    weak_self: Option<WeakElementCell>,
    cached_packet_count: usize,
    state: ContainerState,
}

impl PacketView {
    pub fn new(packets: TrackedState<Vec<PacketEntry>>, cleared: Remote<()>, show_pings: Remote<bool>, update: Remote<bool>) -> Self {
        let weak_self = None;
        let (elements, cached_packet_count) = {
            let packets = packets.borrow();
            let elements = packets.iter().map(PacketEntry::to_element).collect();
            let cached_packet_count = packets.len();

            (elements, cached_packet_count)
        };

        Self {
            packets,
            cleared,
            show_pings,
            update,
            weak_self,
            cached_packet_count,
            state: ContainerState::new(elements),
        }
    }

    pub fn wrap(self) -> ElementCell {
        Rc::new(RefCell::new(self))
    }
}

impl Element for PacketView {
    fn get_state(&self) -> &ElementState {
        &self.state.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state.state
    }

    fn link_back(&mut self, weak_self: Weak<RefCell<dyn Element>>, weak_parent: Option<Weak<RefCell<dyn Element>>>) {
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
        self.state.resolve(
            placement_resolver,
            interface_settings,
            theme,
            &constraint!(100%, ?),
            Vector2::zero(),
        );
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        let mut reresolve = false;
        let mut packet_count = match *self.update.borrow() {
            true => self.packets.borrow().len(),
            false => self.cached_packet_count,
        };

        if self.cleared.consume_changed() {
            self.state.elements.clear();
            self.cached_packet_count = 0;
            packet_count = 0;
            reresolve = true;
        }

        if self.show_pings.consume_changed() {
            self.state.elements.clear();
            self.cached_packet_count = 0;
            reresolve = true;
        }

        if self.cached_packet_count < packet_count {
            let show_pings = *self.show_pings.borrow();
            let mut new_elements: Vec<ElementCell> = self.packets.borrow()[self.cached_packet_count..packet_count]
                .iter()
                .filter(|entry| show_pings || !entry.is_ping())
                .map(PacketEntry::to_element)
                .collect();

            new_elements.iter().for_each(|element| {
                let weak_element = Rc::downgrade(element);
                element.borrow_mut().link_back(weak_element, self.weak_self.clone());
            });

            self.state.elements.append(&mut new_elements);
            self.cached_packet_count = packet_count;
            reresolve = true;
        }

        match reresolve {
            true => Some(ChangeEvent::Reresolve), // TODO: ReresolveWindow
            false => None,
        }
    }

    fn hovered_element(&self, mouse_position: Position, mouse_mode: &MouseInputMode) -> HoverInformation {
        match mouse_mode {
            MouseInputMode::None => self.state.hovered_element(mouse_position, mouse_mode, false),
            _ => HoverInformation::Missed,
        }
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
    }
}
