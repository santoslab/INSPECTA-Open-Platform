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
      //   https://jasonbelt.github.io/inspecta-open-platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=3
      api.EthernetFramesRxIn0.is_some() && GumboLib::valid_arp_spec(api.EthernetFramesRxIn0.unwrap()) ==>
        api.VmmOut0.is_some() &&
          ((api.EthernetFramesRxIn0.unwrap() == api.VmmOut0.unwrap()) &&
            api.MavlinkOut0.is_none()),
      // guarantee hlr_18_rx0_can_send_mavlink_udp
      //   https://jasonbelt.github.io/inspecta-open-platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=4
      api.EthernetFramesRxIn0.is_some() && GumboLib::valid_ipv4_udp_mavlink_spec(api.EthernetFramesRxIn0.unwrap()) ==>
        api.MavlinkOut0.is_some() &&
          (GumboLib::input_eq_mav_output_spec(api.EthernetFramesRxIn0.unwrap(), api.MavlinkOut0.unwrap()) && api.VmmOut0.is_none()),
      // guarantee hlr_13_rx0_can_send_ipv4_udp
      //   https://jasonbelt.github.io/inspecta-open-platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=3
      api.EthernetFramesRxIn0.is_some() && GumboLib::valid_ipv4_udp_port_spec(api.EthernetFramesRxIn0.unwrap()) ==>
        api.VmmOut0.is_some() &&
          ((api.EthernetFramesRxIn0.unwrap() == api.VmmOut0.unwrap()) &&
            api.MavlinkOut0.is_none()),
      // guarantee hlr_15_rx0_disallow
      //   https://jasonbelt.github.io/inspecta-open-platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=4
      api.EthernetFramesRxIn0.is_some() && !(GumboLib::rx_allow_outbound_frame_spec(api.EthernetFramesRxIn0.unwrap())) ==>
        (api.VmmOut0.is_none() && api.MavlinkOut0.is_none()),
      // guarantee hlr_17_rx0_no_input
      //   https://jasonbelt.github.io/inspecta-open-platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=4
      api.EthernetFramesRxIn0.is_some() ||
        (api.VmmOut0.is_none() && api.MavlinkOut0.is_none()),
      // guarantee hlr_05_rx1_can_send_arp_to_vmm
      //   https://jasonbelt.github.io/inspecta-open-platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=3
      api.EthernetFramesRxIn1.is_some() && GumboLib::valid_arp_spec(api.EthernetFramesRxIn1.unwrap()) ==>
        api.VmmOut1.is_some() &&
          ((api.EthernetFramesRxIn1.unwrap() == api.VmmOut1.unwrap()) &&
            api.MavlinkOut1.is_none()),
      // guarantee hlr_18_rx1_can_send_mavlink_udp
      //   https://jasonbelt.github.io/inspecta-open-platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=4
      api.EthernetFramesRxIn1.is_some() && GumboLib::valid_ipv4_udp_mavlink_spec(api.EthernetFramesRxIn1.unwrap()) ==>
        api.MavlinkOut1.is_some() &&
          (GumboLib::input_eq_mav_output_spec(api.EthernetFramesRxIn1.unwrap(), api.MavlinkOut1.unwrap()) && api.VmmOut1.is_none()),
      // guarantee hlr_13_rx1_can_send_ipv4_udp
      //   https://jasonbelt.github.io/inspecta-open-platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=3
      api.EthernetFramesRxIn1.is_some() && GumboLib::valid_ipv4_udp_port_spec(api.EthernetFramesRxIn1.unwrap()) ==>
        api.VmmOut1.is_some() &&
          ((api.EthernetFramesRxIn1.unwrap() == api.VmmOut1.unwrap()) &&
            api.MavlinkOut1.is_none()),
      // guarantee hlr_15_rx1_disallow
      //   https://jasonbelt.github.io/inspecta-open-platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=4
      api.EthernetFramesRxIn1.is_some() && !(GumboLib::rx_allow_outbound_frame_spec(api.EthernetFramesRxIn1.unwrap())) ==>
        (api.VmmOut1.is_none() && api.MavlinkOut1.is_none()),
      // guarantee hlr_17_rx1_no_input
      //   https://jasonbelt.github.io/inspecta-open-platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=4
      api.EthernetFramesRxIn1.is_some() ||
        (api.VmmOut1.is_none() && api.MavlinkOut1.is_none()),
      // guarantee hlr_05_rx2_can_send_arp_to_vmm
      //   https://jasonbelt.github.io/inspecta-open-platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=3
      api.EthernetFramesRxIn2.is_some() && GumboLib::valid_arp_spec(api.EthernetFramesRxIn2.unwrap()) ==>
        api.VmmOut2.is_some() &&
          ((api.EthernetFramesRxIn2.unwrap() == api.VmmOut2.unwrap()) &&
            api.MavlinkOut2.is_none()),
      // guarantee hlr_18_rx2_can_send_mavlink_udp
      //   https://jasonbelt.github.io/inspecta-open-platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=4
      api.EthernetFramesRxIn2.is_some() && GumboLib::valid_ipv4_udp_mavlink_spec(api.EthernetFramesRxIn2.unwrap()) ==>
        api.MavlinkOut2.is_some() &&
          (GumboLib::input_eq_mav_output_spec(api.EthernetFramesRxIn2.unwrap(), api.MavlinkOut2.unwrap()) && api.VmmOut2.is_none()),
      // guarantee hlr_13_rx2_can_send_ipv4_udp
      //   https://jasonbelt.github.io/inspecta-open-platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=3
      api.EthernetFramesRxIn2.is_some() && GumboLib::valid_ipv4_udp_port_spec(api.EthernetFramesRxIn2.unwrap()) ==>
        api.VmmOut2.is_some() &&
          ((api.EthernetFramesRxIn2.unwrap() == api.VmmOut2.unwrap()) &&
            api.MavlinkOut2.is_none()),
      // guarantee hlr_15_rx2_disallow
      //   https://jasonbelt.github.io/inspecta-open-platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=4
      api.EthernetFramesRxIn2.is_some() && !(GumboLib::rx_allow_outbound_frame_spec(api.EthernetFramesRxIn2.unwrap())) ==>
        (api.VmmOut2.is_none() && api.MavlinkOut2.is_none()),
      // guarantee hlr_17_rx2_no_input
      //   https://jasonbelt.github.io/inspecta-open-platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=4
      api.EthernetFramesRxIn2.is_some() ||
        (api.VmmOut2.is_none() && api.MavlinkOut2.is_none()),
      // guarantee hlr_05_rx3_can_send_arp_to_vmm
      //   https://jasonbelt.github.io/inspecta-open-platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=3
      api.EthernetFramesRxIn3.is_some() && GumboLib::valid_arp_spec(api.EthernetFramesRxIn3.unwrap()) ==>
        api.VmmOut3.is_some() &&
          ((api.EthernetFramesRxIn3.unwrap() == api.VmmOut3.unwrap()) &&
            api.MavlinkOut3.is_none()),
      // guarantee hlr_18_rx3_can_send_mavlink_udp
      //   https://jasonbelt.github.io/inspecta-open-platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=4
      api.EthernetFramesRxIn3.is_some() && GumboLib::valid_ipv4_udp_mavlink_spec(api.EthernetFramesRxIn3.unwrap()) ==>
        api.MavlinkOut3.is_some() &&
          (GumboLib::input_eq_mav_output_spec(api.EthernetFramesRxIn3.unwrap(), api.MavlinkOut3.unwrap()) && api.VmmOut3.is_none()),
      // guarantee hlr_13_rx3_can_send_ipv4_udp
      //   https://jasonbelt.github.io/inspecta-open-platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=3
      api.EthernetFramesRxIn3.is_some() && GumboLib::valid_ipv4_udp_port_spec(api.EthernetFramesRxIn3.unwrap()) ==>
        api.VmmOut3.is_some() &&
          ((api.EthernetFramesRxIn3.unwrap() == api.VmmOut3.unwrap()) &&
            api.MavlinkOut3.is_none()),
      // guarantee hlr_15_rx3_disallow
      //   https://jasonbelt.github.io/inspecta-open-platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=4
      api.EthernetFramesRxIn3.is_some() && !(GumboLib::rx_allow_outbound_frame_spec(api.EthernetFramesRxIn3.unwrap())) ==>
        (api.VmmOut3.is_none() && api.MavlinkOut3.is_none()),
      // guarantee hlr_17_rx3_no_input
      //   https://jasonbelt.github.io/inspecta-open-platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=4
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



