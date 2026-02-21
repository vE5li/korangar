use std::cell::UnsafeCell;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::mpsc::TryRecvError;

use korangar_debug::logging::{Colorize, print_debug};
use korangar_interface::application::{Application, Size};
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, StateElement};
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::prelude::*;
use korangar_interface::theme::theme;
use ragnarok_bytes::{ByteReader, ByteWriter, ConversionError, ConversionResult};
use ragnarok_packets::handler::PacketCallback;
use ragnarok_packets::{Packet, PacketHeader};
use rust_state::{DowncastExt, ManuallyAssertExt, Path, RustState, VecIndexExt};

use crate::client_state;
use crate::state::{ClientState, ClientStatePathExt};

struct MaybeHeader<P> {
    path: P,
    cached: Option<[u8; 2]>,
    text: String,
}

impl<P> MaybeHeader<P> {
    fn new(path: P) -> Self {
        Self {
            path,
            cached: None,
            text: String::new(),
        }
    }
}

impl<App, P> Element<App> for MaybeHeader<P>
where
    App: Application,
    P: Path<App, Vec<u8>>,
{
    fn create_layout_info(
        &mut self,
        state: &rust_state::Context<App>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<App>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let height = *state.get(&theme().text().height());

            let data = state.get(&self.path);
            if data.len() >= 2 {
                if self.cached.is_none_or(|cached| cached[0..2] != data[0..2]) {
                    self.text = format!("0x{:0>4x}", u16::from_le_bytes([data[0], data[1]]));
                    self.cached = Some([data[0], data[1]]);
                }
            } else if self.cached.is_some() || self.text.is_empty() {
                self.text = "<cut off>".to_owned();
                self.cached = None;
            }

            let font_size = *state.get(&theme().text().font_size());
            let color = *state.get(&theme().text().color());
            let highlight_color = *state.get(&theme().text().highlight_color());
            let horizontal_alignment = *state.get(&theme().text().horizontal_alignment());
            let overflow_behavior = *state.get(&theme().text().overflow_behavior());

            let (size, font_size) = resolver.get_text_dimensions(
                &self.text,
                color,
                highlight_color,
                font_size,
                horizontal_alignment,
                overflow_behavior,
            );
            let area = resolver.with_height(height.max(size.height()));

            Self::LayoutInfo { area, font_size }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a rust_state::Context<App>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, App>,
    ) {
        layout.add_text(
            layout_info.area,
            &self.text,
            layout_info.font_size,
            *state.get(&theme().text().color()),
            *state.get(&theme().text().highlight_color()),
            // TODO: Check if we really want it like this.
            *state.get(&theme().text().horizontal_alignment()),
            // TODO: Check if we really want it like this.
            *state.get(&theme().text().vertical_alignment()),
            // TODO: Check if we really want it like this.
            *state.get(&theme().text().overflow_behavior()),
        );
    }
}

struct ErrorMessage<P> {
    path: P,
    cached: Option<Box<ConversionError>>,
    text: String,
}

impl<P> ErrorMessage<P> {
    fn new(path: P) -> Self {
        Self {
            path,
            cached: None,
            text: String::new(),
        }
    }
}

impl<App, P> Element<App> for ErrorMessage<P>
where
    App: Application,
    P: Path<App, Box<ConversionError>>,
{
    fn create_layout_info(
        &mut self,
        state: &rust_state::Context<App>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<App>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let height = *state.get(&theme().text().height());

            let error = state.get(&self.path);
            if !self.cached.as_ref().is_some_and(|cached| cached == error) {
                self.text = format!("{error:?}");
                self.cached = Some(error.clone());
            }

            let font_size = *state.get(&theme().text().font_size());
            let color = *state.get(&theme().text().color());
            let highlight_color = *state.get(&theme().text().highlight_color());
            let horizontal_alignment = *state.get(&theme().text().horizontal_alignment());
            let overflow_behavior = *state.get(&theme().text().overflow_behavior());

            let (size, font_size) = resolver.get_text_dimensions(
                &self.text,
                color,
                highlight_color,
                font_size,
                horizontal_alignment,
                overflow_behavior,
            );
            let area = resolver.with_height(height.max(size.height()));

            Self::LayoutInfo { area, font_size }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a rust_state::Context<App>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, App>,
    ) {
        layout.add_text(
            layout_info.area,
            &self.text,
            layout_info.font_size,
            *state.get(&theme().text().color()),
            *state.get(&theme().text().highlight_color()),
            // TODO: Check if we really want it like this.
            *state.get(&theme().text().horizontal_alignment()),
            // TODO: Check if we really want it like this.
            *state.get(&theme().text().vertical_alignment()),
            // TODO: Check if we really want it like this.
            *state.get(&theme().text().overflow_behavior()),
        );
    }
}

#[derive(Debug, Clone, RustState)]
struct UnknownPacket {
    pub bytes: Vec<u8>,
}

