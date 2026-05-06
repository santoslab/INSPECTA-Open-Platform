// This file will not be overwritten if codegen is rerun

use crate::bridge::seL4_TxFirewall_TxFirewall_api::*;
use data::*;
#[cfg(feature = "sel4")]
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use vstd::prelude::*;

use crate::bridge::seL4_TxFirewall_TxFirewall_GUMBOX as gumbox;
use firewall_core::{Arp, EthFrame, EthernetRepr, Ipv4Packet, PacketType};
mod config;
use crate::SW::SW_RawEthernetMessage_DIM_0;
use GumboLib::*;

verus! {

  fn can_send_packet(packet: &PacketType) -> (r: Option<u16>)
      requires
          (packet is Ipv4) ==> (firewall_core::ipv4_valid_length(*packet))
      ensures
          (packet is Arp || packet is Ipv4) == r.is_some(),
          packet is Arp ==> (r == Some(64u16)),
          packet is Ipv4 ==> (r == Some((packet->Ipv4_0.header.length + EthernetRepr::SIZE) as u16)),
  {
      match packet {
          PacketType::Arp(_) => Some(64u16),
          PacketType::Ipv4(ip) => Some(ip.header.length + EthernetRepr::SIZE as u16),
          PacketType::Ipv6 => {
              log_info("IPv6 packet: Throw it away.");
              None
          }
      }
  }

  pub struct seL4_TxFirewall_TxFirewall {
    // PLACEHOLDER MARKER STATE VARS
  }

  impl seL4_TxFirewall_TxFirewall {
    pub fn new() -> Self
    {
      Self {
        // PLACEHOLDER MARKER STATE VAR INIT
      }
    }

    fn get_frame_packet(frame: &SW::RawEthernetMessage) -> (r: Option<EthFrame>)
    requires
        frame@.len() == SW_RawEthernetMessage_DIM_0
    ensures
        valid_arp_spec(*frame) == firewall_core::res_is_arp(r),
        valid_ipv4_spec(*frame) == firewall_core::res_is_ipv4(r),
        valid_ipv4_spec(*frame) ==> firewall_core::ipv4_length_bytes_match(frame, r),
    {
        let eth = EthFrame::parse(frame);
        if eth.is_none() {
            log_info("Malformed packet. Throw it away.")
        }
        eth
    }

    pub fn initialize<API: seL4_TxFirewall_TxFirewall_Put_Api> (
      &mut self,
      api: &mut seL4_TxFirewall_TxFirewall_Application_Api<API>)
      ensures
        // PLACEHOLDER MARKER INITIALIZATION ENSURES
    {
      log_info("initialize entrypoint invoked");
    }

    pub fn timeTriggered<API: seL4_TxFirewall_TxFirewall_Full_Api> (
      &mut self,
      api: &mut seL4_TxFirewall_TxFirewall_Application_Api<API>)
      requires
        // BEGIN MARKER TIME TRIGGERED REQUIRES
        // assume AADL_Requirement
        //   All outgoing event ports must be empty
        old(api).EthernetFramesTxOut0.is_none(),
        old(api).EthernetFramesTxOut1.is_none(),
        old(api).EthernetFramesTxOut2.is_none(),
        old(api).EthernetFramesTxOut3.is_none(),
        // END MARKER TIME TRIGGERED REQUIRES
      ensures
        // BEGIN MARKER TIME TRIGGERED ENSURES
        // guarantee hlr_07_tx0_can_send_valid_arp
        api.EthernetFramesTxIn0.is_some() && GumboLib::valid_arp_spec(api.EthernetFramesTxIn0.unwrap()) ==>
          api.EthernetFramesTxOut0.is_some() &&
            ((api.EthernetFramesTxIn0.unwrap() == api.EthernetFramesTxOut0.unwrap().amessage) &&
              GumboLib::valid_output_arp_size_spec(api.EthernetFramesTxOut0.unwrap())),
        // guarantee hlr_12_tx0_can_send_valid_ipv4
        api.EthernetFramesTxIn0.is_some() && GumboLib::valid_ipv4_spec(api.EthernetFramesTxIn0.unwrap()) ==>
          api.EthernetFramesTxOut0.is_some() &&
            ((api.EthernetFramesTxIn0.unwrap() == api.EthernetFramesTxOut0.unwrap().amessage) &&
              GumboLib::valid_output_ipv4_size_spec(api.EthernetFramesTxIn0.unwrap(), api.EthernetFramesTxOut0.unwrap())),
        // guarantee hlr_14_tx0_disallow
        api.EthernetFramesTxIn0.is_some() && !(GumboLib::tx_allow_outbound_frame_spec(api.EthernetFramesTxIn0.unwrap())) ==>
          api.EthernetFramesTxOut0.is_none(),
        // guarantee hlr_16_tx0_no_input
        !(api.EthernetFramesTxIn0.is_some()) ==> api.EthernetFramesTxOut0.is_none(),
        // guarantee hlr_07_tx1_can_send_valid_arp
        api.EthernetFramesTxIn1.is_some() && GumboLib::valid_arp_spec(api.EthernetFramesTxIn1.unwrap()) ==>
          api.EthernetFramesTxOut1.is_some() &&
            ((api.EthernetFramesTxIn1.unwrap() == api.EthernetFramesTxOut1.unwrap().amessage) &&
              GumboLib::valid_output_arp_size_spec(api.EthernetFramesTxOut1.unwrap())),
        // guarantee hlr_12_tx1_can_send_valid_ipv4
        api.EthernetFramesTxIn1.is_some() && GumboLib::valid_ipv4_spec(api.EthernetFramesTxIn1.unwrap()) ==>
          api.EthernetFramesTxOut1.is_some() &&
            ((api.EthernetFramesTxIn1.unwrap() == api.EthernetFramesTxOut1.unwrap().amessage) &&
              GumboLib::valid_output_ipv4_size_spec(api.EthernetFramesTxIn1.unwrap(), api.EthernetFramesTxOut1.unwrap())),
        // guarantee hlr_14_tx1_disallow
        api.EthernetFramesTxIn1.is_some() && !(GumboLib::tx_allow_outbound_frame_spec(api.EthernetFramesTxIn1.unwrap())) ==>
          api.EthernetFramesTxOut1.is_none(),
        // guarantee hlr_16_tx1_no_input
        !(api.EthernetFramesTxIn1.is_some()) ==> api.EthernetFramesTxOut1.is_none(),
        // guarantee hlr_07_tx2_can_send_valid_arp
        api.EthernetFramesTxIn2.is_some() && GumboLib::valid_arp_spec(api.EthernetFramesTxIn2.unwrap()) ==>
          api.EthernetFramesTxOut2.is_some() &&
            ((api.EthernetFramesTxIn2.unwrap() == api.EthernetFramesTxOut2.unwrap().amessage) &&
              GumboLib::valid_output_arp_size_spec(api.EthernetFramesTxOut2.unwrap())),
        // guarantee hlr_12_tx2_can_send_valid_ipv4
        api.EthernetFramesTxIn2.is_some() && GumboLib::valid_ipv4_spec(api.EthernetFramesTxIn2.unwrap()) ==>
          api.EthernetFramesTxOut2.is_some() &&
            ((api.EthernetFramesTxIn2.unwrap() == api.EthernetFramesTxOut2.unwrap().amessage) &&
              GumboLib::valid_output_ipv4_size_spec(api.EthernetFramesTxIn2.unwrap(), api.EthernetFramesTxOut2.unwrap())),
        // guarantee hlr_14_tx2_disallow
        api.EthernetFramesTxIn2.is_some() && !(GumboLib::tx_allow_outbound_frame_spec(api.EthernetFramesTxIn2.unwrap())) ==>
          api.EthernetFramesTxOut2.is_none(),
        // guarantee hlr_16_tx2_no_input
        !(api.EthernetFramesTxIn2.is_some()) ==> api.EthernetFramesTxOut2.is_none(),
        // guarantee hlr_07_tx3_can_send_valid_arp
        api.EthernetFramesTxIn3.is_some() && GumboLib::valid_arp_spec(api.EthernetFramesTxIn3.unwrap()) ==>
          api.EthernetFramesTxOut3.is_some() &&
            ((api.EthernetFramesTxIn3.unwrap() == api.EthernetFramesTxOut3.unwrap().amessage) &&
              GumboLib::valid_output_arp_size_spec(api.EthernetFramesTxOut3.unwrap())),
        // guarantee hlr_12_tx3_can_send_valid_ipv4
        api.EthernetFramesTxIn3.is_some() && GumboLib::valid_ipv4_spec(api.EthernetFramesTxIn3.unwrap()) ==>
          api.EthernetFramesTxOut3.is_some() &&
            ((api.EthernetFramesTxIn3.unwrap() == api.EthernetFramesTxOut3.unwrap().amessage) &&
              GumboLib::valid_output_ipv4_size_spec(api.EthernetFramesTxIn3.unwrap(), api.EthernetFramesTxOut3.unwrap())),
        // guarantee hlr_14_tx3_disallow
        api.EthernetFramesTxIn3.is_some() && !(GumboLib::tx_allow_outbound_frame_spec(api.EthernetFramesTxIn3.unwrap())) ==>
          api.EthernetFramesTxOut3.is_none(),
        // guarantee hlr_16_tx3_no_input
        !(api.EthernetFramesTxIn3.is_some()) ==> api.EthernetFramesTxOut3.is_none(),
        // END MARKER TIME TRIGGERED ENSURES
    {
      // Tx0 ports
      if let Some(frame) = api.get_EthernetFramesTxIn0() {
          if let Some(eth) = Self::get_frame_packet(&frame) {
              if let Some(size) = can_send_packet(&eth.eth_type) {
                  let out = SW::SizedEthernetMessage_Impl {
                      sz: size,
                      amessage: frame,
                  };
                  api.put_EthernetFramesTxOut0(out);
              }
          }
      }

        // Tx1 ports
        if let Some(frame) = api.get_EthernetFramesTxIn1() {
            if let Some(eth) = Self::get_frame_packet(&frame) {
                if let Some(size) = can_send_packet(&eth.eth_type) {
                    let out = SW::SizedEthernetMessage_Impl {
                        sz: size,
                        amessage: frame,
                    };
                    api.put_EthernetFramesTxOut1(out);
                }
            }
        }

        // Tx2 ports
        if let Some(frame) = api.get_EthernetFramesTxIn2() {
            if let Some(eth) = Self::get_frame_packet(&frame) {
                if let Some(size) = can_send_packet(&eth.eth_type) {
                    let out = SW::SizedEthernetMessage_Impl {
                        sz: size,
                        amessage: frame,
                    };
                    api.put_EthernetFramesTxOut2(out);
                }
            }
        }

        // Tx3 ports
        if let Some(frame) = api.get_EthernetFramesTxIn3() {
            if let Some(eth) = Self::get_frame_packet(&frame) {
                if let Some(size) = can_send_packet(&eth.eth_type) {
                    let out = SW::SizedEthernetMessage_Impl {
                        sz: size,
                        amessage: frame,
                    };
                    api.put_EthernetFramesTxOut3(out);
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
  pub fn log_info(msg: &str)
  {
    log::info!("{0}", msg);
  }

  #[verifier::external_body]
  pub fn log_warn_channel(channel: u32)
  {
    log::warn!("Unexpected channel: {0}", channel);
  }

  // BEGIN MARKER GUMBO METHODS
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

  // pub open spec fn valid_ipv6(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   frame_is_wellformed_eth2(aframe) && frame_has_ipv6(aframe)
  // }

  // pub open spec fn valid_arp(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   frame_is_wellformed_eth2(aframe) &&
  //     (frame_has_arp(aframe) && wellformed_arp_frame(aframe))
  // }

  // pub open spec fn valid_ipv4(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   frame_is_wellformed_eth2(aframe) &&
  //     (frame_has_ipv4(aframe) && wellformed_ipv4_frame(aframe))
  // }

  // pub open spec fn ipv4_length(aframe: SW::RawEthernetMessage) -> u16
  // {
  //   two_bytes_to_u16(aframe[16], aframe[17])
  // }

  // pub open spec fn valid_output_arp_size(output: SW::SizedEthernetMessage_Impl) -> bool
  // {
  //   output.sz == 64u16
  // }

  // pub open spec fn valid_output_ipv4_size(
  //   input: SW::RawEthernetMessage,
  //   output: SW::SizedEthernetMessage_Impl) -> bool
  // {
  //   (input.len() == 1600) &&
  //     (output.sz == two_bytes_to_u16(input[16], input[17]) + 14u16)
  // }

  // pub open spec fn allow_outbound_frame(aframe: SW::RawEthernetMessage) -> bool
  // {
  //   valid_arp(aframe) || valid_ipv4(aframe)
  // }
  // // END MARKER GUMBO METHODS

}
