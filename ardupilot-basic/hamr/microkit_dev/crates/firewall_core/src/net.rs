use vstd::prelude::*;
use vstd::slice::slice_subrange;
#[cfg(verus_keep_ghost)]
use vstd::std_specs::convert::TryFromSpecImpl;


// ============================================================
// Ipv4Address
// ============================================================

#[verus_verify]
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
pub struct Ipv4Address(pub [u8; 4]);

#[verus_verify]
impl Ipv4Address {
    #[verus_spec(r =>
        requires
            data.len() >= 4,
        ensures
            r.0@ =~= data@.subrange(0, 4 as int),
    )]
    pub fn from_bytes(data: &[u8]) -> Ipv4Address {
        let mut bytes = [0u8; 4];
        let mut i: usize = 0;
        #[verus_spec(
            invariant
                0 <= i <= 4,
                data.len() >= 4,
                bytes@.len() == 4,
                forall|j: int| 0 <= j < i as int ==> bytes@[j] == data@[j],
            decreases
                4 - i,
        )]
        while i < 4 {
            bytes.set(i, data[i]);
            i += 1;
        }
        Ipv4Address(bytes)
    }
}

// ============================================================
// Address
// ============================================================


#[verus_verify]
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
pub struct Address(pub [u8; 6]);

#[verus_verify]
impl Address {
    #[verus_spec(r =>
        requires
            data.len() >= 6,
        ensures
            r.0@ =~= data@.subrange(0, 6 as int),
    )]
    pub fn from_bytes(data: &[u8]) -> Address {
        let mut bytes = [0u8; 6];
        let mut i: usize = 0;
        #[verus_spec(
            invariant
                0 <= i <= 6,
                data.len() >= 6,
                bytes@.len() == 6,
                forall|j: int| 0 <= j < i as int ==> bytes@[j] == data@[j],
            decreases
                6 - i,
        )]
        while i < 6 {
            bytes.set(i, data[i]);
            i += 1;
        }
        Address(bytes)
    }

    #[verus_spec(r =>
        ensures
            r == (forall|j: int| 0 <= j < 6 ==> self.0@[j] == 0u8),
    )]
    pub fn is_empty(&self) -> bool {
        let mut i: usize = 0;
        #[verus_spec(
            invariant
                0 <= i <= 6,
                self.0@.len() == 6,
                forall|j: int| 0 <= j < i as int ==> self.0@[j] == 0u8,
            decreases
                6 - i,
        )]
        while i < 6 {
            if self.0[i] != 0 {
                return false;
            }
            i += 1;
        }
        true
    }
}

// ============================================================
// u16_from_be_bytes
// ============================================================


#[verus_spec(r =>
    requires
        bytes.len() >= 2,
    ensures
        r == (bytes@[0 as int] as u16) * 256 + (bytes@[1 as int] as u16),
)]
fn u16_from_be_bytes(bytes: &[u8]) -> u16 {
    ((bytes[0] as u16) * 256u16) + (bytes[1] as u16)
}

// ============================================================
// EtherType
// ============================================================


#[verus_verify]
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone, Copy)]
#[repr(u16)]
pub enum EtherType {
    Ipv4 = 0x0800,
    Arp = 0x0806,
    Ipv6 = 0x86DD,
}

verus! {
    pub open spec fn spec_ether_type_from_u16(raw: u16) -> Option<EtherType> {
        match raw {
            0x0800u16 => Some(EtherType::Ipv4),
            0x0806u16 => Some(EtherType::Arp),
            0x86DDu16 => Some(EtherType::Ipv6),
            _ => None,
        }
    }
}

#[verus_verify]
impl EtherType {
    #[verus_spec(r =>
        requires
            bytes.len() >= 2,
        ensures
            r == spec_ether_type_from_u16(
                ((bytes@[0 as int] as u16) * 256 + (bytes@[1 as int] as u16)) as u16
            ),
    )]
    pub fn from_bytes(bytes: &[u8]) -> Option<EtherType> {
        let raw = u16_from_be_bytes(bytes);
        EtherType::try_from(raw).ok()
    }
}


