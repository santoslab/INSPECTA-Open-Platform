// This file will not be overwritten if codegen is rerun

use crate::bridge::seL4_RxFirewall_RxFirewall_api::*;
use data::*;
#[cfg(feature = "sel4")]
use log::{debug, error, info, trace, warn};
use vstd::prelude::*;
// use vstd::slice::slice_subrange;
use SW::{
    EthIpUdpHeaders, SW_EthIpUdpHeaders_DIM_0, SW_UdpPayload_DIM_0, UdpFrame_Impl, UdpPayload,
};

use crate::SW::SW_RawEthernetMessage_DIM_0;
use firewall_core::{EthFrame, IpProtocol, Ipv4ProtoPacket, PacketType, TcpRepr, UdpRepr};
use GumboLib::*;

verus! {

    pub const MAV_UDP_SRC_PORT: u16 = 14550;
    pub const MAV_UDP_DST_PORT: u16 = 14562;

    fn udp_headers_from_raw_eth(value: SW::RawEthernetMessage) -> (r: EthIpUdpHeaders)
        ensures
            r@ =~= value@.subrange(0, SW_EthIpUdpHeaders_DIM_0 as int)
    {
        let mut headers = [0u8; SW_EthIpUdpHeaders_DIM_0];
        let mut i = 0;
        while i < SW_EthIpUdpHeaders_DIM_0
            invariant
                0 <= i <= headers@.len() < value@.len(),
                forall |j| 0 <= j < i ==> headers[j] == value[j],
            decreases
                SW_EthIpUdpHeaders_DIM_0 - i
        {
            headers.set(i, value[i]);
            i += 1;
        }
        headers
    }


    fn udp_payload_from_raw_eth(value: SW::RawEthernetMessage) -> (r: UdpPayload)
        ensures
            r@ =~= value@.subrange(SW_EthIpUdpHeaders_DIM_0 as int, SW_RawEthernetMessage_DIM_0 as int)
    {
        let mut payload = [0u8; SW_RawEthernetMessage_DIM_0-SW_EthIpUdpHeaders_DIM_0];

        let mut i = 0;
        while i < SW_UdpPayload_DIM_0
            invariant
                0 <= i <= payload@.len() <= value@.len()-SW_EthIpUdpHeaders_DIM_0,
                forall |j| 0 <= j < i ==> #[trigger] payload[j] == value[j+SW_EthIpUdpHeaders_DIM_0],
            decreases
                SW_UdpPayload_DIM_0 - i
        {
            payload.set(i, value[i+SW_EthIpUdpHeaders_DIM_0]);
            i += 1;
        }
        payload
    }

    fn udp_frame_from_raw_eth(value: SW::RawEthernetMessage) -> (r: UdpFrame_Impl)
        ensures
            r.headers@ =~= value@.subrange(0, SW_EthIpUdpHeaders_DIM_0 as int),
            r.payload@ =~= value@.subrange(SW_EthIpUdpHeaders_DIM_0 as int, SW_RawEthernetMessage_DIM_0 as int),
     {
        let headers = udp_headers_from_raw_eth(value);
        let payload = udp_payload_from_raw_eth(value);
        UdpFrame_Impl { headers, payload}
    }

    // mod config;

    // Testing
    mod config {
    pub mod tcp {
        pub const ALLOWED_PORTS: [u16; 1] = [5760u16];
    }

    pub mod udp {
        const NUM_UDP_PORTS: usize = 1;
        pub const ALLOWED_PORTS: [u16; NUM_UDP_PORTS] = [68u16];
    }
  }
    // End Testing

    const NUM_MSGS: usize = 4;

    pub struct seL4_RxFirewall_RxFirewall {
    // PLACEHOLDER MARKER STATE VARS
    }

    fn port_allowed(allowed_ports: &[u16], port: u16) -> (r: bool)
        ensures
            r == allowed_ports@.contains(port),
    {
        let mut i: usize = 0;
        while i < allowed_ports.len()
            invariant
                0 <= i <= allowed_ports@.len(),
                forall |j| 0 <= j < i ==> allowed_ports@[j] != port,
            decreases
                allowed_ports@.len() - i
        {
            if allowed_ports[i] == port {
                return true;
            }
            i += 1;
        }
        false
    }

    fn udp_port_allowed(port: u16) -> (r: bool)
        ensures
            r == config::udp::ALLOWED_PORTS@.contains(port),
    {
        port_allowed(&config::udp::ALLOWED_PORTS, port)
    }

    fn tcp_port_allowed(port: u16) -> (r: bool)
        ensures
            r == config::tcp::ALLOWED_PORTS@.contains(port),
    {
        port_allowed(&config::tcp::ALLOWED_PORTS, port)
    }

    pub open spec fn packet_is_whitelisted_tcp(packet: &PacketType) -> bool
    {
        packet is Ipv4 &&
            packet->Ipv4_0.protocol is Tcp &&
            ipv4_tcp_on_allowed_port_quant(packet->Ipv4_0.protocol->Tcp_0.dst_port)
    }


    pub open spec fn packet_is_whitelisted_udp(packet: &PacketType) -> bool
    {
        packet is Ipv4 &&
            packet->Ipv4_0.protocol is Udp &&
            ipv4_udp_on_allowed_port_quant(packet->Ipv4_0.protocol->Udp_0.dst_port)
    }

    fn can_send_to_vmm(packet: &PacketType) -> (r: bool)
        requires
            config::udp::ALLOWED_PORTS =~= UDP_ALLOWED_PORTS_spec(),
        ensures
            ((packet is Arp) ||
                packet_is_whitelisted_udp(packet)
            ) == (r == true),
    {
        match packet {
            PacketType::Arp(_) => true,
            PacketType::Ipv4(ip) => match &ip.protocol {
                // Ipv4ProtoPacket::Tcp(tcp) => {
                //     let allowed = tcp_port_allowed(tcp.dst_port);
                //     if !allowed {
                //         info("TCP packet filtered out");
                //     }
                //     allowed
                // }
                Ipv4ProtoPacket::Udp(udp) => {
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
            PacketType::Ipv6 => {
                log_info("Not an IPv4 or Arp packet. Throw it away.");
                false
            },
        }
    }

    pub open spec fn packet_is_mavlink_udp(packet: &PacketType) -> bool
    {
        packet is Ipv4 &&
            packet->Ipv4_0.protocol is Udp &&
            packet->Ipv4_0.protocol->Udp_0.src_port == MAV_UDP_SRC_PORT &&
            packet->Ipv4_0.protocol->Udp_0.dst_port == MAV_UDP_DST_PORT
    }

    fn can_send_to_mavlink(packet: &PacketType) -> (r: bool)
        ensures
            (packet_is_mavlink_udp(packet)) == (r == true),
    {
        if let PacketType::Ipv4(ip) = packet {
            if let Ipv4ProtoPacket::Udp(udp) = &ip.protocol {
                return udp.src_port == MAV_UDP_SRC_PORT && udp.dst_port == MAV_UDP_DST_PORT;
            }
        }
        false
    }

  impl seL4_RxFirewall_RxFirewall {
    pub fn new() -> Self
    {
      Self {
        // PLACEHOLDER MARKER STATE VAR INIT
      }
    }

    pub fn get_frame_packet(frame: &SW::RawEthernetMessage) -> (r: Option<EthFrame>)
        requires
            frame@.len() == SW_RawEthernetMessage_DIM_0
        ensures
            valid_arp_spec(*frame) == firewall_core::res_is_arp(r),
            valid_ipv4_udp_spec(*frame) == firewall_core::res_is_udp(r),
            valid_ipv4_tcp_spec(*frame) == firewall_core::res_is_tcp(r),
            valid_ipv4_tcp_spec(*frame) ==> firewall_core::tcp_port_bytes_match(frame, r),
            valid_ipv4_udp_spec(*frame) ==> firewall_core::udp_port_bytes_match(frame, r),
    {
        let eth = EthFrame::parse(frame);
        if eth.is_none() {
            log_info("Malformed packet. Throw it away.")
        }
        eth
    }

    pub fn initialize<API: seL4_RxFirewall_RxFirewall_Put_Api> (
      &mut self,
      api: &mut seL4_RxFirewall_RxFirewall_Application_Api<API>)
      ensures
        // PLACEHOLDER MARKER INITIALIZATION ENSURES
    {
      log_info("initialize entrypoint invoked");
    }

    pub fn timeTriggered<API: seL4_RxFirewall_RxFirewall_Full_Api> (
      &mut self,
      api: &mut seL4_RxFirewall_RxFirewall_Application_Api<API>)
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
        api.EthernetFramesRxIn0.is_some() && GumboLib::valid_arp_spec(api.EthernetFramesRxIn0.unwrap()) ==>
          api.VmmOut0.is_some() &&
            ((api.EthernetFramesRxIn0.unwrap() == api.VmmOut0.unwrap()) &&
              api.MavlinkOut0.is_none()),
        // guarantee hlr_18_rx0_can_send_mavlink_udp
        api.EthernetFramesRxIn0.is_some() && GumboLib::valid_ipv4_udp_mavlink_spec(api.EthernetFramesRxIn0.unwrap()) ==>
          api.MavlinkOut0.is_some() &&
            (GumboLib::input_eq_mav_output_spec(api.EthernetFramesRxIn0.unwrap(), api.MavlinkOut0.unwrap()) && api.VmmOut0.is_none()),
        // guarantee hlr_13_rx0_can_send_ipv4_udp
        api.EthernetFramesRxIn0.is_some() && GumboLib::valid_ipv4_udp_port_spec(api.EthernetFramesRxIn0.unwrap()) ==>
          api.VmmOut0.is_some() &&
            ((api.EthernetFramesRxIn0.unwrap() == api.VmmOut0.unwrap()) &&
              api.MavlinkOut0.is_none()),
        // guarantee hlr_15_rx0_disallow
        api.EthernetFramesRxIn0.is_some() && !(GumboLib::rx_allow_outbound_frame_spec(api.EthernetFramesRxIn0.unwrap())) ==>
          api.VmmOut0.is_none() && api.MavlinkOut0.is_none(),
        // guarantee hlr_17_rx0_no_input
        !(api.EthernetFramesRxIn0.is_some()) ==>
          api.VmmOut0.is_none() && api.MavlinkOut0.is_none(),
        // guarantee hlr_05_rx1_can_send_arp_to_vmm
        api.EthernetFramesRxIn1.is_some() && GumboLib::valid_arp_spec(api.EthernetFramesRxIn1.unwrap()) ==>
          api.VmmOut1.is_some() &&
            ((api.EthernetFramesRxIn1.unwrap() == api.VmmOut1.unwrap()) &&
              api.MavlinkOut1.is_none()),
        // guarantee hlr_18_rx1_can_send_mavlink_udp
        api.EthernetFramesRxIn1.is_some() && GumboLib::valid_ipv4_udp_mavlink_spec(api.EthernetFramesRxIn1.unwrap()) ==>
          api.MavlinkOut1.is_some() &&
            (GumboLib::input_eq_mav_output_spec(api.EthernetFramesRxIn1.unwrap(), api.MavlinkOut1.unwrap()) && api.VmmOut1.is_none()),
        // guarantee hlr_13_rx1_can_send_ipv4_udp
        api.EthernetFramesRxIn1.is_some() && GumboLib::valid_ipv4_udp_port_spec(api.EthernetFramesRxIn1.unwrap()) ==>
          api.VmmOut1.is_some() &&
            ((api.EthernetFramesRxIn1.unwrap() == api.VmmOut1.unwrap()) &&
              api.MavlinkOut1.is_none()),
        // guarantee hlr_15_rx1_disallow
        api.EthernetFramesRxIn1.is_some() && !(GumboLib::rx_allow_outbound_frame_spec(api.EthernetFramesRxIn1.unwrap())) ==>
          api.VmmOut1.is_none() && api.MavlinkOut1.is_none(),
        // guarantee hlr_17_rx1_no_input
        !(api.EthernetFramesRxIn1.is_some()) ==>
          api.VmmOut1.is_none() && api.MavlinkOut1.is_none(),
        // guarantee hlr_05_rx2_can_send_arp_to_vmm
        api.EthernetFramesRxIn2.is_some() && GumboLib::valid_arp_spec(api.EthernetFramesRxIn2.unwrap()) ==>
          api.VmmOut2.is_some() &&
            ((api.EthernetFramesRxIn2.unwrap() == api.VmmOut2.unwrap()) &&
              api.MavlinkOut2.is_none()),
        // guarantee hlr_18_rx2_can_send_mavlink_udp
        api.EthernetFramesRxIn2.is_some() && GumboLib::valid_ipv4_udp_mavlink_spec(api.EthernetFramesRxIn2.unwrap()) ==>
          api.MavlinkOut2.is_some() &&
            (GumboLib::input_eq_mav_output_spec(api.EthernetFramesRxIn2.unwrap(), api.MavlinkOut2.unwrap()) && api.VmmOut2.is_none()),
        // guarantee hlr_13_rx2_can_send_ipv4_udp
        api.EthernetFramesRxIn2.is_some() && GumboLib::valid_ipv4_udp_port_spec(api.EthernetFramesRxIn2.unwrap()) ==>
          api.VmmOut2.is_some() &&
            ((api.EthernetFramesRxIn2.unwrap() == api.VmmOut2.unwrap()) &&
              api.MavlinkOut2.is_none()),
        // guarantee hlr_15_rx2_disallow
        api.EthernetFramesRxIn2.is_some() && !(GumboLib::rx_allow_outbound_frame_spec(api.EthernetFramesRxIn2.unwrap())) ==>
          api.VmmOut2.is_none() && api.MavlinkOut2.is_none(),
        // guarantee hlr_17_rx2_no_input
        !(api.EthernetFramesRxIn2.is_some()) ==>
          api.VmmOut2.is_none() && api.MavlinkOut2.is_none(),
        // guarantee hlr_05_rx3_can_send_arp_to_vmm
        api.EthernetFramesRxIn3.is_some() && GumboLib::valid_arp_spec(api.EthernetFramesRxIn3.unwrap()) ==>
          api.VmmOut3.is_some() &&
            ((api.EthernetFramesRxIn3.unwrap() == api.VmmOut3.unwrap()) &&
              api.MavlinkOut3.is_none()),
        // guarantee hlr_18_rx3_can_send_mavlink_udp
        api.EthernetFramesRxIn3.is_some() && GumboLib::valid_ipv4_udp_mavlink_spec(api.EthernetFramesRxIn3.unwrap()) ==>
          api.MavlinkOut3.is_some() &&
            (GumboLib::input_eq_mav_output_spec(api.EthernetFramesRxIn3.unwrap(), api.MavlinkOut3.unwrap()) && api.VmmOut3.is_none()),
        // guarantee hlr_13_rx3_can_send_ipv4_udp
        api.EthernetFramesRxIn3.is_some() && GumboLib::valid_ipv4_udp_port_spec(api.EthernetFramesRxIn3.unwrap()) ==>
          api.VmmOut3.is_some() &&
            ((api.EthernetFramesRxIn3.unwrap() == api.VmmOut3.unwrap()) &&
              api.MavlinkOut3.is_none()),
        // guarantee hlr_15_rx3_disallow
        api.EthernetFramesRxIn3.is_some() && !(GumboLib::rx_allow_outbound_frame_spec(api.EthernetFramesRxIn3.unwrap())) ==>
          api.VmmOut3.is_none() && api.MavlinkOut3.is_none(),
        // guarantee hlr_17_rx3_no_input
        !(api.EthernetFramesRxIn3.is_some()) ==>
          api.VmmOut3.is_none() && api.MavlinkOut3.is_none(),
        // END MARKER TIME TRIGGERED ENSURES
    {
        // Rx0 ports
        if let Some(frame) = api.get_EthernetFramesRxIn0() {
            if let Some(eth) = Self::get_frame_packet(&frame) {
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
            if let Some(eth) = Self::get_frame_packet(&frame) {
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
            if let Some(eth) = Self::get_frame_packet(&frame) {
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
            if let Some(eth) = Self::get_frame_packet(&frame) {
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
      // this method is called when the monitor does not handle the passed in channel
      match channel {
        _ => {
          log_warn_channel(channel)
        }
      }
    }
  }

  #[verifier::external_body]
  fn info_protocol(protocol: IpProtocol) {
      log::info!("Not a TCP or UDP packet. ({:?}) Throw it away.", protocol);
  }

  #[verifier::external_body]
  pub fn log_info(msg: &str)
  {
    log::info!("{0}", msg);
  }

  #[verifier::external_body]
  pub fn log_warn_channel(channel: u32)
  {
    log::warn!("Unexpected channel: {0}", channel);
  }

  pub open spec fn ipv4_udp_on_allowed_port_quant(port: u16) -> bool
  {
      exists|i:int| 0 <= i && i < UDP_ALLOWED_PORTS_spec().len() && UDP_ALLOWED_PORTS_spec()[i] == port
  }

  pub open spec fn ipv4_tcp_on_allowed_port_quant(port: u16) -> bool
  {
      exists|i:int| 0 <= i && i < TCP_ALLOWED_PORTS_spec().len() && TCP_ALLOWED_PORTS_spec()[i] == port
  }


  // BEGIN MARKER GUMBO METHODS
  // pub open spec fn TCP_ALLOWED_PORTS() -> SW::u16Array
  // {
  //   [5760u16]
  // }

  // pub open spec fn UDP_ALLOWED_PORTS() -> SW::u16Array
  // {
  //   [68u16]
  // }

  // pub open spec fn two_bytes_to_u16(
  //   byte0: u8,
  //   byte1: u8) -> u16
  // {
  //   (((byte0) as u16) * 256u16 + ((byte1) as u16)) as u16
  // }

  // pub open spec fn frame_is_wellformed_eth2(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   valid_frame_ethertype(aframe) && valid_frame_dst_addr(aframe)
  // }

  // pub open spec fn valid_frame_ethertype(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   frame_has_ipv4(aframe) ||
  //     (frame_has_arp(aframe) || frame_has_ipv6(aframe))
  // }

  // pub open spec fn valid_frame_dst_addr(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   (aframe.len() == 1600) &&
  //     !((aframe[0] == 0u8) &&
  //       ((aframe[1] == 0u8) &&
  //         ((aframe[2] == 0u8) &&
  //           ((aframe[3] == 0u8) &&
  //             ((aframe[4] == 0u8) &&
  //               (aframe[5] == 0u8))))))
  // }

  // pub open spec fn frame_has_ipv4(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   (aframe.len() == 1600) &&
  //     ((aframe[12] == 8u8) &&
  //       (aframe[13] == 0u8))
  // }

  // pub open spec fn frame_has_ipv6(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   (aframe.len() == 1600) &&
  //     ((aframe[12] == 134u8) &&
  //       (aframe[13] == 221u8))
  // }

  // pub open spec fn frame_has_arp(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   (aframe.len() == 1600) &&
  //     ((aframe[12] == 8u8) &&
  //       (aframe[13] == 6u8))
  // }

  // pub open spec fn arp_has_ipv4(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   (aframe.len() == 1600) &&
  //     ((aframe[16] == 8u8) &&
  //       (aframe[17] == 0u8))
  // }

  // pub open spec fn arp_has_ipv6(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   (aframe.len() == 1600) &&
  //     ((aframe[16] == 134u8) &&
  //       (aframe[17] == 221u8))
  // }

  // pub open spec fn valid_arp_ptype(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   arp_has_ipv4(aframe) || arp_has_ipv6(aframe)
  // }

  // pub open spec fn valid_arp_op(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   (aframe.len() == 1600) &&
  //     ((aframe[20] == 0u8) &&
  //       ((aframe[21] == 1u8) ||
  //         (aframe[21] == 2u8)))
  // }

  // pub open spec fn valid_arp_htype(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   (aframe.len() == 1600) &&
  //     ((aframe[14] == 0u8) &&
  //       (aframe[15] == 1u8))
  // }

  // pub open spec fn wellformed_arp_frame(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   valid_arp_op(aframe) &&
  //     (valid_arp_htype(aframe) && valid_arp_ptype(aframe))
  // }

  // pub open spec fn valid_ipv4_length(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   (aframe.len() == 1600) &&
  //     (two_bytes_to_u16(aframe[16], aframe[17]) <= 9000u16)
  // }

  // pub open spec fn valid_ipv4_protocol(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   (aframe.len() == 1600) &&
  //     ((aframe[23] == 0u8) ||
  //       ((aframe[23] == 1u8) ||
  //         ((aframe[23] == 2u8) ||
  //           ((aframe[23] == 6u8) ||
  //             ((aframe[23] == 17u8) ||
  //               ((aframe[23] == 43u8) ||
  //                 ((aframe[23] == 44u8) ||
  //                   ((aframe[23] == 58u8) ||
  //                     ((aframe[23] == 59u8) ||
  //                       (aframe[23] == 60u8))))))))))
  // }

  // pub open spec fn valid_ipv4_vers_ihl(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   (aframe.len() == 1600) &&
  //     (aframe[14] == 69u8)
  // }

  // pub open spec fn wellformed_ipv4_frame(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   valid_ipv4_protocol(aframe) &&
  //     (valid_ipv4_length(aframe) && valid_ipv4_vers_ihl(aframe))
  // }

  // pub open spec fn ipv4_is_tcp(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   (aframe.len() == 1600) &&
  //     (aframe[23] == 6u8)
  // }

  // pub open spec fn ipv4_is_udp(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   (aframe.len() == 1600) &&
  //     (aframe[23] == 17u8)
  // }

  // pub open spec fn tcp_is_valid_port(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   (aframe.len() == 1600) &&
  //     (two_bytes_to_u16(aframe[36], aframe[37]) == TCP_ALLOWED_PORTS()[0])
  // }

  // pub open spec fn udp_is_valid_port(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   (aframe.len() == 1600) &&
  //     (two_bytes_to_u16(aframe[36], aframe[37]) == UDP_ALLOWED_PORTS()[0])
  // }

  // pub open spec fn udp_is_mavlink_src_port(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   (aframe.len() == 1600) &&
  //     (two_bytes_to_u16(aframe[34], aframe[35]) == 14550u16)
  // }

  // pub open spec fn udp_is_mavlink_dst_port(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   (aframe.len() == 1600) &&
  //     (two_bytes_to_u16(aframe[36], aframe[37]) == 14562u16)
  // }

  // pub open spec fn udp_is_mavlink(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   udp_is_mavlink_src_port(aframe) && udp_is_mavlink_dst_port(aframe)
  // }

  // pub open spec fn frame_has_ipv4_tcp_on_allowed_port_quant(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   (aframe.len() == 1600) &&
  //     exists|i:int| 0 <= i <= TCP_ALLOWED_PORTS().len() - 1 && #[trigger] TCP_ALLOWED_PORTS()[i] == two_bytes_to_u16(aframe[36], aframe[37])
  // }

  // pub open spec fn udp_is_valid_direct_dst_port(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   (aframe.len() == 1600) &&
  //     exists|i:int| 0 <= i <= UDP_ALLOWED_PORTS().len() - 1 && #[trigger] UDP_ALLOWED_PORTS()[i] == two_bytes_to_u16(aframe[36], aframe[37])
  // }

  // pub open spec fn valid_arp(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   frame_is_wellformed_eth2(aframe) &&
  //     (frame_has_arp(aframe) && wellformed_arp_frame(aframe))
  // }

  // pub open spec fn valid_ipv4_tcp(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   frame_is_wellformed_eth2(aframe) &&
  //     (frame_has_ipv4(aframe) &&
  //       (wellformed_ipv4_frame(aframe) && ipv4_is_tcp(aframe)))
  // }

  // pub open spec fn valid_ipv4_udp(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   frame_is_wellformed_eth2(aframe) &&
  //     (frame_has_ipv4(aframe) &&
  //       (wellformed_ipv4_frame(aframe) && ipv4_is_udp(aframe)))
  // }

  // pub open spec fn valid_ipv4_tcp_port(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   valid_ipv4_tcp(aframe) && frame_has_ipv4_tcp_on_allowed_port_quant(aframe)
  // }

  // pub open spec fn valid_ipv4_udp_port(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   valid_ipv4_udp(aframe) &&
  //     (udp_is_valid_direct_dst_port(aframe) && !(udp_is_mavlink(aframe)))
  // }

  // pub open spec fn valid_ipv4_udp_mavlink(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   valid_ipv4_udp(aframe) && udp_is_mavlink(aframe)
  // }

  // pub open spec fn allow_outbound_frame(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   valid_arp(aframe) ||
  //     (valid_ipv4_udp_mavlink(aframe) || valid_ipv4_udp_port(aframe))
  // }

  // pub open spec fn input_eq_mav_output_headers(
  //   aframe: SW::RawEthernetMessage,
  //   headers: SW::EthIpUdpHeaders) -> bool
  // {
  //   (aframe.len() == 1600) &&
  //     forall|i:int| 0 <= i <= headers.len() - 1 ==> #[trigger] headers[i] == aframe[i]
  // }

  // pub open spec fn input_eq_mav_output_payload(
  //   aframe: SW::RawEthernetMessage,
  //   payload: SW::UdpPayload,
  //   headers: SW::EthIpUdpHeaders) -> bool
  // {
  //   (aframe.len() == 1600) &&
  //     ((payload.len() == 1558) &&
  //       forall|i:int| 0 <= i <= payload.len() - 1 ==> #[trigger] aframe[i + headers.len()] == payload[i])
  // }

  // pub open spec fn input_eq_mav_output(
  //   aframe: SW::RawEthernetMessage,
  //   output: SW::UdpFrame_Impl) -> bool
  // {
  //   input_eq_mav_output_headers(aframe, output.headers) && input_eq_mav_output_payload(aframe, output.payload, output.headers)
  // }
  // END MARKER GUMBO METHODS
}

// #[test]
// fn tcp_port_allowed_test() {
//     assert!(tcp_port_allowed(5760));
//     assert!(!tcp_port_allowed(42));
// }

// #[test]
// fn udp_port_allowed_test() {
//     assert!(udp_port_allowed(68));
//     assert!(!udp_port_allowed(19));
// }

// #[cfg(test)]
// mod parse_frame_tests {
//     use super::*;

//     #[test]
//     fn parse_malformed_packet() {
//         let mut frame = [0u8; 1600];
//         let pkt = [
//             0xffu8, 0xff, 0xff, 0xff, 0xff, 0xff, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x02, 0xC2,
//         ];
//         frame[0..14].copy_from_slice(&pkt);
//         let res = seL4_RxFirewall_RxFirewall::get_frame_packet(&frame);
//         assert!(res.is_none());
//     }

//     #[test]
//     fn parse_valid_arp() {
//         let mut frame = [0u8; 1600];
//         let pkt = [
//             0xffu8, 0xff, 0xff, 0xff, 0xff, 0xff, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x08, 0x06, 0x0,
//             0x1, 0x8, 0x0, 0x6, 0x4, 0x0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0xc0, 0xa8, 0x0, 0x1,
//             0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0xc0, 0xa8, 0x0, 0xce,
//         ];
//         frame[0..42].copy_from_slice(&pkt);
//         let res = seL4_RxFirewall_RxFirewall::get_frame_packet(&frame);
//         assert!(res.is_some());
//     }
// }

// #[cfg(test)]
// mod can_send_tests {
//     use super::*;
//     use firewall_core::{
//         Address, Arp, ArpOp, EtherType, HardwareType, IpProtocol, Ipv4Address, Ipv4Packet, Ipv4Repr,
//     };

//     #[test]
//     fn packet_valid_arp_request() {
//         let packet = PacketType::Arp(Arp {
//             htype: HardwareType::Ethernet,
//             ptype: EtherType::Ipv4,
//             hsize: 0x6,
//             psize: 0x4,
//             op: ArpOp::Request,
//             src_addr: Address([0x2, 0x3, 0x4, 0x5, 0x6, 0x7]),
//             src_protocol_addr: Ipv4Address([0xc0, 0xa8, 0x00, 0x01]),
//             dest_addr: Address([0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
//             dest_protocol_addr: Ipv4Address([0xc0, 0xa8, 0x0, 0xce]),
//         });
//         assert!(can_send_to_vmm(&packet));
//     }

//     #[test]
//     fn packet_valid_arp_reply() {
//         let packet = PacketType::Arp(Arp {
//             htype: HardwareType::Ethernet,
//             ptype: EtherType::Ipv4,
//             hsize: 0x6,
//             psize: 0x4,
//             op: ArpOp::Reply,
//             src_addr: Address([0x18, 0x20, 0x22, 0x24, 0x26, 0x28]),
//             src_protocol_addr: Ipv4Address([0xc0, 0xa8, 0x00, 0xce]),
//             dest_addr: Address([0x2, 0x3, 0x4, 0x5, 0x6, 0x7]),
//             dest_protocol_addr: Ipv4Address([0xc0, 0xa8, 0x0, 0x01]),
//         });
//         assert!(can_send_to_vmm(&packet));
//     }

//     #[test]
//     fn packet_invalid_ipv6() {
//         let packet = PacketType::Ipv6;
//         assert!(!can_send_to_vmm(&packet));
//     }

//     #[test]
//     fn invalid_ipv4_protocols() {
//         // Hop by Hop
//         let mut packet = PacketType::Ipv4(Ipv4Packet {
//             header: Ipv4Repr {
//                 protocol: IpProtocol::HopByHop,
//                 length: 0x29,
//             },
//             protocol: Ipv4ProtoPacket::HopByHop,
//         });
//         assert!(!can_send_to_vmm(&packet));

//         // ICMP
//         if let PacketType::Ipv4(ip) = &mut packet {
//             ip.header.protocol = IpProtocol::Icmp;
//         }
//         assert!(!can_send_to_vmm(&packet));

//         // IGMP
//         if let PacketType::Ipv4(ip) = &mut packet {
//             ip.header.protocol = IpProtocol::Igmp;
//         }
//         assert!(!can_send_to_vmm(&packet));

//         // Ipv6 Route
//         if let PacketType::Ipv4(ip) = &mut packet {
//             ip.header.protocol = IpProtocol::Ipv6Route;
//         }
//         assert!(!can_send_to_vmm(&packet));

//         // Ipv6 Frag
//         if let PacketType::Ipv4(ip) = &mut packet {
//             ip.header.protocol = IpProtocol::Ipv6Frag;
//         }
//         assert!(!can_send_to_vmm(&packet));

//         // ICMPv6
//         if let PacketType::Ipv4(ip) = &mut packet {
//             ip.header.protocol = IpProtocol::Icmpv6;
//         }
//         assert!(!can_send_to_vmm(&packet));

//         // IPv6 No Nxt
//         if let PacketType::Ipv4(ip) = &mut packet {
//             ip.header.protocol = IpProtocol::Ipv6NoNxt;
//         }
//         assert!(!can_send_to_vmm(&packet));

//         // IPv6 Opts
//         if let PacketType::Ipv4(ip) = &mut packet {
//             ip.header.protocol = IpProtocol::Ipv6Opts;
//         }
//         assert!(!can_send_to_vmm(&packet));
//     }

//     #[test]
//     fn disallowed_tcp() {
//         let packet = PacketType::Ipv4(Ipv4Packet {
//             header: Ipv4Repr {
//                 protocol: IpProtocol::Tcp,
//                 length: 0x29,
//             },
//             protocol: Ipv4ProtoPacket::Tcp(TcpRepr { dst_port: 443 }),
//         });
//         assert!(!can_send_to_vmm(&packet));
//     }

//     #[test]
//     fn allowed_tcp() {
//         let packet = PacketType::Ipv4(Ipv4Packet {
//             header: Ipv4Repr {
//                 protocol: IpProtocol::Tcp,
//                 length: 0x29,
//             },
//             protocol: Ipv4ProtoPacket::Tcp(TcpRepr { dst_port: 5760 }),
//         });
//         assert!(can_send_to_vmm(&packet));
//     }

//     #[test]
//     fn disallowed_udp() {
//         let packet = PacketType::Ipv4(Ipv4Packet {
//             header: Ipv4Repr {
//                 protocol: IpProtocol::Udp,
//                 length: 0x29,
//             },
//             protocol: Ipv4ProtoPacket::Udp(UdpRepr { dst_port: 15 }),
//         });
//         assert!(!can_send_to_vmm(&packet));
//     }

//     #[test]
//     fn allowed_udp() {
//         let packet = PacketType::Ipv4(Ipv4Packet {
//             header: Ipv4Repr {
//                 protocol: IpProtocol::Udp,
//                 length: 0x29,
//             },
//             protocol: Ipv4ProtoPacket::Udp(UdpRepr { dst_port: 68 }),
//         });
//         assert!(can_send_to_vmm(&packet));
//     }
// }
