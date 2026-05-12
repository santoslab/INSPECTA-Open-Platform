#![cfg_attr(not(test), no_std)]
#![feature(proc_macro_hygiene)]
#![cfg_attr(not(verus_keep_ghost), feature(stmt_expr_attributes))]

use vstd::prelude::*;
use vstd::slice::slice_subrange;

mod net;
pub use net::{Address, ArpOp, HardwareType, Ipv4Address};
pub use net::{Arp, EtherType, EthernetRepr, IpProtocol, Ipv4Repr, TcpRepr, UdpRepr};
#[cfg(verus_keep_ghost)]
pub use net::{spec_ether_type_from_u16, spec_arp_op_from_u16, spec_hardware_type_from_u16, spec_ip_protocol_from_u8};


#[verus_verify]
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
pub enum PacketType {
    Arp(Arp),
    Ipv4(Ipv4Packet),
    Ipv6,
}

#[verus_verify]
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
pub enum Ipv4ProtoPacket {
    Tcp(TcpRepr),
    Udp(UdpRepr),
    HopByHop,
    Icmp,
    Igmp,
    Ipv6Route,
    Ipv6Frag,
    Icmpv6,
    Ipv6NoNxt,
    Ipv6Opts,
}

#[verus_verify]
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
pub struct Ipv4Packet {
    pub header: Ipv4Repr,
    pub protocol: Ipv4ProtoPacket,
}

#[verus_verify]
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
pub struct EthFrame {
    pub header: EthernetRepr,
    pub eth_type: PacketType,
}

verus! {
    pub const ARP_TOTAL: usize = 42;
    pub const IPV4_TOTAL: usize = 34;
    pub const TCP_TOTAL: usize = 54;
    pub const UDP_TOTAL: usize = 54;

    pub open spec fn is_arp_packet(pkt: PacketType) -> bool {
        match pkt {
            PacketType::Arp(_) => true,
            _ => false,
        }
    }

    pub open spec fn is_ipv4_packet(pkt: PacketType) -> bool {
        match pkt {
            PacketType::Ipv4(_) => true,
            _ => false,
        }
    }

    pub open spec fn is_ipv4_udp_packet(pkt: PacketType) -> bool {
        match pkt {
            PacketType::Ipv4(ip) => match ip.protocol {
                Ipv4ProtoPacket::Udp(_) => true,
                _ => false,
            },
            _ => false,
        }
    }

    pub open spec fn get_udp_src_port(pkt: PacketType) -> u16 {
        match pkt {
            PacketType::Ipv4(ip) => match ip.protocol {
                Ipv4ProtoPacket::Udp(udp) => udp.src_port,
                _ => 0u16,
            },
            _ => 0u16,
        }
    }

    pub open spec fn get_udp_dst_port(pkt: PacketType) -> u16 {
        match pkt {
            PacketType::Ipv4(ip) => match ip.protocol {
                Ipv4ProtoPacket::Udp(udp) => udp.dst_port,
                _ => 0u16,
            },
            _ => 0u16,
        }
    }
}