#[verus_verify]
impl TryFrom<u16> for EtherType {
    type Error = ();

    fn try_from(value: u16) -> Result<Self, Self::Error>
    {
        match value {
            0x0800 => Ok(EtherType::Ipv4),
            0x0806 => Ok(EtherType::Arp),
            0x86DD => Ok(EtherType::Ipv6),
            _ => Err(()),
        }
    }
}

#[cfg(verus_keep_ghost)]
verus! {
    impl TryFromSpecImpl<u16> for EtherType {
        open spec fn obeys_try_from_spec() -> bool { true }
        open spec fn try_from_spec(value: u16) -> Result<Self, ()> {
            match value {
                0x0800u16 => Ok(EtherType::Ipv4),
                0x0806u16 => Ok(EtherType::Arp),
                0x86DDu16 => Ok(EtherType::Ipv6),
                _ => Err(()),
            }
        }
    }
}


impl From<EtherType> for u16 {
    fn from(value: EtherType) -> Self {
        match value {
            EtherType::Ipv4 => 0x0800,
            EtherType::Arp => 0x0806,
            EtherType::Ipv6 => 0x86DD,
        }
    }
}

// ============================================================
// EthernetRepr
// ============================================================


#[verus_verify]
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
pub struct EthernetRepr {
    pub src_addr: Address,
    pub dst_addr: Address,
    pub ethertype: EtherType,
}


#[verus_verify]
impl EthernetRepr {
    pub const SIZE: usize = 14;

    /// Parse an Ethernet II frame and return a high-level representation.
    #[verus_spec(r =>
        requires
            frame.len() >= 14,
        ensures
            r.is_some() ==> (
                r.unwrap().dst_addr.0@ =~= frame@.subrange(0, 6 as int)
                && r.unwrap().src_addr.0@ =~= frame@.subrange(6, 12 as int)
            ),
    )]
    pub fn parse(frame: &[u8]) -> Option<EthernetRepr> {
        let dst_addr = Address::from_bytes(slice_subrange(frame, 0, 6));
        if dst_addr.is_empty() {
            return None;
        }
        let src_addr = Address::from_bytes(slice_subrange(frame, 6, 12));
        let ethertype = EtherType::from_bytes(slice_subrange(frame,12,14))?;

        Some(EthernetRepr {
            src_addr,
            dst_addr,
            ethertype,
        })
    }
}

// ============================================================
// ArpOp
// ============================================================


#[verus_verify]
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone, Copy)]
#[repr(u16)]
pub enum ArpOp {
    Request = 1,
    Reply = 2,
}

verus! {
    pub open spec fn spec_arp_op_from_u16(raw: u16) -> Option<ArpOp> {
        match raw {
            1u16 => Some(ArpOp::Request),
            2u16 => Some(ArpOp::Reply),
            _ => None,
        }
    }
}

#[verus_verify]
impl ArpOp {
    #[verus_spec(r =>
        requires
            bytes.len() >= 2,
        ensures
            r == spec_arp_op_from_u16(
                ((bytes@[0 as int] as u16) * 256 + (bytes@[1 as int] as u16)) as u16
            ),
    )]
    pub fn from_bytes(bytes: &[u8]) -> Option<ArpOp> {
        let raw = u16_from_be_bytes(bytes);
        ArpOp::try_from(raw).ok()
    }
}


#[verus_verify]
impl TryFrom<u16> for ArpOp {
    type Error = ();

    fn try_from(value: u16) -> Result<Self, Self::Error>
    {
        match value {
            1 => Ok(ArpOp::Request),
            2 => Ok(ArpOp::Reply),
            _ => Err(()),
        }
    }
}

