// This file will not be overwritten if HAMR codegen is rerun

use data::*;
use crate::bridge::seL4_RxFirewall_RxFirewall_api::*;
use vstd::prelude::*;

#[verus_verify]
pub struct seL4_RxFirewall_RxFirewall {
  // PLACEHOLDER MARKER STATE VARS
}

#[verus_verify]
impl seL4_RxFirewall_RxFirewall {
  pub fn new() -> Self
  {
    Self {
      // PLACEHOLDER MARKER STATE VAR INIT
    }
  }

  #[verus_spec(
    ensures
      // PLACEHOLDER MARKER INITIALIZATION ENSURES
  )]
  pub fn initialize<API: seL4_RxFirewall_RxFirewall_Put_Api> (
    &mut self,
    api: &mut seL4_RxFirewall_RxFirewall_Application_Api<API>)
  {
    log_info("initialize entrypoint invoked");
  }

  #[verus_spec(
    requires
      // BEGIN MARKER TIME TRIGGERED REQUIRES
      // assume AADL_Requirement
      //   All outgoing event ports must be empty
      old(api).VmmOut0.is_none(),
      old(api).VmmOut1.is_none(),
      old(api).VmmOut2.is_none(),
      old(api).VmmOut3.is_none(),
      old(api).MavlinkOut0.is_none(),
      old(api).MavlinkOut1.is_none(),
      old(api).MavlinkOut2.is_none(),
      old(api).MavlinkOut3.is_none(),
      // END MARKER TIME TRIGGERED REQUIRES
    ensures
      // BEGIN MARKER TIME TRIGGERED ENSURES
      // guarantee hlr_05_rx0_can_send_arp_to_vmm
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=3
      api.EthernetFramesRxIn0.is_some() && GumboLib::valid_arp_spec(api.EthernetFramesRxIn0.unwrap()) ==>
        api.VmmOut0.is_some() &&
          ((api.EthernetFramesRxIn0.unwrap() == api.VmmOut0.unwrap()) &&
            api.MavlinkOut0.is_none()),
      // guarantee hlr_18_rx0_can_send_mavlink_udp
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=4
      api.EthernetFramesRxIn0.is_some() && GumboLib::valid_ipv4_udp_mavlink_spec(api.EthernetFramesRxIn0.unwrap()) ==>
        api.MavlinkOut0.is_some() &&
          (GumboLib::input_eq_mav_output_spec(api.EthernetFramesRxIn0.unwrap(), api.MavlinkOut0.unwrap()) && api.VmmOut0.is_none()),
      // guarantee hlr_13_rx0_can_send_ipv4_udp
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=3
      api.EthernetFramesRxIn0.is_some() && GumboLib::valid_ipv4_udp_port_spec(api.EthernetFramesRxIn0.unwrap()) ==>
        api.VmmOut0.is_some() &&
          ((api.EthernetFramesRxIn0.unwrap() == api.VmmOut0.unwrap()) &&
            api.MavlinkOut0.is_none()),
      // guarantee hlr_15_rx0_disallow
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=4
      api.EthernetFramesRxIn0.is_some() && !(GumboLib::rx_allow_outbound_frame_spec(api.EthernetFramesRxIn0.unwrap())) ==>
        (api.VmmOut0.is_none() && api.MavlinkOut0.is_none()),
      // guarantee hlr_17_rx0_no_input
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=4
      api.EthernetFramesRxIn0.is_some() ||
        (api.VmmOut0.is_none() && api.MavlinkOut0.is_none()),
      // guarantee hlr_05_rx1_can_send_arp_to_vmm
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=3
      api.EthernetFramesRxIn1.is_some() && GumboLib::valid_arp_spec(api.EthernetFramesRxIn1.unwrap()) ==>
        api.VmmOut1.is_some() &&
          ((api.EthernetFramesRxIn1.unwrap() == api.VmmOut1.unwrap()) &&
            api.MavlinkOut1.is_none()),
      // guarantee hlr_18_rx1_can_send_mavlink_udp
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=4
      api.EthernetFramesRxIn1.is_some() && GumboLib::valid_ipv4_udp_mavlink_spec(api.EthernetFramesRxIn1.unwrap()) ==>
        api.MavlinkOut1.is_some() &&
          (GumboLib::input_eq_mav_output_spec(api.EthernetFramesRxIn1.unwrap(), api.MavlinkOut1.unwrap()) && api.VmmOut1.is_none()),
      // guarantee hlr_13_rx1_can_send_ipv4_udp
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=3
      api.EthernetFramesRxIn1.is_some() && GumboLib::valid_ipv4_udp_port_spec(api.EthernetFramesRxIn1.unwrap()) ==>
        api.VmmOut1.is_some() &&
          ((api.EthernetFramesRxIn1.unwrap() == api.VmmOut1.unwrap()) &&
            api.MavlinkOut1.is_none()),
      // guarantee hlr_15_rx1_disallow
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=4
      api.EthernetFramesRxIn1.is_some() && !(GumboLib::rx_allow_outbound_frame_spec(api.EthernetFramesRxIn1.unwrap())) ==>
        (api.VmmOut1.is_none() && api.MavlinkOut1.is_none()),
      // guarantee hlr_17_rx1_no_input
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=4
      api.EthernetFramesRxIn1.is_some() ||
        (api.VmmOut1.is_none() && api.MavlinkOut1.is_none()),
      // guarantee hlr_05_rx2_can_send_arp_to_vmm
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=3
      api.EthernetFramesRxIn2.is_some() && GumboLib::valid_arp_spec(api.EthernetFramesRxIn2.unwrap()) ==>
        api.VmmOut2.is_some() &&
          ((api.EthernetFramesRxIn2.unwrap() == api.VmmOut2.unwrap()) &&
            api.MavlinkOut2.is_none()),
      // guarantee hlr_18_rx2_can_send_mavlink_udp
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=4
      api.EthernetFramesRxIn2.is_some() && GumboLib::valid_ipv4_udp_mavlink_spec(api.EthernetFramesRxIn2.unwrap()) ==>
        api.MavlinkOut2.is_some() &&
          (GumboLib::input_eq_mav_output_spec(api.EthernetFramesRxIn2.unwrap(), api.MavlinkOut2.unwrap()) && api.VmmOut2.is_none()),
      // guarantee hlr_13_rx2_can_send_ipv4_udp
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=3
      api.EthernetFramesRxIn2.is_some() && GumboLib::valid_ipv4_udp_port_spec(api.EthernetFramesRxIn2.unwrap()) ==>
        api.VmmOut2.is_some() &&
          ((api.EthernetFramesRxIn2.unwrap() == api.VmmOut2.unwrap()) &&
            api.MavlinkOut2.is_none()),
      // guarantee hlr_15_rx2_disallow
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=4
      api.EthernetFramesRxIn2.is_some() && !(GumboLib::rx_allow_outbound_frame_spec(api.EthernetFramesRxIn2.unwrap())) ==>
        (api.VmmOut2.is_none() && api.MavlinkOut2.is_none()),
      // guarantee hlr_17_rx2_no_input
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=4
      api.EthernetFramesRxIn2.is_some() ||
        (api.VmmOut2.is_none() && api.MavlinkOut2.is_none()),
      // guarantee hlr_05_rx3_can_send_arp_to_vmm
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=3
      api.EthernetFramesRxIn3.is_some() && GumboLib::valid_arp_spec(api.EthernetFramesRxIn3.unwrap()) ==>
        api.VmmOut3.is_some() &&
          ((api.EthernetFramesRxIn3.unwrap() == api.VmmOut3.unwrap()) &&
            api.MavlinkOut3.is_none()),
      // guarantee hlr_18_rx3_can_send_mavlink_udp
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=4
      api.EthernetFramesRxIn3.is_some() && GumboLib::valid_ipv4_udp_mavlink_spec(api.EthernetFramesRxIn3.unwrap()) ==>
        api.MavlinkOut3.is_some() &&
          (GumboLib::input_eq_mav_output_spec(api.EthernetFramesRxIn3.unwrap(), api.MavlinkOut3.unwrap()) && api.VmmOut3.is_none()),
      // guarantee hlr_13_rx3_can_send_ipv4_udp
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=3
      api.EthernetFramesRxIn3.is_some() && GumboLib::valid_ipv4_udp_port_spec(api.EthernetFramesRxIn3.unwrap()) ==>
        api.VmmOut3.is_some() &&
          ((api.EthernetFramesRxIn3.unwrap() == api.VmmOut3.unwrap()) &&
            api.MavlinkOut3.is_none()),
      // guarantee hlr_15_rx3_disallow
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=4
      api.EthernetFramesRxIn3.is_some() && !(GumboLib::rx_allow_outbound_frame_spec(api.EthernetFramesRxIn3.unwrap())) ==>
        (api.VmmOut3.is_none() && api.MavlinkOut3.is_none()),
      // guarantee hlr_17_rx3_no_input
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=4
      api.EthernetFramesRxIn3.is_some() ||
        (api.VmmOut3.is_none() && api.MavlinkOut3.is_none()),
      // END MARKER TIME TRIGGERED ENSURES
  )]
  pub fn timeTriggered<API: seL4_RxFirewall_RxFirewall_Full_Api> (
    &mut self,
    api: &mut seL4_RxFirewall_RxFirewall_Application_Api<API>)
  {
    //log_info("compute entrypoint invoked");
    
    // Rx0 ports
    if let Some(frame) = api.get_EthernetFramesRxIn0() {
      if let Some(eth) = get_frame_packet(&frame) {
        if can_send_to_mavlink(&eth.eth_type) {
          let output = udp_frame_from_raw_eth(frame);
          api.put_MavlinkOut0(output);
        } else if can_send_to_vmm(&eth.eth_type) {
          api.put_VmmOut0(frame);
        }
      }
    }

    // Rx1 ports
    if let Some(frame) = api.get_EthernetFramesRxIn1() {
      if let Some(eth) = get_frame_packet(&frame) {
        if can_send_to_mavlink(&eth.eth_type) {
          let output = udp_frame_from_raw_eth(frame);
          api.put_MavlinkOut1(output);
        } else if can_send_to_vmm(&eth.eth_type) {
          api.put_VmmOut1(frame);
        }
      }
    }

    // Rx2 ports
    if let Some(frame) = api.get_EthernetFramesRxIn2() {
      if let Some(eth) = get_frame_packet(&frame) {
        if can_send_to_mavlink(&eth.eth_type) {
          let output = udp_frame_from_raw_eth(frame);
          api.put_MavlinkOut2(output);
        } else if can_send_to_vmm(&eth.eth_type) {
          api.put_VmmOut2(frame);
        }
      }
    }

    // Rx3 ports
    if let Some(frame) = api.get_EthernetFramesRxIn3() {
      if let Some(eth) = get_frame_packet(&frame) {
        if can_send_to_mavlink(&eth.eth_type) {
          let output = udp_frame_from_raw_eth(frame);
          api.put_MavlinkOut3(output);
        } else if can_send_to_vmm(&eth.eth_type) {
          api.put_VmmOut3(frame);
        }
      }
    }
  }

  pub fn notify(
    &mut self,
    channel: microkit_channel)
  {
    match channel {
      _ => {
        log_warn_channel(channel)
      }
    }
  }
}

