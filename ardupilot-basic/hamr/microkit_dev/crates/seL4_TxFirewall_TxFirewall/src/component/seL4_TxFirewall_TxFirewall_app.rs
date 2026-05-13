// This file will not be overwritten if HAMR codegen is rerun

use data::*;
use crate::bridge::seL4_TxFirewall_TxFirewall_api::*;
use vstd::prelude::*;

#[verus_verify]
pub struct seL4_TxFirewall_TxFirewall {
  // PLACEHOLDER MARKER STATE VARS
}

#[verus_verify]
impl seL4_TxFirewall_TxFirewall {
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
  pub fn initialize<API: seL4_TxFirewall_TxFirewall_Put_Api> (
    &mut self,
    api: &mut seL4_TxFirewall_TxFirewall_Application_Api<API>)
  {
    log_info("initialize entrypoint invoked");
  }

  #[verus_spec(
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
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=5
      api.EthernetFramesTxIn0.is_some() && GumboLib::valid_arp_spec(api.EthernetFramesTxIn0.unwrap()) ==>
        api.EthernetFramesTxOut0.is_some() &&
          ((api.EthernetFramesTxIn0.unwrap() == api.EthernetFramesTxOut0.unwrap().amessage) &&
            GumboLib::valid_output_arp_size_spec(api.EthernetFramesTxOut0.unwrap())),
      // guarantee hlr_12_tx0_can_send_valid_ipv4
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=5
      api.EthernetFramesTxIn0.is_some() && GumboLib::valid_ipv4_spec(api.EthernetFramesTxIn0.unwrap()) ==>
        api.EthernetFramesTxOut0.is_some() &&
          ((api.EthernetFramesTxIn0.unwrap() == api.EthernetFramesTxOut0.unwrap().amessage) &&
            GumboLib::valid_output_ipv4_size_spec(api.EthernetFramesTxIn0.unwrap(), api.EthernetFramesTxOut0.unwrap())),
      // guarantee hlr_14_tx0_disallow
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=6
      api.EthernetFramesTxIn0.is_some() && !(GumboLib::tx_allow_outbound_frame_spec(api.EthernetFramesTxIn0.unwrap())) ==>
        api.EthernetFramesTxOut0.is_none(),
      // guarantee hlr_16_tx0_no_input
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=6
      api.EthernetFramesTxIn0.is_some() || api.EthernetFramesTxOut0.is_none(),
      // guarantee hlr_07_tx1_can_send_valid_arp
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=5
      api.EthernetFramesTxIn1.is_some() && GumboLib::valid_arp_spec(api.EthernetFramesTxIn1.unwrap()) ==>
        api.EthernetFramesTxOut1.is_some() &&
          ((api.EthernetFramesTxIn1.unwrap() == api.EthernetFramesTxOut1.unwrap().amessage) &&
            GumboLib::valid_output_arp_size_spec(api.EthernetFramesTxOut1.unwrap())),
      // guarantee hlr_12_tx1_can_send_valid_ipv4
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=5
      api.EthernetFramesTxIn1.is_some() && GumboLib::valid_ipv4_spec(api.EthernetFramesTxIn1.unwrap()) ==>
        api.EthernetFramesTxOut1.is_some() &&
          ((api.EthernetFramesTxIn1.unwrap() == api.EthernetFramesTxOut1.unwrap().amessage) &&
            GumboLib::valid_output_ipv4_size_spec(api.EthernetFramesTxIn1.unwrap(), api.EthernetFramesTxOut1.unwrap())),
      // guarantee hlr_14_tx1_disallow
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=6
      api.EthernetFramesTxIn1.is_some() && !(GumboLib::tx_allow_outbound_frame_spec(api.EthernetFramesTxIn1.unwrap())) ==>
        api.EthernetFramesTxOut1.is_none(),
      // guarantee hlr_16_tx1_no_input
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=6
      api.EthernetFramesTxIn1.is_some() || api.EthernetFramesTxOut1.is_none(),
      // guarantee hlr_07_tx2_can_send_valid_arp
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=5
      api.EthernetFramesTxIn2.is_some() && GumboLib::valid_arp_spec(api.EthernetFramesTxIn2.unwrap()) ==>
        api.EthernetFramesTxOut2.is_some() &&
          ((api.EthernetFramesTxIn2.unwrap() == api.EthernetFramesTxOut2.unwrap().amessage) &&
            GumboLib::valid_output_arp_size_spec(api.EthernetFramesTxOut2.unwrap())),
      // guarantee hlr_12_tx2_can_send_valid_ipv4
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=5
      api.EthernetFramesTxIn2.is_some() && GumboLib::valid_ipv4_spec(api.EthernetFramesTxIn2.unwrap()) ==>
        api.EthernetFramesTxOut2.is_some() &&
          ((api.EthernetFramesTxIn2.unwrap() == api.EthernetFramesTxOut2.unwrap().amessage) &&
            GumboLib::valid_output_ipv4_size_spec(api.EthernetFramesTxIn2.unwrap(), api.EthernetFramesTxOut2.unwrap())),
      // guarantee hlr_14_tx2_disallow
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=6
      api.EthernetFramesTxIn2.is_some() && !(GumboLib::tx_allow_outbound_frame_spec(api.EthernetFramesTxIn2.unwrap())) ==>
        api.EthernetFramesTxOut2.is_none(),
      // guarantee hlr_16_tx2_no_input
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=6
      api.EthernetFramesTxIn2.is_some() || api.EthernetFramesTxOut2.is_none(),
      // guarantee hlr_07_tx3_can_send_valid_arp
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=5
      api.EthernetFramesTxIn3.is_some() && GumboLib::valid_arp_spec(api.EthernetFramesTxIn3.unwrap()) ==>
        api.EthernetFramesTxOut3.is_some() &&
          ((api.EthernetFramesTxIn3.unwrap() == api.EthernetFramesTxOut3.unwrap().amessage) &&
            GumboLib::valid_output_arp_size_spec(api.EthernetFramesTxOut3.unwrap())),
      // guarantee hlr_12_tx3_can_send_valid_ipv4
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=5
      api.EthernetFramesTxIn3.is_some() && GumboLib::valid_ipv4_spec(api.EthernetFramesTxIn3.unwrap()) ==>
        api.EthernetFramesTxOut3.is_some() &&
          ((api.EthernetFramesTxIn3.unwrap() == api.EthernetFramesTxOut3.unwrap().amessage) &&
            GumboLib::valid_output_ipv4_size_spec(api.EthernetFramesTxIn3.unwrap(), api.EthernetFramesTxOut3.unwrap())),
      // guarantee hlr_14_tx3_disallow
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=6
      api.EthernetFramesTxIn3.is_some() && !(GumboLib::tx_allow_outbound_frame_spec(api.EthernetFramesTxIn3.unwrap())) ==>
        api.EthernetFramesTxOut3.is_none(),
      // guarantee hlr_16_tx3_no_input
      //   https://loonwerks.com/INSPECTA-Open-Platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=6
      api.EthernetFramesTxIn3.is_some() || api.EthernetFramesTxOut3.is_none(),
      // END MARKER TIME TRIGGERED ENSURES
  )]
  pub fn timeTriggered<API: seL4_TxFirewall_TxFirewall_Full_Api> (
    &mut self,
    api: &mut seL4_TxFirewall_TxFirewall_Application_Api<API>)
  {
    //log_info("compute entrypoint invoked");
      // Tx0 ports
      if let Some(frame) = api.get_EthernetFramesTxIn0() {
          if let Some(eth) = get_frame_packet(&frame) {
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
            if let Some(eth) = get_frame_packet(&frame) {
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
            if let Some(eth) = get_frame_packet(&frame) {
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
            if let Some(eth) = get_frame_packet(&frame) {
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

/// Parses a raw Ethernet frame into a structured `EthFrame` via `firewall_core`. The
/// ensures clauses bridge the GUMBO spec-level predicates (`GumboLib::valid_arp_spec`,
/// `GumboLib::valid_ipv4_spec`) — which operate on raw byte arrays — to the firewall_core
/// result predicates (`res_is_arp`, `res_is_ipv4`), which operate on the parsed structure.
/// This lets Verus connect the raw-frame GUMBO contracts on `timeTriggered` to the
/// structured packet decisions made by `can_send_packet`.
#[verus_verify]
#[verus_spec(r =>
    requires
        frame@.len() == SW::SW_RawEthernetMessage_DIM_0,
    ensures
        GumboLib::valid_arp_spec(*frame) == firewall_core::res_is_arp(r),
        GumboLib::valid_ipv4_spec(*frame) == firewall_core::res_is_ipv4(r),
        GumboLib::valid_ipv4_spec(*frame) ==> firewall_core::ipv4_length_bytes_match(frame, r),
)]
fn get_frame_packet(frame: &SW::RawEthernetMessage) -> Option<firewall_core::EthFrame>
{
    let eth = firewall_core::EthFrame::parse(frame);
    if eth.is_none() {
        log_info("Malformed packet. Throw it away.")
    }
    eth
}

/// Determines whether a parsed packet should be forwarded and computes the output
/// frame size. Returns `Some(size)` for ARP (fixed 64 bytes) and IPv4 (header length +
/// Ethernet header), `None` for IPv6. The ensures clauses let Verus propagate the size
/// relationship back to the GUMBO output-size predicates (`valid_output_arp_size_spec`,
/// `valid_output_ipv4_size_spec`) used in the hlr_07/hlr_12 guarantees.
#[verus_verify]
#[verus_spec(r =>
    requires
        (packet is Ipv4) ==> (firewall_core::ipv4_valid_length(*packet)),
    ensures
        (packet is Arp || packet is Ipv4) == r.is_some(),
        packet is Arp ==> (r == Some(64u16)),
        packet is Ipv4 ==> (r == Some((packet->Ipv4_0.header.length + firewall_core::EthernetRepr::SIZE) as u16)),
)]
fn can_send_packet(packet: &firewall_core::PacketType) -> Option<u16>
{
    match packet {
        firewall_core::PacketType::Arp(_) => Some(64u16),
        firewall_core::PacketType::Ipv4(ip) => Some(ip.header.length + firewall_core::EthernetRepr::SIZE as u16),
        firewall_core::PacketType::Ipv6 => {
            log_info("IPv6 packet: Throw it away.");
            None
        }
    }
}
