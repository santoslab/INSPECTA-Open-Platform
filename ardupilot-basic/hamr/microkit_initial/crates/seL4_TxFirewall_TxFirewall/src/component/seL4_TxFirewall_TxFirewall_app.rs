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
    log_info("compute entrypoint invoked");
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