#[verus_verify(external_body)]
pub fn log_info(msg: &str)
{
  log::info!("{0}", msg);
}

#[verus_verify(external_body)]
pub fn log_warn_channel(channel: u32)
{
  log::warn!("Unexpected channel: {0}", channel);
}

// PLACEHOLDER MARKER GUMBO METHODS

// The verus! macro is required here because these items use Verus-specific syntax
// (open spec fn, exists|...|, `is` variant tests, `->` variant field access) that
// does not parse as valid Rust, so the attribute forms (#[verus_verify], #[verus_spec])
// cannot be applied — those only work on items that are already syntactically valid Rust.
//
// This is acceptable for mutation testing because the macro contains only spec functions
// (ghost code erased at runtime, validated by Verus verification rather than runtime tests)
// and constant definitions (no branching logic to mutate). All executable control flow
// lives outside the macro using attribute syntax, where mutation testing tools can
// operate normally.
verus! {
    /// MAVLink UDP source port used to identify MAVLink traffic.
    pub const MAV_UDP_SRC_PORT: u16 = 14550;
    /// MAVLink UDP destination port used to identify MAVLink traffic.
    pub const MAV_UDP_DST_PORT: u16 = 14562;

    /// Firewall port allowlists. Packets with destination ports not in these lists are dropped.
    mod config {
        /// TCP allowlist: only port 5760 (MAVLink TCP) is permitted.
        pub mod tcp {
            pub const ALLOWED_PORTS: [u16; 1] = [5760u16];
        }
        /// UDP allowlist: only port 68 (DHCP client) is permitted.
        pub mod udp {
            const NUM_UDP_PORTS: usize = 1;
            pub const ALLOWED_PORTS: [u16; NUM_UDP_PORTS] = [68u16];
        }
    }

    /// Spec predicate: true when the packet is an IPv4 TCP packet whose destination
    /// port appears in the TCP allowlist.
    pub open spec fn packet_is_whitelisted_tcp(packet: &firewall_core::PacketType) -> bool {
        packet is Ipv4 &&
            packet->Ipv4_0.protocol is Tcp &&
            ipv4_tcp_on_allowed_port_quant(packet->Ipv4_0.protocol->Tcp_0.dst_port)
    }

    /// Spec predicate: true when the packet is an IPv4 UDP packet whose destination
    /// port appears in the UDP allowlist.
    pub open spec fn packet_is_whitelisted_udp(packet: &firewall_core::PacketType) -> bool {
        packet is Ipv4 &&
            packet->Ipv4_0.protocol is Udp &&
            ipv4_udp_on_allowed_port_quant(packet->Ipv4_0.protocol->Udp_0.dst_port)
    }

    /// Spec predicate: true when the packet is an IPv4 UDP packet with MAVLink source
    /// and destination ports (14550 -> 14562).
    pub open spec fn packet_is_mavlink_udp(packet: &firewall_core::PacketType) -> bool {
        packet is Ipv4 &&
            packet->Ipv4_0.protocol is Udp &&
            packet->Ipv4_0.protocol->Udp_0.src_port == MAV_UDP_SRC_PORT &&
            packet->Ipv4_0.protocol->Udp_0.dst_port == MAV_UDP_DST_PORT
    }

    /// Spec predicate: true when the given port exists in the GUMBO-level UDP allowed ports list.
    pub open spec fn ipv4_udp_on_allowed_port_quant(port: u16) -> bool {
        exists|i: int| 0 <= i && i < GumboLib::UDP_ALLOWED_PORTS_spec().len() && GumboLib::UDP_ALLOWED_PORTS_spec()[i] == port
    }

    /// Spec predicate: true when the given port exists in the GUMBO-level TCP allowed ports list.
    pub open spec fn ipv4_tcp_on_allowed_port_quant(port: u16) -> bool {
        exists|i: int| 0 <= i && i < GumboLib::TCP_ALLOWED_PORTS_spec().len() && GumboLib::TCP_ALLOWED_PORTS_spec()[i] == port
    }
}

