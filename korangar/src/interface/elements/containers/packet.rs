use std::cell::RefCell;
use std::fmt::{Display, Formatter, Result};
use std::rc::{Rc, Weak};
use std::sync::{LazyLock, Mutex, MutexGuard};

use korangar_debug::profiling::RingBuffer;
use korangar_interface::application::Application;
use korangar_interface::elements::{ContainerState, Element, ElementCell, ElementState, ElementWrap, Expandable, Focus, PrototypeElement};
use korangar_interface::event::{ChangeEvent, HoverInformation};
use korangar_interface::layout::PlacementResolver;
use korangar_interface::size_bound;
use korangar_interface::state::{PlainRemote, Remote, RemoteClone};
use ragnarok_bytes::{ByteReader, ConversionError, ConversionResult, FromBytes};
use ragnarok_packets::handler::PacketCallback;
use ragnarok_packets::{Packet, PacketHeader};

use crate::input::MouseInputMode;
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{ScreenClip, ScreenPosition, ScreenSize};
use crate::interface::linked::LinkedElement;
use crate::interface::theme::InterfaceTheme;
use crate::renderer::InterfaceRenderer;

#[derive(Debug, Clone)]
struct UnknownPacket {
    pub bytes: Vec<u8>,
}

impl Packet for UnknownPacket {
    const HEADER: PacketHeader = PacketHeader(0);
    const IS_PING: bool = false;

    fn payload_from_bytes<Meta>(byte_reader: &mut ByteReader<Meta>) -> ConversionResult<Self> {
        let _ = byte_reader;
        unimplemented!()
    }

    fn payload_to_bytes(&self) -> ConversionResult<Vec<u8>> {
        unimplemented!()
    }

    fn to_prototype_element<App: Application>(&self) -> Box<dyn PrototypeElement<App> + Send> {
        Box::new(self.clone())
    }
}

impl<App: Application> PrototypeElement<App> for UnknownPacket {
    fn to_element(&self, display: String) -> ElementCell<App> {
        let mut byte_reader = ByteReader::<()>::without_metadata(&self.bytes);

        let elements = match self.bytes.len() >= 2 {
            true => {
                let signature = PacketHeader::from_bytes(&mut byte_reader).unwrap();
                let header = format!("0x{:0>4x}", signature.0);
                let data = &self.bytes[byte_reader.get_offset()..];

                vec![header.to_element("header".to_owned()), data.to_element("data".to_owned())]
            }
            false => {
                vec![self.bytes.to_element("data".to_owned())]
            }
        };

        Expandable::new(display, elements, false).wrap()
    }
}

#[derive(Debug, Clone)]
struct ErrorPacket {
    pub bytes: Vec<u8>,
    pub error: Box<ConversionError>,
}

impl Packet for ErrorPacket {
    const HEADER: PacketHeader = PacketHeader(0);
    const IS_PING: bool = false;

    fn payload_from_bytes<Meta>(byte_reader: &mut ByteReader<Meta>) -> ConversionResult<Self> {
        let _ = byte_reader;
        unimplemented!()
    }

    fn payload_to_bytes(&self) -> ConversionResult<Vec<u8>> {
        unimplemented!()
    }

    fn to_prototype_element<App: Application>(&self) -> Box<dyn PrototypeElement<App> + Send> {
        Box::new(self.clone())
    }
}