impl Packet for UnknownPacket {
    const HEADER: PacketHeader = PacketHeader(0);
    const IS_PING: bool = false;

    fn payload_from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        let _ = byte_reader;
        unimplemented!()
    }

    fn payload_to_bytes(&self, _byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        unimplemented!()
    }

    fn to_element<App: korangar_interface::application::Application>(
        self_path: impl Path<App, Self>,
        name: String,
    ) -> Box<dyn korangar_interface::element::Element<App, LayoutInfo = ()>> {
        use korangar_interface::prelude::*;

        ErasedElement::new(collapsible! {
            text: name,
            children: (
                split! {
                    children: (
                        text! {
                            text: "header",
                        },
                        MaybeHeader::new(self_path.bytes()),
                    )
                },
                // TODO: Currently this data includes the header which was previously not the
                // case if we had more than 2 bytes. Ideally, we could go back to that
                // behavior.
                StateElement::to_element(self_path.bytes(), "data".to_owned()),
            ),
        })
    }
}

#[derive(Debug, Clone, RustState)]
struct ErrorPacket {
    pub bytes: Vec<u8>,
    pub error: Box<ConversionError>,
}

impl Packet for ErrorPacket {
    const HEADER: PacketHeader = PacketHeader(0);
    const IS_PING: bool = false;