/// Parses a raw Ethernet frame into a structured `EthFrame` via `firewall_core`. The
/// ensures clauses bridge the GUMBO spec-level predicates (`valid_arp_spec`,
/// `valid_ipv4_udp_spec`, `valid_ipv4_tcp_spec`) — which operate on raw byte arrays — to
/// the firewall_core result predicates (`res_is_arp`, `res_is_udp`, `res_is_tcp`), which
/// operate on the parsed structure. Additional ensures connect the parsed port bytes to
/// the raw frame bytes (`udp_port_bytes_match`, `tcp_port_bytes_match`), enabling Verus
/// to verify port-based filtering decisions against the GUMBO contracts.
#[verus_verify]
#[verus_spec(r =>
    requires
        frame@.len() == SW::SW_RawEthernetMessage_DIM_0,
    ensures
        GumboLib::valid_arp_spec(*frame) == firewall_core::res_is_arp(r),
        GumboLib::valid_ipv4_udp_spec(*frame) == firewall_core::res_is_udp(r),
        GumboLib::valid_ipv4_tcp_spec(*frame) == firewall_core::res_is_tcp(r),
        GumboLib::valid_ipv4_tcp_spec(*frame) ==> firewall_core::tcp_port_bytes_match(frame, r),
        GumboLib::valid_ipv4_udp_spec(*frame) ==> firewall_core::udp_port_bytes_match(frame, r),
)]
fn get_frame_packet(frame: &SW::RawEthernetMessage) -> Option<firewall_core::EthFrame> {
    let eth = firewall_core::EthFrame::parse(frame);
    if eth.is_none() {
        log_info("Malformed packet. Throw it away.")
    }
    eth
}