#[verus_verify]
impl EthFrame {
    #[verus_spec(r =>
        requires
            frame.len() >= TCP_TOTAL,
        ensures
            r.is_some() ==> (
                r.unwrap().header.dst_addr.0@ =~= frame@.subrange(0, 6 as int)
                && r.unwrap().header.src_addr.0@ =~= frame@.subrange(6, 12 as int)
                && spec_ether_type_from_u16(
                    ((frame@[12 as int] as u16) * 256 + (frame@[13 as int] as u16)) as u16
                ) == Some(r.unwrap().header.ethertype)
            ),
            // Variant correspondence: ethertype determines PacketType variant
            r.is_some() && r.unwrap().header.ethertype == EtherType::Arp
                ==> is_arp_packet(r.unwrap().eth_type),
            r.is_some() && r.unwrap().header.ethertype == EtherType::Ipv4
                ==> is_ipv4_packet(r.unwrap().eth_type),
            r.is_some() && r.unwrap().header.ethertype == EtherType::Ipv6
                ==> r.unwrap().eth_type == PacketType::Ipv6,
            // UDP port correspondence: parsed ports match byte positions
            r.is_some() && is_ipv4_udp_packet(r.unwrap().eth_type) ==> (
                get_udp_src_port(r.unwrap().eth_type)
                    == (frame@[34 as int] as u16) * 256 + (frame@[35 as int] as u16)
                && get_udp_dst_port(r.unwrap().eth_type)
                    == (frame@[36 as int] as u16) * 256 + (frame@[37 as int] as u16)
            ),
            // IPv4 protocol byte correspondence
            r.is_some() && is_ipv4_packet(r.unwrap().eth_type) ==> (
                spec_ip_protocol_from_u8(frame@[23 as int]).is_some()
                && frame@[14 as int] == 69u8
            ),
            // ARP completeness
            (
                !(frame@[0 as int] == 0u8 && frame@[1 as int] == 0u8 && frame@[2 as int] == 0u8
                  && frame@[3 as int] == 0u8 && frame@[4 as int] == 0u8 && frame@[5 as int] == 0u8)
                && spec_ether_type_from_u16(
                    ((frame@[12 as int] as u16) * 256 + (frame@[13 as int] as u16)) as u16
                ) == Some(EtherType::Arp)
                && spec_hardware_type_from_u16(
                    ((frame@[14 as int] as u16) * 256 + (frame@[15 as int] as u16)) as u16
                ).is_some()
                && spec_ether_type_from_u16(
                    ((frame@[16 as int] as u16) * 256 + (frame@[17 as int] as u16)) as u16
                ).is_some()
                && spec_ether_type_from_u16(
                    ((frame@[16 as int] as u16) * 256 + (frame@[17 as int] as u16)) as u16
                ) != Some(EtherType::Arp)
                && spec_arp_op_from_u16(
                    ((frame@[20 as int] as u16) * 256 + (frame@[21 as int] as u16)) as u16
                ).is_some()
            ) ==> r.is_some() && is_arp_packet(r.unwrap().eth_type),
            // IPv4 UDP completeness
            (
                !(frame@[0 as int] == 0u8 && frame@[1 as int] == 0u8 && frame@[2 as int] == 0u8
                  && frame@[3 as int] == 0u8 && frame@[4 as int] == 0u8 && frame@[5 as int] == 0u8)
                && spec_ether_type_from_u16(
                    ((frame@[12 as int] as u16) * 256 + (frame@[13 as int] as u16)) as u16
                ) == Some(EtherType::Ipv4)
                && frame@[14 as int] == 69u8
                && spec_ip_protocol_from_u8(frame@[23 as int]) == Some(IpProtocol::Udp)
                && (frame@[16 as int] as u16) * 256 + (frame@[17 as int] as u16) <= 9000u16
            ) ==> r.is_some() && is_ipv4_udp_packet(r.unwrap().eth_type),
            // Reverse: parse success implies dst addr not all zeros
            r.is_some() ==> !(frame@[0 as int] == 0u8 && frame@[1 as int] == 0u8
                && frame@[2 as int] == 0u8 && frame@[3 as int] == 0u8
                && frame@[4 as int] == 0u8 && frame@[5 as int] == 0u8),
            // Reverse: ARP ethertype bytes
            r.is_some() && is_arp_packet(r.unwrap().eth_type) ==> (
                frame@[12 as int] == 8u8 && frame@[13 as int] == 6u8
            ),
            // Reverse: ARP htype bytes (HardwareType::Ethernet = 0x0001)
            r.is_some() && is_arp_packet(r.unwrap().eth_type) ==> (
                frame@[14 as int] == 0u8 && frame@[15 as int] == 1u8
            ),
            // Reverse: ARP ptype bytes (Ipv4=0x0800 or Ipv6=0x86DD)
            r.is_some() && is_arp_packet(r.unwrap().eth_type) ==> (
                (frame@[16 as int] == 8u8 && frame@[17 as int] == 0u8)
                || (frame@[16 as int] == 134u8 && frame@[17 as int] == 221u8)
            ),
            // Reverse: ARP op bytes (Request=0x0001 or Reply=0x0002)
            r.is_some() && is_arp_packet(r.unwrap().eth_type) ==> (
                frame@[20 as int] == 0u8
                && (frame@[21 as int] == 1u8 || frame@[21 as int] == 2u8)
            ),
            // Reverse: ARP ptype bytes (Ipv4=0x0800 or Ipv6=0x86DD)
            r.is_some() && is_arp_packet(r.unwrap().eth_type) ==> (
                (frame@[16 as int] == 8u8 && frame@[17 as int] == 0u8)
                || (frame@[16 as int] == 134u8 && frame@[17 as int] == 221u8)
            ),
            // Reverse: IPv4 ethertype bytes
            r.is_some() && is_ipv4_packet(r.unwrap().eth_type) ==> (
                frame@[12 as int] == 8u8 && frame@[13 as int] == 0u8
            ),
            // Reverse: IPv4 vers_ihl and length in range
            r.is_some() && is_ipv4_packet(r.unwrap().eth_type) ==> (
                frame@[14 as int] == 69u8
                && (frame@[16 as int] as u16) * 256 + (frame@[17 as int] as u16) <= 9000u16
            ),
            // Reverse: IPv4 UDP protocol byte
            r.is_some() && is_ipv4_udp_packet(r.unwrap().eth_type) ==> (
                frame@[23 as int] == 17u8
            ),
            // Reverse: IPv4 protocol byte is valid
            r.is_some() && is_ipv4_packet(r.unwrap().eth_type) ==> (
                frame@[23 as int] == 0u8 || frame@[23 as int] == 1u8
                || frame@[23 as int] == 2u8 || frame@[23 as int] == 6u8
                || frame@[23 as int] == 17u8 || frame@[23 as int] == 43u8
                || frame@[23 as int] == 44u8 || frame@[23 as int] == 58u8
                || frame@[23 as int] == 59u8 || frame@[23 as int] == 60u8
            ),
    )]
    pub fn parse(frame: &[u8]) -> Option<EthFrame> {
        let header = EthernetRepr::parse(slice_subrange(frame, 0, EthernetRepr::SIZE))?;
        let eth_type = match header.ethertype {
             EtherType::Arp => {
                 let a = Arp::parse(slice_subrange(frame, EthernetRepr::SIZE, ARP_TOTAL))?;
                 PacketType::Arp(a)
             }
            EtherType::Ipv4 => {
                let ip = Ipv4Repr::parse(slice_subrange(frame, EthernetRepr::SIZE, IPV4_TOTAL))?;
                // TODO: Check that the entire IPv4 Packet is not malformed
                let protocol = match ip.protocol {
                    IpProtocol::Tcp => Ipv4ProtoPacket::Tcp(TcpRepr::parse(
                        slice_subrange(frame, IPV4_TOTAL, TCP_TOTAL),
                    )),
                    IpProtocol::Udp => Ipv4ProtoPacket::Udp(UdpRepr::parse(
                        slice_subrange(frame, IPV4_TOTAL, UDP_TOTAL),
                    )),
                    IpProtocol::HopByHop => Ipv4ProtoPacket::HopByHop,
                    IpProtocol::Icmp => Ipv4ProtoPacket::Icmp,
                    IpProtocol::Igmp => Ipv4ProtoPacket::Igmp,
                    IpProtocol::Ipv6Route => Ipv4ProtoPacket::Ipv6Route,
                    IpProtocol::Ipv6Frag => Ipv4ProtoPacket::Ipv6Frag,
                    IpProtocol::Icmpv6 => Ipv4ProtoPacket::Icmpv6,
                    IpProtocol::Ipv6NoNxt => Ipv4ProtoPacket::Ipv6NoNxt,
                    IpProtocol::Ipv6Opts => Ipv4ProtoPacket::Ipv6Opts,
                };

                PacketType::Ipv4(Ipv4Packet {
                    header: ip,
                    protocol,
                })
            }
            EtherType::Ipv6 => PacketType::Ipv6,
        };

        Some(EthFrame { header, eth_type })
    }
}