#[cfg(verus_keep_ghost)]
verus! {
    impl TryFromSpecImpl<u16> for ArpOp {
        open spec fn obeys_try_from_spec() -> bool { true }
        open spec fn try_from_spec(value: u16) -> Result<Self, ()> {
            match value {
                1u16 => Ok(ArpOp::Request),
                2u16 => Ok(ArpOp::Reply),
                _ => Err(()),
            }
        }
    }
}


impl From<ArpOp> for u16 {
    fn from(value: ArpOp) -> Self {
        match value {
            ArpOp::Request => 1,
            ArpOp::Reply => 2,
        }
    }
}

// ============================================================
// HardwareType
// ============================================================


#[verus_verify]
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone, Copy)]
#[repr(u16)]
pub enum HardwareType {
    Ethernet = 1,
}

verus! {
    pub open spec fn spec_hardware_type_from_u16(raw: u16) -> Option<HardwareType> {
        match raw {
            1u16 => Some(HardwareType::Ethernet),
            _ => None,
        }
    }
}

#[verus_verify]
impl HardwareType {
    #[verus_spec(r =>
        requires
            bytes.len() >= 2,
        ensures
            r == spec_hardware_type_from_u16(
                ((bytes@[0 as int] as u16) * 256 + (bytes@[1 as int] as u16)) as u16
            ),
    )]
    pub fn from_bytes(bytes: &[u8]) -> Option<HardwareType> {
        let raw = u16_from_be_bytes(bytes);
        HardwareType::try_from(raw).ok()
    }
}


#[verus_verify]
impl TryFrom<u16> for HardwareType {
    type Error = ();

    fn try_from(value: u16) -> Result<Self, Self::Error>
    {
        match value {
            1 => Ok(HardwareType::Ethernet),
            _ => Err(()),
        }
    }
}

#[cfg(verus_keep_ghost)]
verus! {
    impl TryFromSpecImpl<u16> for HardwareType {
        open spec fn obeys_try_from_spec() -> bool { true }
        open spec fn try_from_spec(value: u16) -> Result<Self, ()> {
            match value {
                1u16 => Ok(HardwareType::Ethernet),
                _ => Err(()),
            }
        }
    }
}


impl From<HardwareType> for u16 {
    fn from(value: HardwareType) -> Self {
        match value {
            HardwareType::Ethernet => 1,
        }
    }
}

// ============================================================
// Arp
// ============================================================

// TODO: Protocol addresses should be variable, but I only care about supporting ipv4 for now

#[verus_verify]
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
pub struct Arp {
    pub htype: HardwareType,
    pub ptype: EtherType,
    pub hsize: u8,
    pub psize: u8,
    pub op: ArpOp,
    pub src_addr: Address,
    pub src_protocol_addr: Ipv4Address,
    pub dest_addr: Address,
    pub dest_protocol_addr: Ipv4Address,
}


#[verus_verify]
impl Arp {
    pub const SIZE: usize = 28;