/// Returns true if the parsed packet is a MAVLink UDP packet (src port 14550,
/// dst port 14562). MAVLink packets are routed to the MavlinkOut port rather
/// than the VmmOut port.
#[verus_verify]
#[verus_spec(r =>
    ensures
        packet_is_mavlink_udp(packet) == (r == true),
)]
fn can_send_to_mavlink(packet: &firewall_core::PacketType) -> bool {
    if let firewall_core::PacketType::Ipv4(ip) = packet {
        if let firewall_core::Ipv4ProtoPacket::Udp(udp) = &ip.protocol {
            return udp.src_port == MAV_UDP_SRC_PORT && udp.dst_port == MAV_UDP_DST_PORT;
        }
    }
    false
}

/// Splits a raw Ethernet frame into a `UdpFrame_Impl` containing separate header and
/// payload byte arrays. The headers are the first `EthIpUdpHeaders_DIM_0` bytes
/// (Ethernet + IP + UDP headers) and the payload is the remainder.
#[verus_verify]
#[verus_spec(r =>
    ensures
        r.headers@ =~= value@.subrange(0, SW::SW_EthIpUdpHeaders_DIM_0 as int),
        r.payload@ =~= value@.subrange(SW::SW_EthIpUdpHeaders_DIM_0 as int, SW::SW_RawEthernetMessage_DIM_0 as int),
)]
fn udp_frame_from_raw_eth(value: SW::RawEthernetMessage) -> SW::UdpFrame_Impl {
    let headers = udp_headers_from_raw_eth(value);
    let payload = udp_payload_from_raw_eth(value);
    SW::UdpFrame_Impl { headers, payload }
}

