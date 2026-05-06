#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

// This file will not be overwritten if codegen is rerun

use crate::bridge::seL4_LowLevelEthernetDriver_LowLevelEthernetDriver_api::*;
use data::*;
use vstd::prelude::*;
// use vstd::slice::slice_subrange;
#[cfg(feature = "sel4")]
#[allow(unused_imports)]
use log::{trace, info, debug};

use crate::microkit_channel;
use crate::SW;


use sel4_driver_interfaces::HandleInterrupt;
use sel4_microkit_base::memory_region_symbol;
use smoltcp::{
    phy::{Device, RxToken, TxToken},
    time::Instant,
};

use eth_driver_core::{DmaDef, Driver};

mod config;

// verus! {
  const NUM_MSGS: usize = 4;

  fn get_tx<API: seL4_LowLevelEthernetDriver_LowLevelEthernetDriver_Get_Api>(
      idx: usize,
      api: &mut seL4_LowLevelEthernetDriver_LowLevelEthernetDriver_Application_Api<API>,
  ) -> Option<SW::SizedEthernetMessage_Impl> {
      match idx {
          0 => api.get_EthernetFramesTx0(),
          1 => api.get_EthernetFramesTx1(),
          2 => api.get_EthernetFramesTx2(),
          3 => api.get_EthernetFramesTx3(),
          _ => None,
      }
  }

  fn put_rx<API: seL4_LowLevelEthernetDriver_LowLevelEthernetDriver_Put_Api>(
      idx: usize,
      rx_buf: &[u8],
      api: &mut seL4_LowLevelEthernetDriver_LowLevelEthernetDriver_Application_Api<API>,
  ) {
      // let value: SW::RawEthernetMessage = slice_subrange(rx_buf, 0, SW::SW_RawEthernetMessage_DIM_0)
      let value: SW::RawEthernetMessage = rx_buf[0..SW::SW_RawEthernetMessage_DIM_0]
          .try_into()
          .unwrap();
      match idx {
          0 => api.put_EthernetFramesRx0(value),
          1 => api.put_EthernetFramesRx1(value),
          2 => api.put_EthernetFramesRx2(value),
          3 => api.put_EthernetFramesRx3(value),
          _ => (),
      }
  }


  pub struct seL4_LowLevelEthernetDriver_LowLevelEthernetDriver {
    drv: Driver,
  }

  impl seL4_LowLevelEthernetDriver_LowLevelEthernetDriver {
    pub fn new() -> Self
    {
        let dev = {
            let dma = DmaDef {
                vaddr: memory_region_symbol!(net_driver_dma_vaddr: *mut ()),
                paddr: memory_region_symbol!(net_driver_dma_paddr: *mut ()),
                size: config::sizes::DRIVER_DMA,
            };
            Driver::new(
                memory_region_symbol!(gem_register_block: *mut ()).as_ptr(),
                dma,
            )
        };
        Self {
            drv: dev,
        }
    }

    pub fn initialize<API: seL4_LowLevelEthernetDriver_LowLevelEthernetDriver_Put_Api> (
      &mut self,
      api: &mut seL4_LowLevelEthernetDriver_LowLevelEthernetDriver_Application_Api<API>)
    {
      #[cfg(feature = "sel4")]
      log_info("initialize entrypoint invoked");
      self.drv.handle_interrupt();
      info!("Acked driver IRQ");
      
    }

    pub fn timeTriggered<API: seL4_LowLevelEthernetDriver_LowLevelEthernetDriver_Full_Api> (
      &mut self,
      api: &mut seL4_LowLevelEthernetDriver_LowLevelEthernetDriver_Application_Api<API>)
    {
      #[cfg(feature = "sel4")]
        trace!("compute entrypoint invoked");
        let tmp: SW::RawEthernetMessage = [0; SW::SW_RawEthernetMessage_DIM_0];

        for i in 0..NUM_MSGS {
            if let Some((rx_tok, _tx_tok)) = self.drv.receive(Instant::ZERO) {
                rx_tok.consume(|rx_buf| {
                    debug!("RX Packet: {:?}", &rx_buf[0..64]);
                    put_rx(i, rx_buf, api);
                });
            }
        }

        for i in 0..NUM_MSGS {
            if let Some(sz_pkt) = get_tx(i, api) {
                let size = sz_pkt.sz as usize;
                if size > 0 {
                    // warn!("TX Packet: {:0>2X?}", &sz_pkt.message[0..size]);
                    debug!("TX Packet");
                    if let Some(tx_tok) = self.drv.transmit(Instant::ZERO) {
                        trace!("Valid tx token");
                        tx_tok.consume(size, |tx_buf| {
                            tx_buf.copy_from_slice(&sz_pkt.amessage[0..size]);
                            trace!("Copied from tmp to tx_buf");
                        });
                    };
                }
            }
        }

        self.drv.handle_interrupt();
    }

    pub fn notify(
      &mut self,
      channel: microkit_channel)
    {
      // this method is called when the monitor does not handle the passed in channel
      match channel {
        _ => {
          #[cfg(feature = "sel4")]
          log_warn_channel(channel)
        }
      }
    }
  }