impl<App: Application> PrototypeElement<App> for ErrorPacket {
    fn to_element(&self, display: String) -> ElementCell<App> {
        let mut byte_reader = ByteReader::<()>::without_metadata(&self.bytes);
        let error = format!("{:?}", self.error);

        let elements = match self.bytes.len() >= 2 {
            true => {
                let signature = PacketHeader::from_bytes(&mut byte_reader).unwrap();
                let header = format!("0x{:0>4x}", signature.0);
                let data = &self.bytes[byte_reader.get_offset()..];

                vec![
                    header.to_element("header".to_owned()),
                    error.to_element("error".to_owned()),
                    data.to_element("data".to_owned()),
                ]
            }
            false => {
                vec![error.to_element("error".to_owned()), self.bytes.to_element("data".to_owned())]
            }
        };

        Expandable::new(display, elements, false).wrap()
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

struct PacketEntry {
    element: Box<dyn PrototypeElement<InterfaceSettings> + Send>,
    name: &'static str,
    is_ping: bool,
    direction: Direction,
}

impl PacketEntry {
    pub fn new_incoming(element: Box<dyn PrototypeElement<InterfaceSettings> + Send>, name: &'static str, is_ping: bool) -> Self {
        Self {
            element,
            name,
            is_ping,
            direction: Direction::Incoming,
        }
    }

    pub fn new_outgoing(element: Box<dyn PrototypeElement<InterfaceSettings> + Send>, name: &'static str, is_ping: bool) -> Self {
        Self {
            element,
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

type PacketHistoryBuffer = RingBuffer<(PacketEntry, LinkedElement), 256>;
type PacketHistoryBufferPointer = &'static Mutex<(PacketHistoryBuffer, usize)>;

#[derive(Clone)]
pub struct PacketHistoryCallback {
    buffer_pointer: PacketHistoryBufferPointer,
}

static BUFFER_POINTER: LazyLock<PacketHistoryBufferPointer> = LazyLock::new(|| Box::leak(Box::new(Mutex::new((RingBuffer::default(), 0)))));

impl PacketHistoryCallback {
    pub fn get_static_instance() -> Self {
        Self {
            buffer_pointer: &BUFFER_POINTER,
        }
    }

    pub fn remote(&self) -> PacketHistoryRemote {
        PacketHistoryRemote {
            buffer_pointer: self.buffer_pointer,
            version: 0,
        }
    }

    pub fn clear_all(&self) {
        let mut lock = self.buffer_pointer.lock().unwrap();

        lock.0.clear();
        lock.1 += 1;
    }
}

impl PacketCallback for PacketHistoryCallback {
    fn incoming_packet<Packet>(&self, packet: &Packet)
    where
        Packet: ragnarok_packets::Packet,
    {
        let mut lock = self.buffer_pointer.lock().unwrap();

        let prototype_element = packet.to_prototype_element();
        let entry = PacketEntry::new_incoming(prototype_element, std::any::type_name::<Packet>(), Packet::IS_PING);

        lock.0.push((entry, LinkedElement::new()));
        lock.1 += 1;
    }

    fn outgoing_packet<Packet>(&self, packet: &Packet)
    where
        Packet: ragnarok_packets::Packet,
    {
        let mut lock = self.buffer_pointer.lock().unwrap();

        let prototype_element = packet.to_prototype_element();
        let entry = PacketEntry::new_outgoing(prototype_element, std::any::type_name::<Packet>(), Packet::IS_PING);

        lock.0.push((entry, LinkedElement::new()));
        lock.1 += 1;
    }

    fn unknown_packet(&self, bytes: Vec<u8>) {
        let mut lock = self.buffer_pointer.lock().unwrap();

        let packet = UnknownPacket { bytes };
        let prototype_element = packet.to_prototype_element();
        let entry = PacketEntry::new_incoming(prototype_element, "^FF8810� Unknown �^000000", false);

        lock.0.push((entry, LinkedElement::new()));
        lock.1 += 1;
    }

    fn failed_packet(&self, bytes: Vec<u8>, error: Box<ConversionError>) {
        let mut lock = self.buffer_pointer.lock().unwrap();

        let packet = ErrorPacket { bytes, error };
        let prototype_element = packet.to_prototype_element();
        let entry = PacketEntry::new_incoming(prototype_element, "^FF4444✖ Error ✖^000000", false);

        lock.0.push((entry, LinkedElement::new()));
        lock.1 += 1;
    }
}

#[derive(Clone)]
pub struct PacketHistoryRemote {
    buffer_pointer: PacketHistoryBufferPointer,
    version: usize,
}

impl PacketHistoryRemote {
    pub fn consume_changed(&mut self) -> bool {
        let lock = self.buffer_pointer.lock().unwrap();

        let version = lock.1;
        let changed = version != self.version;
        self.version = version;

        changed
    }

    fn get(&self) -> MutexGuard<'_, (PacketHistoryBuffer, usize)> {
        self.buffer_pointer.lock().unwrap()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer_pointer.lock().unwrap().0.is_empty()
    }
}

pub struct PacketView {
    packets: PacketHistoryRemote,
    show_pings: PlainRemote<bool>,
    state: ContainerState<InterfaceSettings>,
}

impl PacketView {
    pub fn new(packets: PacketHistoryRemote, show_pings: PlainRemote<bool>) -> Self {
        let elements = {
            let packets = packets.get();
            let show_pings = show_pings.cloned();

            packets
                .0
                .iter()
                .filter_map(|(packet, linked_element)| {
                    let show_packet = show_pings || !packet.is_ping();

                    match show_packet {
                        true => {
                            let element = PacketEntry::to_element(packet);
                            linked_element.link(&element);
                            Some(element)
                        }
                        false => {
                            linked_element.link_hidden();
                            None
                        }
                    }
                })
                .collect()
        };

        Self {
            packets,
            show_pings,
            state: ContainerState::new(elements),
        }
    }
}

impl Element<InterfaceSettings> for PacketView {
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
            // Remove elements of packets that are no longer in the list.
            if let Some(first_visible_packet) = self.packets.get().0.iter().find(|(_, linked_element)| !linked_element.is_hidden()) {
                for _index in 0..self.state.elements.len() {
                    if !first_visible_packet.1.is_linked_to(&self.state.elements[0]) {
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
            self.packets.get().0.iter().for_each(|(packet, linked_element)| {
                // Getting here means that the packet was already processed once.
                let show_packet = show_pings || !packet.is_ping();

                if linked_element.is_linked() {
                    let was_hidden = linked_element.is_hidden();

                    // Packet was previously hidden but should be visible now.
                    if show_packet && was_hidden {
                        let element = PacketEntry::to_element(packet);
                        linked_element.link(&element);
                        element
                            .borrow_mut()
                            .link_back(Rc::downgrade(&element), self.state.state.self_element.clone());

                        self.state.elements.insert(index, element);
                        resolve = true;
                    }

                    // Packet was previously visible but now should be hidden.
                    if !show_packet && !was_hidden {
                        linked_element.link_hidden();

                        self.state.elements.remove(index);
                        resolve = true;
                    }
                } else {
                    // Getting here means that the packet was newly added.
                    match show_packet {
                        true => {
                            let element = PacketEntry::to_element(packet);
                            linked_element.link(&element);
                            element
                                .borrow_mut()
                                .link_back(Rc::downgrade(&element), self.state.state.self_element.clone());

                            self.state.elements.push(element);
                            resolve = true;
                        }
                        false => {
                            linked_element.link_hidden();
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
            .element_renderer(renderer, application, parent_position, screen_clip);

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