/// Copies the first `EthIpUdpHeaders_DIM_0` bytes from a raw Ethernet frame into
/// a fixed-size header array (Ethernet + IP + UDP headers).
#[verus_verify]
#[verus_spec(r =>
    ensures
        r@ =~= value@.subrange(0, SW::SW_EthIpUdpHeaders_DIM_0 as int),
)]
fn udp_headers_from_raw_eth(value: SW::RawEthernetMessage) -> SW::EthIpUdpHeaders {
    let mut headers = [0u8; SW::SW_EthIpUdpHeaders_DIM_0];
    let mut i = 0;
    #[verus_spec(
        invariant
            0 <= i <= headers@.len() < value@.len(),
            forall |j| 0 <= j < i ==> headers[j] == value[j],
        decreases
            SW::SW_EthIpUdpHeaders_DIM_0 - i,
    )]
    while i < SW::SW_EthIpUdpHeaders_DIM_0 {
        headers.set(i, value[i]);
        i += 1;
    }
    headers
}

/// Copies the bytes following the Ethernet/IP/UDP headers from a raw Ethernet frame
/// into a fixed-size payload array (everything after the first `EthIpUdpHeaders_DIM_0` bytes).
#[verus_verify]
#[verus_spec(r =>
    ensures
        r@ =~= value@.subrange(SW::SW_EthIpUdpHeaders_DIM_0 as int, SW::SW_RawEthernetMessage_DIM_0 as int),
)]
fn udp_payload_from_raw_eth(value: SW::RawEthernetMessage) -> SW::UdpPayload {
    let mut payload = [0u8; SW::SW_RawEthernetMessage_DIM_0 - SW::SW_EthIpUdpHeaders_DIM_0];
    let mut i = 0;
    #[verus_spec(
        invariant
            0 <= i <= payload@.len() <= value@.len() - SW::SW_EthIpUdpHeaders_DIM_0,
            forall |j| 0 <= j < i ==> #[trigger] payload[j] == value[j + SW::SW_EthIpUdpHeaders_DIM_0],
        decreases
            SW::SW_UdpPayload_DIM_0 - i,
    )]
    while i < SW::SW_UdpPayload_DIM_0 {
        payload.set(i, value[i + SW::SW_EthIpUdpHeaders_DIM_0]);
        i += 1;
    }
    payload
}

