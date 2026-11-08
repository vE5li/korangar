use std::collections::HashMap;

use ragnarok_bytes::{ByteReader, ConversionError, ConversionResult, FromBytes};

use crate::PacketHeader;

/// Possible results of [`PacketHandler::process_one`].
pub enum HandlerResult<Output> {
    /// Packet was successfully processed and produced some output.
    Ok(Output),
    /// No packet handler was registered for the incoming packet.
    UnhandledPacket,
    /// Packet was most likely cut-off.
    PacketCutOff,
    /// An error occurred inside the packet handler.
    InternalError(Box<ConversionError>),
}

/// Error when trying to register two separate handlers for the same packet.
#[derive(Debug, Clone)]
pub struct DuplicateHandlerError {
    /// Header of the packet.
    pub packet_header: PacketHeader,
}

/// Trait for monitoring the incoming and outgoing packets.
pub trait PacketCallback: Clone + 'static {
    /// Called by the [`PacketHandler`] when a packet is received.
    fn incoming_packet<Packet>(&self, packet: &Packet)
    where
        Packet: ragnarok_packets::Packet,
    {
        let _ = packet;
    }

    /// Called by when a packet is sent.
    fn outgoing_packet<Packet>(&self, packet: &Packet)
    where
        Packet: ragnarok_packets::Packet,
    {
        let _ = packet;
    }

    /// Called by the [`PacketHandler`] when a packet arrives that doesn't have
    /// a handler registered.
    fn unknown_packet(&self, bytes: Vec<u8>) {
        let _ = bytes;
    }

    /// A recognized wire frame has no semantic handler. Only its opcode is
    /// exposed.
    fn unsupported_packet(&self, _opcode: u16) {}

    /// Called by the [`PacketHandler`] when a packet handler returned an error.
    fn failed_packet(&self, bytes: Vec<u8>, error: Box<ConversionError>) {
        let _ = (bytes, error);
    }
}

#[derive(Debug, Default, Clone)]
pub struct NoPacketCallback;

impl PacketCallback for NoPacketCallback {}

pub type HandlerFunction<Output> = Box<dyn Fn(&mut ByteReader) -> ConversionResult<Output>>;

/// A struct to help with reading packets from a [`ByteReader`] and
/// converting them to some common event type.
///
/// It allows passing a packet callback to monitor incoming packets.
pub struct PacketHandler<Output, Callback> {
    handlers: HashMap<PacketHeader, HandlerFunction<Output>>,
    packet_callback: Callback,
    callbacks_enabled: bool,
}

impl<Output, Callback> Default for PacketHandler<Output, Callback>
where
    Callback: Default,
{
    fn default() -> Self {
        Self {
            handlers: Default::default(),
            packet_callback: Default::default(),
            callbacks_enabled: true,
        }
    }
}