    fn payload_from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        let _ = byte_reader;
        unimplemented!()
    }

    fn payload_to_bytes(&self, _byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        unimplemented!()
    }

    fn to_element<App: korangar_interface::application::Application>(
        self_path: impl Path<App, Self>,
        name: String,
    ) -> Box<dyn korangar_interface::element::Element<App, LayoutInfo = ()>> {
        use korangar_interface::prelude::*;

        ErasedElement::new(collapsible! {
            text: name,
            children: (
                split! {
                    children: (
                        text! {
                            text: "header",
                        },
                        MaybeHeader::new(self_path.bytes()),
                    )
                },
                split! {
                    children: (
                        text! {
                            text: "error",
                        },
                        ErrorMessage::new(self_path.error()),
                    )
                },
                // TODO: Currently this data includes the header which was previously not the
                // case if we had more than 2 bytes. Ideally, we could go back to that
                // behavior.
                StateElement::to_element(self_path.bytes(), "data".to_owned()),
            ),
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, StateElement)]
pub enum Direction {
    Incoming,
    Outgoing,
}

#[derive(RustState, StateElement)]
pub struct PacketEntry {
    /// Stores the data of the packet.
    #[hidden_element]
    packet: Box<dyn std::any::Any>,
    /// Stores the UI element.
    // TODO: Unfortunately this has to be an unsafe cell as of now. Ideally this can be changed
    // later.
    #[hidden_element]
    pub element: UnsafeCell<Box<dyn Element<ClientState, LayoutInfo = ()>>>,
    pub unique_id: u64,
    is_ping: bool,
    direction: Direction,
}

impl PacketEntry {
    // TODO: This _might_ be problematic if two packets are added quickly enough to
    // get the same hash.
    fn create_unique_id() -> u64 {
        let mut hasher = DefaultHasher::new();
        std::time::Instant::now().hash(&mut hasher);
        hasher.finish()
    }

    pub fn new_incoming<P: Packet>(packet: P, packet_path: impl Path<ClientState, P>, name: &'static str, is_ping: bool) -> Self {
        let element = UnsafeCell::new(P::to_element(packet_path, format!("[^000001in^000000] {name}")));
        let packet = Box::new(packet);

        Self {
            packet,
            element,
            unique_id: Self::create_unique_id(),
            is_ping,
            direction: Direction::Incoming,
        }
    }

    pub fn new_outgoing<P: Packet>(packet: P, packet_path: impl Path<ClientState, P>, name: &'static str, is_ping: bool) -> Self {
        let element = UnsafeCell::new(P::to_element(packet_path, format!("[^000001out^000000] {name}")));
        let packet = Box::new(packet);

        Self {
            packet,
            element,
            unique_id: Self::create_unique_id(),
            is_ping,
            direction: Direction::Outgoing,
        }
    }

    pub fn is_ping(&self) -> bool {
        self.is_ping
    }

    pub fn is_incoming(&self) -> bool {
        self.direction == Direction::Incoming
    }

    pub fn is_outgoing(&self) -> bool {
        self.direction == Direction::Outgoing
    }
}

type PacketApplicator = Box<dyn FnOnce(&mut PacketHistory) + Send>;

#[derive(Clone)]
pub struct PacketHistoryCallback {
    sender: std::sync::mpsc::Sender<PacketApplicator>,
}

#[derive(RustState, StateElement)]
pub struct PacketHistory {
    #[hidden_element]
    receiver: std::sync::mpsc::Receiver<PacketApplicator>,
    pub entries: Vec<PacketEntry>,
    pub show_incoming: bool,
    pub show_outgoing: bool,
    pub show_pings: bool,
}

impl PacketHistory {
    pub fn new() -> (PacketHistory, PacketHistoryCallback) {
        let (sender, receiver) = std::sync::mpsc::channel();

        let packet_history = PacketHistory {
            receiver,
            entries: Vec::default(),
            show_incoming: true,
            show_outgoing: true,
            show_pings: false,
        };
        let packet_history_callback = PacketHistoryCallback { sender };

        (packet_history, packet_history_callback)
    }

    pub fn update(&mut self, save_all: bool) {
        const BASE_BUFFER_SIZE: usize = 30;

        loop {
            match self.receiver.try_recv() {
                Ok(applicator) => {
                    if !save_all && self.entries.len() >= BASE_BUFFER_SIZE {
                        // let last_valid_index = self.entries.len() - BASE_BUFFER_SIZE;
                        // self.entries.drain(..last_valid_index);

                        // HACK: Ideally we don't want to clear every packe here but just thes that
                        // exceed the base amount. But due to how we do the path lookups this
                        // would cause the client to crash. I believe this can be improved by using
                        // a RingBuffer for the packets again.
                        self.entries.clear();
                    }

                    applicator(self);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    print_debug!(
                        "[{}] packet history channel disconnectd. New packets can not be inspected",
                        "error".red()
                    )
                }
            }
        }
    }

    pub fn get_entries(&self) -> &[PacketEntry] {
        &self.entries
    }

    pub fn clear_all(&mut self) {
        self.entries.clear();
    }
}

impl PacketCallback for PacketHistoryCallback {
    fn incoming_packet<Packet>(&self, packet: &Packet)
    where
        Packet: ragnarok_packets::Packet,
    {
        let packet: Packet = packet.clone();

        // NOTE: Since this is just for debugging purposes we don't care if sending the
        // packet failed, so we discard the result.
        let _ = self.sender.send(Box::new(move |receiver: &mut PacketHistory| {
            let index = receiver.entries.len();
            let path = client_state()
                .packet_history()
                .entries()
                .index(index)
                .packet()
                .downcast::<Packet>()
                // NOTE: This should be safe since the element will be removed at the same time
                // as the packet entry. For any point in time before that this will be a safe
                // lookup.
                .manually_asserted();

            let entry = PacketEntry::new_incoming(packet, path, std::any::type_name::<Packet>(), Packet::IS_PING);

            receiver.entries.push(entry);
        }));
    }

    fn outgoing_packet<Packet>(&self, packet: &Packet)
    where
        Packet: ragnarok_packets::Packet,
    {
        let packet: Packet = packet.clone();

        // NOTE: Since this is just for debugging purposes we don't care if sending the
        // packet failed, so we discard the result.
        let _ = self.sender.send(Box::new(move |receiver: &mut PacketHistory| {
            let index = receiver.entries.len();
            let path = client_state()
                .packet_history()
                .entries()
                .index(index)
                .packet()
                .downcast::<Packet>()
                // NOTE: This should be safe since the element will be removed at the same time
                // as the packet entry. For any point in time before that this will be a safe
                // lookup.
                .manually_asserted();

            let entry = PacketEntry::new_outgoing(packet, path, std::any::type_name::<Packet>(), Packet::IS_PING);

            receiver.entries.push(entry);
        }));
    }

    fn unknown_packet(&self, bytes: Vec<u8>) {
        let packet = UnknownPacket { bytes };

        // NOTE: Since this is just for debugging purposes we don't care if sending the
        // packet failed, so we discard the result.
        let _ = self.sender.send(Box::new(move |receiver: &mut PacketHistory| {
            let index = receiver.entries.len();
            let path = client_state()
                .packet_history()
                .entries()
                .index(index)
                .packet()
                .downcast::<UnknownPacket>()
                // NOTE: This should be safe since the element will be removed at the same time
                // as the packet entry. For any point in time before that this will be a safe
                // lookup.
                .manually_asserted();

            let entry = PacketEntry::new_incoming(packet, path, "^000001Unknown^000000", UnknownPacket::IS_PING);

            receiver.entries.push(entry);
        }));
    }

    fn failed_packet(&self, bytes: Vec<u8>, error: Box<ConversionError>) {
        let packet = ErrorPacket { bytes, error };

        // NOTE: Since this is just for debugging purposes we don't care if sending the
        // packet failed, so we discard the result.
        let _ = self.sender.send(Box::new(move |receiver: &mut PacketHistory| {
            let index = receiver.entries.len();
            let path = client_state()
                .packet_history()
                .entries()
                .index(index)
                .packet()
                .downcast::<ErrorPacket>()
                // NOTE: This should be safe since the element will be removed at the same time
                // as the packet entry. For any point in time before that this will be a safe
                // lookup.
                .manually_asserted();

            let entry = PacketEntry::new_incoming(packet, path, "^000001Error^000000", ErrorPacket::IS_PING);

            receiver.entries.push(entry);
        }));
    }
}