    /// Parse an ARP packet and return a high-level representation.
    #[verus_spec(r =>
        requires
            packet.len() >= 28,
        ensures
            r.is_some() ==> (
                r.unwrap().hsize == packet@[4 as int]
                && r.unwrap().psize == packet@[5 as int]
                && r.unwrap().src_addr.0@ =~= packet@.subrange(8, 14 as int)
                && r.unwrap().src_protocol_addr.0@ =~= packet@.subrange(14, 18 as int)
                && r.unwrap().dest_addr.0@ =~= packet@.subrange(18, 24 as int)
                && r.unwrap().dest_protocol_addr.0@ =~= packet@.subrange(24, 28 as int)
            ),
    )]
    pub fn parse(packet: &[u8]) -> Option<Arp> {
        let htype = HardwareType::from_bytes(slice_subrange(packet, 0, 2))?;
        let ptype = EtherType::from_bytes(slice_subrange(packet, 2, 4))?;
        if !Self::allowed_ptype(&ptype) {
            return None;
        }
        let hsize = packet[4];
        let psize = packet[5];
        let op = ArpOp::from_bytes(slice_subrange(packet, 6, 8))?;
        let src_addr = Address::from_bytes(slice_subrange(packet, 8, 14));
        let src_protocol_addr = Ipv4Address::from_bytes(slice_subrange(packet, 14, 18));
        let dest_addr = Address::from_bytes(slice_subrange(packet, 18, 24));
        let dest_protocol_addr = Ipv4Address::from_bytes(slice_subrange(packet, 24, 28));
        Some(Arp {
            htype,
            ptype,
            hsize,
            psize,
            op,
            src_addr,
            src_protocol_addr,
            dest_addr,
            dest_protocol_addr,
        })
    }

    #[verus_spec(r =>
        ensures
            r == match *ptype {
                EtherType::Arp => false,
                _ => true,
            },
    )]
    fn allowed_ptype(ptype: &EtherType) -> bool {
        if let EtherType::Arp = ptype {
            false
        } else {
            true
        }
    }
}

// ============================================================
// IpProtocol
// ============================================================


#[verus_verify]
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum IpProtocol {
    HopByHop = 0x00,
    Icmp = 0x01,
    Igmp = 0x02,
    Tcp = 0x06,
    Udp = 0x11,
    Ipv6Route = 0x2b,
    Ipv6Frag = 0x2c,
    Icmpv6 = 0x3a,
    Ipv6NoNxt = 0x3b,
    Ipv6Opts = 0x3c,
}


#[verus_verify]
impl TryFrom<u8> for IpProtocol {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error>
    {
        match value {
            0x00 => Ok(IpProtocol::HopByHop),
            0x01 => Ok(IpProtocol::Icmp),
            0x02 => Ok(IpProtocol::Igmp),
            0x06 => Ok(IpProtocol::Tcp),
            0x11 => Ok(IpProtocol::Udp),
            0x2b => Ok(IpProtocol::Ipv6Route),
            0x2c => Ok(IpProtocol::Ipv6Frag),
            0x3a => Ok(IpProtocol::Icmpv6),
            0x3b => Ok(IpProtocol::Ipv6NoNxt),
            0x3c => Ok(IpProtocol::Ipv6Opts),
            _ => Err(()),
        }
    }
}

#[cfg(verus_keep_ghost)]
verus! {
    impl TryFromSpecImpl<u8> for IpProtocol {
        open spec fn obeys_try_from_spec() -> bool { true }
        open spec fn try_from_spec(value: u8) -> Result<Self, ()> {
            match value {
                0x00u8 => Ok(IpProtocol::HopByHop),
                0x01u8 => Ok(IpProtocol::Icmp),
                0x02u8 => Ok(IpProtocol::Igmp),
                0x06u8 => Ok(IpProtocol::Tcp),
                0x11u8 => Ok(IpProtocol::Udp),
                0x2bu8 => Ok(IpProtocol::Ipv6Route),
                0x2cu8 => Ok(IpProtocol::Ipv6Frag),
                0x3au8 => Ok(IpProtocol::Icmpv6),
                0x3bu8 => Ok(IpProtocol::Ipv6NoNxt),
                0x3cu8 => Ok(IpProtocol::Ipv6Opts),
                _ => Err(()),
            }
        }
    }
}


impl From<IpProtocol> for u8 {
    fn from(value: IpProtocol) -> Self {
        match value {
            IpProtocol::HopByHop => 0x00,
            IpProtocol::Icmp => 0x01,
            IpProtocol::Igmp => 0x02,
            IpProtocol::Tcp => 0x06,
            IpProtocol::Udp => 0x11,
            IpProtocol::Ipv6Route => 0x2b,
            IpProtocol::Ipv6Frag => 0x2c,
            IpProtocol::Icmpv6 => 0x3a,
            IpProtocol::Ipv6NoNxt => 0x3b,
            IpProtocol::Ipv6Opts => 0x3c,
        }
    }
}