impl<Output, Callback> PacketHandler<Output, Callback>
where
    Output: Default,
    Callback: PacketCallback,
{
    /// Create a new packet handler with a callback.
    pub fn with_callback(packet_callback: Callback) -> Self {
        Self {
            handlers: Default::default(),
            packet_callback,
            callbacks_enabled: true,
        }
    }

    /// Exclude authentication packets (including failures) from inspectors.
    /// Call before registering handlers, because handlers capture this setting.
    pub fn without_callbacks(mut self) -> Self {
        self.callbacks_enabled = false;
        self
    }

    pub fn has_handler(&self, header: PacketHeader) -> bool {
        self.handlers.contains_key(&header)
    }

    pub fn report_unsupported(&self, header: PacketHeader) {
        self.packet_callback.unsupported_packet(header.0);
    }

    /// Register a new packet handler.
    pub fn register<Packet, Return>(&mut self, handler: impl Fn(Packet) -> Return + 'static) -> Result<(), DuplicateHandlerError>
    where
        Packet: ragnarok_packets::Packet,
        Return: Into<Output>,
    {
        let packet_callback = self.packet_callback.clone();
        let callbacks_enabled = self.callbacks_enabled;
        let old_handler = self.handlers.insert(
            Packet::HEADER,
            Box::new(move |byte_reader| {
                let packet = Packet::payload_from_bytes(byte_reader)?;

                if callbacks_enabled {
                    packet_callback.incoming_packet(&packet);
                }

                Ok(handler(packet).into())
            }),
        );

        match old_handler.is_some() {
            true => Err(DuplicateHandlerError {
                packet_header: Packet::HEADER,
            }),
            false => Ok(()),
        }
    }

    /// Register a noop packet handler.
    pub fn register_noop<Packet>(&mut self) -> Result<(), DuplicateHandlerError>
    where
        Packet: ragnarok_packets::Packet,
    {
        let packet_callback = self.packet_callback.clone();
        let callbacks_enabled = self.callbacks_enabled;
        let old_handler = self.handlers.insert(
            Packet::HEADER,
            Box::new(move |byte_reader| {
                let packet = Packet::payload_from_bytes(byte_reader)?;

                if callbacks_enabled {
                    packet_callback.incoming_packet(&packet);
                }

                Ok(Output::default())
            }),
        );

        match old_handler.is_some() {
            true => Err(DuplicateHandlerError {
                packet_header: Packet::HEADER,
            }),
            false => Ok(()),
        }
    }

    /// Take a single packet from the byte stream.
    pub fn process_one(&mut self, byte_reader: &mut ByteReader) -> HandlerResult<Output> {
        let save_point = byte_reader.create_save_point();

        let Ok(header) = PacketHeader::from_bytes(byte_reader) else {
            // Packet is cut-off at the header.
            byte_reader.restore_save_point(save_point);
            return HandlerResult::PacketCutOff;
        };

        let Some(handler) = self.handlers.get(&header) else {
            byte_reader.restore_save_point(save_point);

            if self.callbacks_enabled {
                self.packet_callback.unknown_packet(byte_reader.remaining_bytes());
            }

            return HandlerResult::UnhandledPacket;
        };

        match handler(byte_reader) {
            Ok(output) => HandlerResult::Ok(output),
            // Cut-off packet (probably).
            Err(error) if error.is_byte_reader_too_short() => {
                byte_reader.restore_save_point(save_point);
                HandlerResult::PacketCutOff
            }
            Err(error) => {
                byte_reader.restore_save_point(save_point);

                if self.callbacks_enabled {
                    self.packet_callback.failed_packet(byte_reader.remaining_bytes(), error.clone());
                }

                HandlerResult::InternalError(error)
            }
        }
    }
}

#[cfg(test)]
mod security_tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use ragnarok_bytes::ByteWriter;

    use super::*;
    use crate::{LoginServerLoginPacket, PacketExt};
    #[derive(Clone, Default)]
    struct Inspector(Arc<AtomicUsize>);
    impl PacketCallback for Inspector {
        fn incoming_packet<P: crate::Packet>(&self, _: &P) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }

        fn unknown_packet(&self, _: Vec<u8>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }

        fn failed_packet(&self, _: Vec<u8>, _: Box<ConversionError>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    #[test]
    fn authentication_payloads_never_reach_inspectors() {
        let inspector = Inspector::default();
        let mut handler = PacketHandler::<(), _>::with_callback(inspector.clone()).without_callbacks();
        handler.register(|_: LoginServerLoginPacket| ()).unwrap();
        let mut writer = ByteWriter::new();
        LoginServerLoginPacket::new("synthetic".into(), "synthetic-only-password".into())
            .packet_to_bytes(&mut writer)
            .unwrap();
        let mut reader = ByteReader::without_metadata(writer.as_slice());
        assert!(matches!(handler.process_one(&mut reader), HandlerResult::Ok(())));
        let mut reader = ByteReader::without_metadata(&[0xFF, 0xFF, 1, 2]);
        assert!(matches!(handler.process_one(&mut reader), HandlerResult::UnhandledPacket));
        assert_eq!(inspector.0.load(Ordering::SeqCst), 0);
    }
}
