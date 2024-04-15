use std::cell::{RefCell, UnsafeCell};
use std::fmt::{Display, Formatter, Result};
use std::rc::{Rc, Weak};

use korangar_interface::elements::{
    ContainerState, Element, ElementCell, ElementState, ElementWrap, Focus, PrototypeElement, WeakElementCell,
};
use korangar_interface::event::{ChangeEvent, HoverInformation};
use korangar_interface::layout::PlacementResolver;
use korangar_interface::state::{PlainRemote, Remote, RemoteClone};
use korangar_procedural::size_bound;

use crate::debug::RingBuffer;
use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{ScreenClip, ScreenPosition, ScreenSize};
use crate::interface::theme::InterfaceTheme;

struct HiddenElement;

impl Element<InterfaceSettings> for HiddenElement {
    fn get_state(&self) -> &ElementState<InterfaceSettings> {
        unimplemented!()
    }

    fn get_state_mut(&mut self) -> &mut ElementState<InterfaceSettings> {
        unimplemented!()
    }

    fn resolve(
        &mut self,
        _placement_resolver: &mut PlacementResolver<InterfaceSettings>,
        _application: &InterfaceSettings,
        _theme: &InterfaceTheme,
    ) {
        unimplemented!()
    }

    fn render(
        &self,
        _render_target: &mut <InterfaceRenderer as Renderer>::Target,
        _render: &InterfaceRenderer,
        _application: &InterfaceSettings,
        _theme: &InterfaceTheme,
        _parent_position: ScreenPosition,
        _screen_clip: ScreenClip,
        _hovered_element: Option<&dyn Element<InterfaceSettings>>,
        _focused_element: Option<&dyn Element<InterfaceSettings>>,
        _mouse_mode: &MouseInputMode,
        _second_theme: bool,
    ) {
        unimplemented!()
    }
}

enum Direction {
    Incoming,
    Outgoing,
}

impl Display for Direction {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Direction::Incoming => write!(f, "[^66FF44in^000000]"),
            Direction::Outgoing => write!(f, "[^FF7744out^000000]"),
        }
    }
}

pub struct PacketEntry {
    element: Box<dyn PrototypeElement<InterfaceSettings>>,
    name: &'static str,
    is_ping: bool,
    direction: Direction,
}