verus! {
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
  pub open spec fn TCP_ALLOWED_PORTS() -> SW::u16Array
  {
    [5760u16]
  }

  pub open spec fn UDP_ALLOWED_PORTS() -> SW::u16Array
  {
    [68u16]
  }

  pub open spec fn two_bytes_to_u16(
    byte0: u8,
    byte1: u8) -> u16
  {
    (((byte0) as u16) * 256u16 + ((byte1) as u16)) as u16
  }

  pub open spec fn frame_is_wellformed_eth2(aframe: SW::RawEthernetMessage) -> bool
  {
    valid_frame_ethertype(aframe) && valid_frame_dst_addr(aframe)
  }

  pub open spec fn valid_frame_ethertype(aframe: SW::RawEthernetMessage) -> bool
  {
    frame_has_ipv4(aframe) ||
      (frame_has_arp(aframe) || frame_has_ipv6(aframe))
  }

  pub open spec fn valid_frame_dst_addr(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      !((aframe[0] == 0u8) &&
        ((aframe[1] == 0u8) &&
          ((aframe[2] == 0u8) &&
            ((aframe[3] == 0u8) &&
              ((aframe[4] == 0u8) &&
                (aframe[5] == 0u8))))))
  }

  pub open spec fn frame_has_ipv4(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      ((aframe[12] == 8u8) &&
        (aframe[13] == 0u8))
  }

  pub open spec fn frame_has_ipv6(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      ((aframe[12] == 134u8) &&
        (aframe[13] == 221u8))
  }

  pub open spec fn frame_has_arp(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      ((aframe[12] == 8u8) &&
        (aframe[13] == 6u8))
  }

  pub open spec fn arp_has_ipv4(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      ((aframe[16] == 8u8) &&
        (aframe[17] == 0u8))
  }

  pub open spec fn arp_has_ipv6(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      ((aframe[16] == 134u8) &&
        (aframe[17] == 221u8))
  }

  pub open spec fn valid_arp_ptype(aframe: SW::RawEthernetMessage) -> bool
  {
    arp_has_ipv4(aframe) || arp_has_ipv6(aframe)
  }

  pub open spec fn valid_arp_op(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      ((aframe[20] == 0u8) &&
        ((aframe[21] == 1u8) ||
          (aframe[21] == 2u8)))
  }

  pub open spec fn valid_arp_htype(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      ((aframe[14] == 0u8) &&
        (aframe[15] == 1u8))
  }

  pub open spec fn wellformed_arp_frame(aframe: SW::RawEthernetMessage) -> bool
  {
    valid_arp_op(aframe) &&
      (valid_arp_htype(aframe) && valid_arp_ptype(aframe))
  }

  pub open spec fn valid_ipv4_length(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      (two_bytes_to_u16(aframe[16], aframe[17]) <= 9000u16)
  }

  pub open spec fn valid_ipv4_protocol(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      ((aframe[23] == 0u8) ||
        ((aframe[23] == 1u8) ||
          ((aframe[23] == 2u8) ||
            ((aframe[23] == 6u8) ||
              ((aframe[23] == 17u8) ||
                ((aframe[23] == 43u8) ||
                  ((aframe[23] == 44u8) ||
                    ((aframe[23] == 58u8) ||
                      ((aframe[23] == 59u8) ||
                        (aframe[23] == 60u8))))))))))
  }

  pub open spec fn valid_ipv4_vers_ihl(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      (aframe[14] == 69u8)
  }

  pub open spec fn wellformed_ipv4_frame(aframe: SW::RawEthernetMessage) -> bool
  {
    valid_ipv4_protocol(aframe) &&
      (valid_ipv4_length(aframe) && valid_ipv4_vers_ihl(aframe))
  }

  pub open spec fn valid_ipv6(aframe: SW::RawEthernetMessage) -> bool
  {
    frame_is_wellformed_eth2(aframe) && frame_has_ipv6(aframe)
  }

  pub open spec fn valid_arp(aframe: SW::RawEthernetMessage) -> bool
  {
    frame_is_wellformed_eth2(aframe) &&
      (frame_has_arp(aframe) && wellformed_arp_frame(aframe))
  }

  pub open spec fn valid_ipv4(aframe: SW::RawEthernetMessage) -> bool
  {
    frame_is_wellformed_eth2(aframe) &&
      (frame_has_ipv4(aframe) && wellformed_ipv4_frame(aframe))
  }

  pub open spec fn ipv4_length(aframe: SW::RawEthernetMessage) -> u16
  {
    two_bytes_to_u16(aframe[16], aframe[17])
  }

  pub open spec fn valid_output_arp_size(output: SW::SizedEthernetMessage_Impl) -> bool
  {
    output.sz == 64u16
  }

  pub open spec fn valid_output_ipv4_size(
    input: SW::RawEthernetMessage,
    output: SW::SizedEthernetMessage_Impl) -> bool
  {
    (input.len() == 1600) &&
      (output.sz == two_bytes_to_u16(input[16], input[17]) + 14u16)
  }

  pub open spec fn tx_allow_outbound_frame(aframe: SW::RawEthernetMessage) -> bool
  {
    valid_arp(aframe) || valid_ipv4(aframe)
  }

  pub open spec fn ipv4_is_tcp(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      (aframe[23] == 6u8)
  }

  pub open spec fn ipv4_is_udp(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      (aframe[23] == 17u8)
  }

  pub open spec fn tcp_is_valid_port(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      (two_bytes_to_u16(aframe[36], aframe[37]) == TCP_ALLOWED_PORTS()[0])
  }

  pub open spec fn udp_is_valid_port(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      (two_bytes_to_u16(aframe[36], aframe[37]) == UDP_ALLOWED_PORTS()[0])
  }

  pub open spec fn udp_is_mavlink_src_port(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      (two_bytes_to_u16(aframe[34], aframe[35]) == 14550u16)
  }

  pub open spec fn udp_is_mavlink_dst_port(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      (two_bytes_to_u16(aframe[36], aframe[37]) == 14562u16)
  }

  pub open spec fn udp_is_mavlink(aframe: SW::RawEthernetMessage) -> bool
  {
    udp_is_mavlink_src_port(aframe) && udp_is_mavlink_dst_port(aframe)
  }

  pub open spec fn frame_has_ipv4_tcp_on_allowed_port_quant(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      exists|i:int| 0 <= i <= TCP_ALLOWED_PORTS().len() - 1 && #[trigger] TCP_ALLOWED_PORTS()[i] == two_bytes_to_u16(aframe[36], aframe[37])
  }

  pub open spec fn udp_is_valid_direct_dst_port(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      exists|i:int| 0 <= i <= UDP_ALLOWED_PORTS().len() - 1 && #[trigger] UDP_ALLOWED_PORTS()[i] == two_bytes_to_u16(aframe[36], aframe[37])
  }

  pub open spec fn valid_ipv4_tcp(aframe: SW::RawEthernetMessage) -> bool
  {
    frame_is_wellformed_eth2(aframe) &&
      (frame_has_ipv4(aframe) &&
        (wellformed_ipv4_frame(aframe) && ipv4_is_tcp(aframe)))
  }

  pub open spec fn valid_ipv4_udp(aframe: SW::RawEthernetMessage) -> bool
  {
    frame_is_wellformed_eth2(aframe) &&
      (frame_has_ipv4(aframe) &&
        (wellformed_ipv4_frame(aframe) && ipv4_is_udp(aframe)))
  }

  pub open spec fn valid_ipv4_tcp_port(aframe: SW::RawEthernetMessage) -> bool
  {
    valid_ipv4_tcp(aframe) && frame_has_ipv4_tcp_on_allowed_port_quant(aframe)
  }

  pub open spec fn valid_ipv4_udp_port(aframe: SW::RawEthernetMessage) -> bool
  {
    valid_ipv4_udp(aframe) &&
      (udp_is_valid_direct_dst_port(aframe) && !(udp_is_mavlink(aframe)))
  }

  pub open spec fn valid_ipv4_udp_mavlink(aframe: SW::RawEthernetMessage) -> bool
  {
    valid_ipv4_udp(aframe) && udp_is_mavlink(aframe)
  }

  pub open spec fn rx_allow_outbound_frame(aframe: SW::RawEthernetMessage) -> bool
  {
    valid_arp(aframe) ||
      (valid_ipv4_udp_mavlink(aframe) || valid_ipv4_udp_port(aframe))
  }

  pub open spec fn input_eq_mav_output_headers(
    aframe: SW::RawEthernetMessage,
    headers: SW::EthIpUdpHeaders) -> bool
  {
    (aframe.len() == 1600) &&
      forall|i:int| 0 <= i <= headers.len() - 1 ==> #[trigger] headers[i] == aframe[i]
  }

  pub open spec fn input_eq_mav_output_payload(
    aframe: SW::RawEthernetMessage,
    payload: SW::UdpPayload,
    headers: SW::EthIpUdpHeaders) -> bool
  {
    (aframe.len() == 1600) &&
      ((payload.len() == 1558) &&
        forall|i:int| 0 <= i <= payload.len() - 1 ==> #[trigger] aframe[i + headers.len()] == payload[i])
  }

  pub open spec fn input_eq_mav_output(
    aframe: SW::RawEthernetMessage,
    output: SW::UdpFrame_Impl) -> bool
  {
    input_eq_mav_output_headers(aframe, output.headers) && input_eq_mav_output_payload(aframe, output.payload, output.headers)
  }
  // END MARKER GUMBO METHODS

}