/// Returns true if the parsed packet should be forwarded to the VMM. ARP packets are
/// always allowed. IPv4 UDP packets are allowed only if their destination port is in
/// the UDP allowlist (port 68/DHCP). All other traffic (IPv4 TCP, IPv6, unknown
/// protocols) is rejected.
#[verus_verify]
#[verus_spec(r =>
    requires
        config::udp::ALLOWED_PORTS =~= GumboLib::UDP_ALLOWED_PORTS_spec(),
    ensures
        ((packet is Arp) || packet_is_whitelisted_udp(packet)) == (r == true),
)]
fn can_send_to_vmm(packet: &firewall_core::PacketType) -> bool {
    match packet {
        firewall_core::PacketType::Arp(_) => true,
        firewall_core::PacketType::Ipv4(ip) => match &ip.protocol {
            firewall_core::Ipv4ProtoPacket::Udp(udp) => {
                let allowed = udp_port_allowed(udp.dst_port);
                if !allowed {
                    log_info("UDP packet filtered out");
                }
                allowed
            }
            _ => {
                info_protocol(ip.header.protocol);
                false
            }
        },
        firewall_core::PacketType::Ipv6 => {
            log_info("Not an IPv4 or Arp packet. Throw it away.");
            false
        }
    }
}

/// Returns true if the given UDP destination port is in the UDP allowlist.
#[verus_verify]
#[verus_spec(r =>
    ensures
        r == config::udp::ALLOWED_PORTS@.contains(port),
)]
fn udp_port_allowed(port: u16) -> bool {
    port_allowed(&config::udp::ALLOWED_PORTS, port)
}

/// Returns true if the given TCP destination port is in the TCP allowlist.
#[verus_verify]
#[verus_spec(r =>
    ensures
        r == config::tcp::ALLOWED_PORTS@.contains(port),
)]
fn tcp_port_allowed(port: u16) -> bool {
    port_allowed(&config::tcp::ALLOWED_PORTS, port)
}

/// Generic allowlist check: linearly scans `allowed_ports` and returns true if `port`
/// is found. Used by both `udp_port_allowed` and `tcp_port_allowed`.
#[verus_verify]
#[verus_spec(r =>
    ensures
        r == allowed_ports@.contains(port),
)]
fn port_allowed(allowed_ports: &[u16], port: u16) -> bool {
    let mut i: usize = 0;
    #[verus_spec(
        invariant
            0 <= i <= allowed_ports@.len(),
            forall |j| 0 <= j < i ==> allowed_ports@[j] != port,
        decreases
            allowed_ports@.len() - i,
    )]
    while i < allowed_ports.len() {
        if allowed_ports[i] == port {
            return true;
        }
        i += 1;
    }
    false
}

/// Logs the IP protocol number of a dropped packet (neither TCP nor UDP).
#[verus_verify(external_body)]
fn info_protocol(protocol: firewall_core::IpProtocol) {
    log::info!("Not a TCP or UDP packet. ({:?}) Throw it away.", protocol);
}