impl PacketEntry {
    pub fn new_incoming(element: &(impl PrototypeElement<InterfaceSettings> + Clone + 'static), name: &'static str, is_ping: bool) -> Self {
        Self {
            element: Box::new(element.clone()),
            name,
            is_ping,
            direction: Direction::Incoming,
        }
    }

    pub fn new_outgoing(element: &(impl PrototypeElement<InterfaceSettings> + Clone + 'static), name: &'static str, is_ping: bool) -> Self {
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

    fn to_element(&self) -> ElementCell<InterfaceSettings> {
        self.element.to_element(format!("{} {}", self.direction, self.name))
    }
}

pub struct PacketView<const N: usize> {
    packets: PlainRemote<RingBuffer<(PacketEntry, UnsafeCell<Option<WeakElementCell<InterfaceSettings>>>), N>>,
    show_pings: PlainRemote<bool>,
    hidden_element: ElementCell<InterfaceSettings>,
    state: ContainerState<InterfaceSettings>,
}

impl<const N: usize> PacketView<N> {
    pub fn new(
        packets: PlainRemote<RingBuffer<(PacketEntry, UnsafeCell<Option<WeakElementCell<InterfaceSettings>>>), N>>,
        show_pings: PlainRemote<bool>,
    ) -> Self {
        let hidden_element = HiddenElement.wrap();
        let elements = {
            let packets = packets.get();
            let show_pings = show_pings.cloned();

            packets
                .iter()
                .filter_map(|(packet, linked_element)| {
                    let show_packet = show_pings || !packet.is_ping();

                    match show_packet {
                        true => {
                            let element = PacketEntry::to_element(packet);
                            unsafe { *linked_element.get() = Some(Rc::downgrade(&element)) };
                            Some(element)
                        }
                        false => {
                            unsafe { *linked_element.get() = Some(Rc::downgrade(&hidden_element)) };
                            None
                        }
                    }
                })
                .collect()
        };

        Self {
            packets,
            show_pings,
            hidden_element,
            state: ContainerState::new(elements),
        }
    }
}

impl<const N: usize> Element<InterfaceSettings> for PacketView<N> {
    fn get_state(&self) -> &ElementState<InterfaceSettings> {
        &self.state.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<InterfaceSettings> {
        &mut self.state.state
    }

    fn link_back(
        &mut self,
        weak_self: Weak<RefCell<dyn Element<InterfaceSettings>>>,
        weak_parent: Option<Weak<RefCell<dyn Element<InterfaceSettings>>>>,
    ) {
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
        self.state.resolve(
            placement_resolver,
            application,
            theme,
            &size_bound!(100%, ?),
            ScreenSize::default(),
        );
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        let mut resolve = false;

        if self.show_pings.consume_changed() | self.packets.consume_changed() {
            fn compare(
                linked_element: &UnsafeCell<Option<WeakElementCell<InterfaceSettings>>>,
                element: &ElementCell<InterfaceSettings>,
            ) -> bool {
                let linked_element = unsafe { &*linked_element.get() };
                let linked_element = linked_element.as_ref().map(|weak| weak.as_ptr());
                linked_element.is_some_and(|pointer| !std::ptr::addr_eq(pointer, Rc::downgrade(element).as_ptr()))
            }

            // Remove elements of packets that are no longer in the list.
            if let Some(first_visible_packet) = self
                .packets
                .get()
                .iter()
                .find(|(_, linked_element)| compare(linked_element, &self.hidden_element))
            {
                let first_visible_element = unsafe { &*first_visible_packet.1.get() };
                let first_visible_element = first_visible_element.as_ref().unwrap().as_ptr();

                for _index in 0..self.state.elements.len() {
                    if !std::ptr::addr_eq(first_visible_element, Rc::downgrade(&self.state.elements[0]).as_ptr()) {
                        self.state.elements.remove(0);
                        resolve = true;
                    } else {
                        break;
                    }
                }
            } else {
                // This means that there are no visible packets at all, so remove every element.
                self.state.elements.clear();
                resolve = true;
            }

            let show_pings = self.show_pings.cloned();
            let mut index = 0;

            // Add or remove elements that need to be shown/hidden based on filtering. Also
            // append new elements for packets that are new.
            self.packets.get().iter().for_each(|(packet, linked_element)| {
                // Getting here means thatt the packet was already processed once.
                let show_packet = show_pings || !packet.is_ping();

                if let Some(linked_element) = unsafe { &mut (*linked_element.get()) } {
                    let was_hidden = std::ptr::addr_eq(linked_element.as_ptr(), Rc::downgrade(&self.hidden_element).as_ptr());

                    // Packet was previously hidden but should be visible now.
                    if show_packet && was_hidden {
                        let element = PacketEntry::to_element(packet);
                        *linked_element = Rc::downgrade(&element);
                        element
                            .borrow_mut()
                            .link_back(Rc::downgrade(&element), self.state.state.self_element.clone());

                        self.state.elements.insert(index, element);
                        resolve = true;
                    }

                    // Packet was previously visible but now should be hidden.
                    if !show_packet && !was_hidden {
                        *linked_element = Rc::downgrade(&self.hidden_element);

                        self.state.elements.remove(index);
                        resolve = true;
                    }
                } else {
                    // Getting here means thatt the packet was newly added.
                    match show_packet {
                        true => {
                            let element = PacketEntry::to_element(packet);
                            unsafe { *linked_element.get() = Some(Rc::downgrade(&element)) };
                            element
                                .borrow_mut()
                                .link_back(Rc::downgrade(&element), self.state.state.self_element.clone());

                            self.state.elements.push(element);
                            resolve = true;
                        }
                        false => {
                            unsafe { *linked_element.get() = Some(Rc::downgrade(&self.hidden_element)) };
                        }
                    }
                }

                if show_packet {
                    index += 1;
                }
            });
        }

        match resolve {
            true => Some(ChangeEvent::RESOLVE_WINDOW),
            false => None,
        }
    }

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation<InterfaceSettings> {
        match mouse_mode {
            MouseInputMode::None => self.state.hovered_element(mouse_position, mouse_mode, false),
            _ => HoverInformation::Missed,
        }
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
    }
}