// ============================================================
// Ipv4Repr
// ============================================================


#[verus_verify]
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
pub struct Ipv4Repr {
    pub protocol: IpProtocol,
    pub length: u16,
}


#[verus_verify]
impl Ipv4Repr {
    pub const SIZE: usize = 20;

    #[verus_spec(r =>
        requires
            packet.len() >= 20,
        ensures
            r.is_some() ==> (
                r.unwrap().length == (packet@[2 as int] as u16) * 256 + (packet@[3 as int] as u16)
                && r.unwrap().length <= MAX_MTU
            ),
    )]
    pub fn parse(packet: &[u8]) -> Option<Ipv4Repr> {
        let protocol = IpProtocol::try_from(packet[9]).ok()?;
        let length =  u16_from_be_bytes(slice_subrange(packet, 2, 4));
        if packet[0] != 0x45 {
            return None;
        }
        if length > MAX_MTU {
            return None;
        }
        Some(Ipv4Repr { protocol, length })
    }
}

// ============================================================
// TcpRepr
// ============================================================


#[verus_verify]
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
pub struct TcpRepr {
    pub dst_port: u16,
}


#[verus_verify]
impl TcpRepr {
    pub const SIZE: usize = 20;

    #[verus_spec(r =>
        requires
            packet.len() >= 4,
        ensures
            r.dst_port == (packet@[2 as int] as u16) * 256 + (packet@[3 as int] as u16),
    )]
    pub fn parse(packet: &[u8]) -> TcpRepr {
        let dst_port = u16_from_be_bytes(slice_subrange(packet, 2, 4));
        TcpRepr { dst_port }
    }
}

// ============================================================
// UdpRepr
// ============================================================


#[verus_verify]
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
pub struct UdpRepr {
    pub src_port: u16,
    pub dst_port: u16,
}


#[verus_verify]
impl UdpRepr {
    pub const SIZE: usize = 20;

    #[verus_spec(r =>
        requires
            packet.len() >= 4,
        ensures
            r.src_port == (packet@[0 as int] as u16) * 256 + (packet@[1 as int] as u16),
            r.dst_port == (packet@[2 as int] as u16) * 256 + (packet@[3 as int] as u16),
    )]
    pub fn parse(packet: &[u8]) -> UdpRepr {
        let src_port =  u16_from_be_bytes(slice_subrange(packet, 0, 2));
        let dst_port =  u16_from_be_bytes(slice_subrange(packet, 2, 4));
        UdpRepr { src_port, dst_port }
    }
}

// ============================================================
// Constants
// ============================================================

verus! {
    /// Define the max possible MTU. Use standard Jumbo size as maximum possible.
    pub const MAX_MTU: u16 = 9000;
}

// ============================================================
// Tests
// ============================================================

#[test]
fn from_arpop_to_u16_test() {
    let res: u16 = ArpOp::Request.into();
    assert_eq!(res, 1);
    let res: u16 = ArpOp::Reply.into();
    assert_eq!(res, 2);
}

#[test]
fn from_hardwaretype_to_u16_test() {
    let res: u16 = HardwareType::Ethernet.into();
    assert_eq!(res, 1);
}

