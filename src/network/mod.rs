use cgmath::Vector2;

use std::net::{ IpAddr, Ipv4Addr, Ipv6Addr };
use std::time::Duration;

use pnet::datalink::{ self, NetworkInterface, DataLinkReceiver, Config };
use pnet::packet::ethernet::{ EtherTypes, EthernetPacket };
use pnet::packet::ip::{ IpNextHeaderProtocol, IpNextHeaderProtocols };
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::ipv6::Ipv6Packet;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::Packet;

#[derive(Clone, Debug)]
pub enum NetworkEvent {
    PlayerMove(/*Timestamp?, */Vector2<usize>, Vector2<usize>),
}

fn handle_tcp_packet(interface_name: &str, packet_data: &[u8]) -> Option<NetworkEvent> {

    if let Some(packet) = TcpPacket::new(packet_data) {

        let payload = packet.payload();

        if payload.is_empty() {
            return None;
        }

        if payload[0] == 0x87 && payload[1] == 0x00 {

            // let timestamp = 4 bytes
            let coordinates = &payload[6..11];
            // let orientation = 1 byte (always 88)

            let y_position_to = (coordinates[4] as usize) | (((coordinates[3] as usize) & 0b11) << 8);
            let x_position_to = ((coordinates[3] as usize) >> 2) | (((coordinates[2] as usize) & 0b1111) << 6);
            let y_position_from = ((coordinates[2] as usize) >> 4) | (((coordinates[1] as usize) & 0b111111) << 4);
            let x_position_from = ((coordinates[1] as usize) >> 6) | ((coordinates[0] as usize) << 2);

            let position_from = Vector2::new(x_position_from, y_position_from);
            let position_to = Vector2::new(x_position_to, y_position_to);

            let event = NetworkEvent::PlayerMove(position_from, position_to);
            return Some(event);
        }
    } else {
        println!("[{}]: Malformed TCP Packet", interface_name);
    }

    return None;
}

fn handle_transport_protocol(interface_name: &str, protocol: IpNextHeaderProtocol, packet: &[u8]) -> Option<NetworkEvent> {
    match protocol {
        IpNextHeaderProtocols::Tcp => return handle_tcp_packet(interface_name, packet),
        _ignored => return None,
    }
}

fn handle_ipv4_packet(interface_name: &str, ethernet: &EthernetPacket) -> Option<NetworkEvent> {

    if let Some(header) = Ipv4Packet::new(ethernet.payload()) {

        let ipv4_source = IpAddr::V4(header.get_source());

        if ipv4_source != Ipv4Addr::new(51, 222, 245, 10) {
            return None;
        }

        return handle_transport_protocol(interface_name, header.get_next_level_protocol(), header.payload());
    } else {
        println!("[{}]: Malformed IPv4 Packet", interface_name);
    }

    return None;
}

fn handle_ipv6_packet(interface_name: &str, ethernet: &EthernetPacket) -> Option<NetworkEvent> {
    let header = Ipv6Packet::new(ethernet.payload());

    if let Some(header) = header {

        let ipv6_source = IpAddr::V6(header.get_source());

        if ipv6_source != Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0) {
            return None;
        }

        return handle_transport_protocol(interface_name, header.get_next_header(), header.payload());
    } else {
        println!("[{}]: Malformed IPv6 Packet", interface_name);
    }

    return None;
}

fn handle_ethernet_frame(interface: &NetworkInterface, ethernet: &EthernetPacket) -> Option<NetworkEvent> {
    match ethernet.get_ethertype() {
        EtherTypes::Ipv4 => return handle_ipv4_packet(&interface.name, ethernet),
        EtherTypes::Ipv6 => return handle_ipv6_packet(&interface.name, ethernet),
        _other => return None,
    }
}

pub struct NetworkingSystem {
    interface: NetworkInterface,
    rx: Box<dyn DataLinkReceiver>,
}

impl NetworkingSystem {

    pub fn new() -> Self {

        use pnet::datalink::Channel::Ethernet;

        let interface_names_match = |interface: &NetworkInterface| interface.name == "enp0s31f6";

        let interfaces = datalink::interfaces();
        let interface = interfaces
            .into_iter()
            .filter(interface_names_match)
            .next()
            .unwrap_or_else(|| panic!("No such network interface: {}", "enp0s31f6"));

        let mut config: Config = Default::default();
        config.read_timeout = Some(Duration::from_millis(1));

        let (_, rx) = match datalink::channel(&interface, config) {
            Ok(Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => panic!("packetdump: unhandled channel type"),
            Err(e) => panic!("packetdump: unable to create channel: {}", e),
        };

        return Self { interface, rx };
    }

    pub fn network_events(&mut self) -> Vec<NetworkEvent> {
        let mut events = Vec::new();

        while let Ok(packet) = self.rx.next() {
            if let Some(event) = handle_ethernet_frame(&self.interface, &EthernetPacket::new(packet).unwrap()) {
                events.push(event);
            }
        }

        return events;
    }
}