fn get_frame_packet(frame: &SW::RawEthernetMessage) -> Option<firewall_core::EthFrame> {
    let eth = firewall_core::EthFrame::parse(frame);
    if eth.is_none() {
        log_info("Malformed packet. Throw it away.")
    }
    eth
}

fn can_send_to_mavlink(packet: &firewall_core::PacketType) -> bool {
    if let firewall_core::PacketType::Ipv4(ip) = packet {
        if let firewall_core::Ipv4ProtoPacket::Udp(udp) = &ip.protocol {
            return udp.src_port == MAV_UDP_SRC_PORT && udp.dst_port == MAV_UDP_DST_PORT;
        }
    }
    false
}

fn udp_frame_from_raw_eth(value: SW::RawEthernetMessage) -> SW::UdpFrame_Impl {
    let headers = udp_headers_from_raw_eth(value);
    let payload = udp_payload_from_raw_eth(value);
    SW::UdpFrame_Impl { headers, payload }
}

fn udp_headers_from_raw_eth(value: SW::RawEthernetMessage) -> SW::EthIpUdpHeaders {
    let mut headers = [0u8; SW::SW_EthIpUdpHeaders_DIM_0];
    let mut i = 0;
    while i < SW::SW_EthIpUdpHeaders_DIM_0 {
        headers.set(i, value[i]);
        i += 1;
    }
    headers
}

fn udp_payload_from_raw_eth(value: SW::RawEthernetMessage) -> SW::UdpPayload {
    let mut payload = [0u8; SW::SW_RawEthernetMessage_DIM_0 - SW::SW_EthIpUdpHeaders_DIM_0];
    let mut i = 0;
    while i < SW::SW_UdpPayload_DIM_0 {
        payload.set(i, value[i + SW::SW_EthIpUdpHeaders_DIM_0]);
        i += 1;
    }
    payload
}

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

fn udp_port_allowed(port: u16) -> bool {
    port_allowed(&config::udp::ALLOWED_PORTS, port)
}

fn tcp_port_allowed(port: u16) -> bool {
    port_allowed(&config::tcp::ALLOWED_PORTS, port)
}

fn port_allowed(allowed_ports: &[u16], port: u16) -> bool {
    let mut i: usize = 0;
    while i < allowed_ports.len() {
        if allowed_ports[i] == port {
            return true;
        }
        i += 1;
    }
    false
}

/// Logs the IP protocol number of a dropped packet (neither TCP nor UDP).
fn info_protocol(protocol: firewall_core::IpProtocol) {
    log::info!("Not a TCP or UDP packet. ({:?}) Throw it away.", protocol);
}