#[test]
fn from_ipprotocol_to_u8_test() {
    let res: u8 = IpProtocol::HopByHop.into();
    assert_eq!(res, 0);
    let res: u8 = IpProtocol::Icmp.into();
    assert_eq!(res, 0x01);
    let res: u8 = IpProtocol::Igmp.into();
    assert_eq!(res, 0x02);
    let res: u8 = IpProtocol::Tcp.into();
    assert_eq!(res, 0x06);
    let res: u8 = IpProtocol::Udp.into();
    assert_eq!(res, 0x11);
    let res: u8 = IpProtocol::Ipv6Route.into();
    assert_eq!(res, 0x2b);
    let res: u8 = IpProtocol::Ipv6Frag.into();
    assert_eq!(res, 0x2c);
    let res: u8 = IpProtocol::Icmpv6.into();
    assert_eq!(res, 0x3a);
    let res: u8 = IpProtocol::Ipv6NoNxt.into();
    assert_eq!(res, 0x3b);
    let res: u8 = IpProtocol::Ipv6Opts.into();
    assert_eq!(res, 0x3c);
}

#[test]
fn from_ethertype_to_u16_test() {
    let res: u16 = EtherType::Ipv4.into();
    assert_eq!(res, 0x0800);
    let res: u16 = EtherType::Arp.into();
    assert_eq!(res, 0x0806);
    let res: u16 = EtherType::Ipv6.into();
    assert_eq!(res, 0x86DD);
}

#[test]
fn mac_address_from_bytes_test() {
    let bytes = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
    let res = Address::from_bytes(&bytes[1..7]);
    assert_eq!(res, Address([0x02, 0x03, 0x04, 0x05, 0x06, 0x07]));
}

#[test]
fn ethertype_from_bytes_test() {
    let bytes = [0x08u8, 0x00];
    let res = EtherType::from_bytes(&bytes).unwrap();
    assert_eq!(res, EtherType::Ipv4);

    let bytes = [0x08u8, 0x06];
    let res = EtherType::from_bytes(&bytes).unwrap();
    assert_eq!(res, EtherType::Arp);

    let bytes = [0x86u8, 0xDD];
    let res = EtherType::from_bytes(&bytes).unwrap();
    assert_eq!(res, EtherType::Ipv6);

    let bytes = [0x10u8, 0x10];
    let res = EtherType::from_bytes(&bytes);
    assert!(res.is_none());
}

#[cfg(test)]
mod ethernet_repr_tests {
    use super::*;

    #[test]
    fn parse() {
        let bytes = [
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x08, 0x00,
        ];
        let eth = EthernetRepr::parse(&bytes);
        assert_eq!(
            eth,
            Some(EthernetRepr {
                src_addr: Address([0x2, 0x3, 0x4, 0x5, 0x6, 0x7]),
                dst_addr: Address([0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
                ethertype: EtherType::Ipv4
            })
        );

        let bytes = [
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x20, 0x20,
        ];
        let eth = EthernetRepr::parse(&bytes);
        assert!(eth.is_none());
    }
}

#[cfg(test)]
mod arp_tests {
    use super::*;

    #[test]
    fn parse() {
        let mut pkt = [
            0x0, 0x1, 0x8, 0x0, 0x6, 0x4, 0x0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0xc0, 0xa8, 0x0,
            0x1, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0xc0, 0xa8, 0x0, 0xce,
        ];
        let expect = Arp {
            htype: HardwareType::Ethernet,
            ptype: EtherType::Ipv4,
            hsize: 0x6,
            psize: 0x4,
            op: ArpOp::Request,
            src_addr: Address([0x2, 0x3, 0x4, 0x5, 0x6, 0x7]),
            src_protocol_addr: Ipv4Address([0xc0, 0xa8, 0x00, 0x01]),
            dest_addr: Address([0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
            dest_protocol_addr: Ipv4Address([0xc0, 0xa8, 0x0, 0xce]),
        };
        let res = Arp::parse(&pkt);
        assert_eq!(res, Some(expect));

        pkt[0] = 5;
        let res = Arp::parse(&pkt);
        assert!(res.is_none());
    }

    #[test]
    fn valid_ptype() {
        assert!(Arp::allowed_ptype(&EtherType::Ipv4));
        assert!(Arp::allowed_ptype(&EtherType::Ipv6));
        assert!(!Arp::allowed_ptype(&EtherType::Arp));
    }
}