#[cfg(test)]
mod eth_frame_tests {
    use super::*;

    #[test]
    fn dest_mac_empty() {
        let mut frame = [0u8; 128];
        let pkt = [0u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x08, 0];
        frame[0..14].copy_from_slice(&pkt);
        let res = EthFrame::parse(&frame);
        assert!(res.is_none());
    }

    #[test]
    fn malformed_eth_header() {
        let mut frame = [0u8; 128];
        let pkt = [
            0xffu8, 0xff, 0xff, 0xff, 0xff, 0xff, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x02, 0xC2,
        ];
        frame[0..14].copy_from_slice(&pkt);
        let res = EthFrame::parse(&frame);
        assert!(res.is_none());
    }

    #[test]
    fn eth_type_ipv6() {
        let mut frame = [0u8; 128];
        let pkt = [
            0xffu8, 0xff, 0xff, 0xff, 0xff, 0xff, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x86, 0xDD,
        ];
        frame[0..14].copy_from_slice(&pkt);
        let res = EthFrame::parse(&frame);
        assert_eq!(
            res,
            Some(EthFrame {
                header: EthernetRepr {
                    src_addr: Address([0x2, 0x3, 0x4, 0x5, 0x6, 0x7]),
                    dst_addr: Address([0xffu8, 0xff, 0xff, 0xff, 0xff, 0xff]),
                    ethertype: EtherType::Ipv6
                },
                eth_type: PacketType::Ipv6,
            })
        );
    }

    #[test]
    fn valid_arp_request() {
        let mut frame = [0u8; 128];
        let pkt = [
            0xffu8, 0xff, 0xff, 0xff, 0xff, 0xff, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x08, 0x06, 0x0,
            0x1, 0x8, 0x0, 0x6, 0x4, 0x0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0xc0, 0xa8, 0x0, 0x1,
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0xc0, 0xa8, 0x0, 0xce,
        ];
        frame[0..42].copy_from_slice(&pkt);
        let res = EthFrame::parse(&frame);
        assert_eq!(
            res,
            Some(EthFrame {
                header: EthernetRepr {
                    src_addr: Address([0x2, 0x3, 0x4, 0x5, 0x6, 0x7]),
                    dst_addr: Address([0xffu8, 0xff, 0xff, 0xff, 0xff, 0xff]),
                    ethertype: EtherType::Arp
                },
                eth_type: PacketType::Arp(Arp {
                    htype: HardwareType::Ethernet,
                    ptype: EtherType::Ipv4,
                    hsize: 0x6,
                    psize: 0x4,
                    op: ArpOp::Request,
                    src_addr: Address([0x2, 0x3, 0x4, 0x5, 0x6, 0x7]),
                    src_protocol_addr: Ipv4Address([0xc0, 0xa8, 0x00, 0x01]),
                    dest_addr: Address([0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
                    dest_protocol_addr: Ipv4Address([0xc0, 0xa8, 0x0, 0xce])
                })
            })
        );
    }

    #[test]
    fn valid_arp_reply() {
        let mut frame = [0u8; 128];
        let pkt = [
            0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x18, 0x20, 0x22, 0x24, 0x26, 0x28, 0x8, 0x6, 0x0, 0x1,
            0x8, 0x0, 0x6, 0x4, 0x0, 0x2, 0x18, 0x20, 0x22, 0x24, 0x26, 0x28, 0xc0, 0xa8, 0x0,
            0xce, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0xc0, 0xa8, 0x0, 0x1,
        ];
        frame[0..42].copy_from_slice(&pkt);
        let res = EthFrame::parse(&frame);
        assert_eq!(
            res,
            Some(EthFrame {
                header: EthernetRepr {
                    src_addr: Address([0x18, 0x20, 0x22, 0x24, 0x26, 0x28]),
                    dst_addr: Address([0x2, 0x3, 0x4, 0x5, 0x6, 0x7]),
                    ethertype: EtherType::Arp
                },
                eth_type: PacketType::Arp(Arp {
                    htype: HardwareType::Ethernet,
                    ptype: EtherType::Ipv4,
                    hsize: 0x6,
                    psize: 0x4,
                    op: ArpOp::Reply,
                    src_addr: Address([0x18, 0x20, 0x22, 0x24, 0x26, 0x28]),
                    src_protocol_addr: Ipv4Address([0xc0, 0xa8, 0x00, 0xce]),
                    dest_addr: Address([0x2, 0x3, 0x4, 0x5, 0x6, 0x7]),
                    dest_protocol_addr: Ipv4Address([0xc0, 0xa8, 0x0, 0x01])
                })
            })
        );
    }

    #[test]
    fn malformed_arp() {
        let mut frame = [0u8; 128];
        let pkt = [
            0xffu8, 0xff, 0xff, 0xff, 0xff, 0xff, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x08, 0x06, 0x0,
            0x2, 0x8, 0x0, 0x6, 0x4, 0x0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0xc0, 0xa8, 0x0, 0x1,
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0xc0, 0xa8, 0x0, 0xce,
        ];
        frame[0..42].copy_from_slice(&pkt);
        let res = EthFrame::parse(&frame);
        assert!(res.is_none());

        let pkt = [
            0xffu8, 0xff, 0xff, 0xff, 0xff, 0xff, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x08, 0x06, 0x0,
            0x1, 0x9, 0x20, 0x6, 0x4, 0x0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0xc0, 0xa8, 0x0, 0x1,
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0xc0, 0xa8, 0x0, 0xce,
        ];
        frame[0..42].copy_from_slice(&pkt);
        let res = EthFrame::parse(&frame);
        assert!(res.is_none());

        let pkt = [
            0xffu8, 0xff, 0xff, 0xff, 0xff, 0xff, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x08, 0x06, 0x0,
            0x1, 0x8, 0x0, 0x6, 0x4, 0x0, 0x6, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0xc0, 0xa8, 0x0, 0x1,
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0xc0, 0xa8, 0x0, 0xce,
        ];
        frame[0..42].copy_from_slice(&pkt);
        let res = EthFrame::parse(&frame);
        assert!(res.is_none());
    }

    #[test]
    fn malformed_ipv4() {
        let mut frame = [0u8; 128];
        let pkt = [
            0xffu8, 0xff, 0xff, 0xff, 0xff, 0xff, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x08, 0x00, 0x45,
            0x0, 0x0, 0x29, 0x15, 0x5f, 0x40, 0x0, 0x80, 0x7, 0xf7, 0x28, 0xc0, 0xa8, 0x0, 0xce,
            0x34, 0x7f, 0xf8, 0x51,
        ];
        frame[0..34].copy_from_slice(&pkt);
        let res = EthFrame::parse(&frame);
        assert!(res.is_none());
    }

    #[test]
    fn valid_ipv4_protocols() {
        let mut frame = [0u8; 128];
        // Hop by Hop
        let pkt = [
            0xffu8, 0xff, 0xff, 0xff, 0xff, 0xff, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x08, 0x00, 0x45,
            0x0, 0x0, 0x29, 0x15, 0x5f, 0x40, 0x0, 0x80, 0x0, 0xf7, 0x28, 0xc0, 0xa8, 0x0, 0xce,
            0x34, 0x7f, 0xf8, 0x51, 0xc4, 0x73, 0x1, 0xbb, 0x21, 0x65, 0x90, 0xfb, 0xe4, 0x98,
            0x7c, 0x9d, 0x50, 0x10, 0x3, 0xff, 0xe3, 0xc7, 0x0, 0x0,
        ];
        frame[0..54].copy_from_slice(&pkt);

        let mut expected = EthFrame {
            header: EthernetRepr {
                src_addr: Address([0x2, 0x3, 0x4, 0x5, 0x6, 0x7]),
                dst_addr: Address([0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
                ethertype: EtherType::Ipv4,
            },
            eth_type: PacketType::Ipv4(Ipv4Packet {
                header: Ipv4Repr {
                    protocol: IpProtocol::HopByHop,
                    length: 0x29,
                },
                protocol: Ipv4ProtoPacket::HopByHop,
            }),
        };

        let res = EthFrame::parse(&frame);
        assert_eq!(res.as_ref(), Some(&expected));

        // ICMP
        frame[23] = 0x01;
        if let PacketType::Ipv4(pack) = &mut expected.eth_type {
            pack.header.protocol = IpProtocol::Icmp;
            pack.protocol = Ipv4ProtoPacket::Icmp;
        }
        let res = EthFrame::parse(&frame);
        assert_eq!(res.as_ref(), Some(&expected));

        // IGMP
        frame[23] = 0x02;
        if let PacketType::Ipv4(pack) = &mut expected.eth_type {
            pack.header.protocol = IpProtocol::Igmp;
            pack.protocol = Ipv4ProtoPacket::Igmp;
        }
        let res = EthFrame::parse(&frame);
        assert_eq!(res.as_ref(), Some(&expected));

        // Ipv6 Route
        frame[23] = 0x2b;
        if let PacketType::Ipv4(pack) = &mut expected.eth_type {
            pack.header.protocol = IpProtocol::Ipv6Route;
            pack.protocol = Ipv4ProtoPacket::Ipv6Route;
        }
        let res = EthFrame::parse(&frame);
        assert_eq!(res.as_ref(), Some(&expected));

        // Ipv6 Frag
        frame[23] = 0x2c;
        if let PacketType::Ipv4(pack) = &mut expected.eth_type {
            pack.header.protocol = IpProtocol::Ipv6Frag;
            pack.protocol = Ipv4ProtoPacket::Ipv6Frag;
        }
        let res = EthFrame::parse(&frame);
        assert_eq!(res.as_ref(), Some(&expected));

        // ICMPv6
        frame[23] = 0x3a;
        if let PacketType::Ipv4(pack) = &mut expected.eth_type {
            pack.header.protocol = IpProtocol::Icmpv6;
            pack.protocol = Ipv4ProtoPacket::Icmpv6;
        }
        let res = EthFrame::parse(&frame);
        assert_eq!(res.as_ref(), Some(&expected));

        // IPv6 No Nxt
        frame[23] = 0x3b;
        if let PacketType::Ipv4(pack) = &mut expected.eth_type {
            pack.header.protocol = IpProtocol::Ipv6NoNxt;
            pack.protocol = Ipv4ProtoPacket::Ipv6NoNxt;
        }
        let res = EthFrame::parse(&frame);
        assert_eq!(res.as_ref(), Some(&expected));

        // IPv6 Opts
        frame[23] = 0x3c;
        if let PacketType::Ipv4(pack) = &mut expected.eth_type {
            pack.header.protocol = IpProtocol::Ipv6Opts;
            pack.protocol = Ipv4ProtoPacket::Ipv6Opts;
        }
        let res = EthFrame::parse(&frame);
        assert_eq!(res.as_ref(), Some(&expected));

        // TCP
        frame[23] = 0x06;
        if let PacketType::Ipv4(pack) = &mut expected.eth_type {
            pack.header.protocol = IpProtocol::Tcp;
            pack.protocol = Ipv4ProtoPacket::Tcp(TcpRepr { dst_port: 443 });
        }
        let res = EthFrame::parse(&frame);
        assert_eq!(res.as_ref(), Some(&expected));

        // UDP
        frame[23] = 0x11;
        if let PacketType::Ipv4(pack) = &mut expected.eth_type {
            pack.header.protocol = IpProtocol::Udp;
            pack.protocol = Ipv4ProtoPacket::Udp(UdpRepr { src_port: 0xc473, dst_port: 443 });
        }
        let res = EthFrame::parse(&frame);
        assert_eq!(res.as_ref(), Some(&expected));
    }
}
