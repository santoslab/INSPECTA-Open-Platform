#![cfg_attr(not(test), no_std)]

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

#![allow(dead_code)]
#![allow(static_mut_refs)]
#![allow(unused_imports)]
#![allow(unused_macros)]
#![allow(unused_parens)]
#![allow(unused_unsafe)]
#![allow(unused_variables)]

// This file will not be overwritten if codegen is rerun

use data::*;
use vstd::prelude::*;

macro_rules! implies {
  ($lhs: expr, $rhs: expr) => {
    !$lhs || $rhs
  };
}

macro_rules! impliesL {
  ($lhs: expr, $rhs: expr) => {
    !$lhs | $rhs
  };
}

// BEGIN MARKER GUMBO RUST MARKER
pub fn TCP_ALLOWED_PORTS() -> SW::u16Array
{
  [5760u16]
}

pub fn UDP_ALLOWED_PORTS() -> SW::u16Array
{
  [68u16]
}

pub fn two_bytes_to_u16_le(
  byte0: u8,
  byte1: u8) -> u16
{
  ((byte1) as u16) * 256u16 + ((byte0) as u16)
}

pub fn two_bytes_to_u16_be(
  byte0: u8,
  byte1: u8) -> u16
{
  ((byte0) as u16) * 256u16 + ((byte1) as u16)
}

pub fn three_bytes_to_u32(
  byte0: u8,
  byte1: u8,
  byte2: u8) -> u32
{
  ((byte2) as u32) * 65536u32 + (((byte1) as u32) * 256u32 + ((byte0) as u32))
}

pub fn frame_is_wellformed_eth2(aframe: SW::RawEthernetMessage) -> bool
{
  valid_frame_ethertype(aframe) && valid_frame_dst_addr(aframe)
}

pub fn valid_frame_ethertype(aframe: SW::RawEthernetMessage) -> bool
{
  frame_has_ipv4(aframe) ||
    (frame_has_arp(aframe) || frame_has_ipv6(aframe))
}

pub fn valid_frame_dst_addr(aframe: SW::RawEthernetMessage) -> bool
{
  (aframe.len() == 1600) &&
    !((aframe[0] == 0u8) &&
      ((aframe[1] == 0u8) &&
        ((aframe[2] == 0u8) &&
          ((aframe[3] == 0u8) &&
            ((aframe[4] == 0u8) &&
              (aframe[5] == 0u8))))))
}

pub fn frame_has_ipv4(aframe: SW::RawEthernetMessage) -> bool
{
  (aframe.len() == 1600) &&
    ((aframe[12] == 8u8) &&
      (aframe[13] == 0u8))
}

pub fn frame_has_ipv6(aframe: SW::RawEthernetMessage) -> bool
{
  (aframe.len() == 1600) &&
    ((aframe[12] == 134u8) &&
      (aframe[13] == 221u8))
}

pub fn frame_has_arp(aframe: SW::RawEthernetMessage) -> bool
{
  (aframe.len() == 1600) &&
    ((aframe[12] == 8u8) &&
      (aframe[13] == 6u8))
}

pub fn arp_has_ipv4(aframe: SW::RawEthernetMessage) -> bool
{
  (aframe.len() == 1600) &&
    ((aframe[16] == 8u8) &&
      (aframe[17] == 0u8))
}

pub fn arp_has_ipv6(aframe: SW::RawEthernetMessage) -> bool
{
  (aframe.len() == 1600) &&
    ((aframe[16] == 134u8) &&
      (aframe[17] == 221u8))
}

pub fn valid_arp_ptype(aframe: SW::RawEthernetMessage) -> bool
{
  arp_has_ipv4(aframe) || arp_has_ipv6(aframe)
}

pub fn valid_arp_op(aframe: SW::RawEthernetMessage) -> bool
{
  (aframe.len() == 1600) &&
    ((aframe[20] == 0u8) &&
      ((aframe[21] == 1u8) ||
        (aframe[21] == 2u8)))
}

pub fn valid_arp_htype(aframe: SW::RawEthernetMessage) -> bool
{
  (aframe.len() == 1600) &&
    ((aframe[14] == 0u8) &&
      (aframe[15] == 1u8))
}

pub fn wellformed_arp_frame(aframe: SW::RawEthernetMessage) -> bool
{
  valid_arp_op(aframe) &&
    (valid_arp_htype(aframe) && valid_arp_ptype(aframe))
}

pub fn valid_ipv4_length(aframe: SW::RawEthernetMessage) -> bool
{
  (aframe.len() == 1600) &&
    (two_bytes_to_u16_be(aframe[16], aframe[17]) <= 9000u16)
}

pub fn valid_ipv4_protocol(aframe: SW::RawEthernetMessage) -> bool
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

pub fn valid_ipv4_vers_ihl(aframe: SW::RawEthernetMessage) -> bool
{
  (aframe.len() == 1600) &&
    (aframe[14] == 69u8)
}

pub fn wellformed_ipv4_frame(aframe: SW::RawEthernetMessage) -> bool
{
  valid_ipv4_protocol(aframe) &&
    (valid_ipv4_length(aframe) && valid_ipv4_vers_ihl(aframe))
}

pub fn valid_ipv6(aframe: SW::RawEthernetMessage) -> bool
{
  frame_is_wellformed_eth2(aframe) && frame_has_ipv6(aframe)
}

pub fn valid_arp(aframe: SW::RawEthernetMessage) -> bool
{
  frame_is_wellformed_eth2(aframe) &&
    (frame_has_arp(aframe) && wellformed_arp_frame(aframe))
}

pub fn valid_ipv4(aframe: SW::RawEthernetMessage) -> bool
{
  frame_is_wellformed_eth2(aframe) &&
    (frame_has_ipv4(aframe) && wellformed_ipv4_frame(aframe))
}

pub fn ipv4_length(aframe: SW::RawEthernetMessage) -> u16
{
  two_bytes_to_u16_le(aframe[16], aframe[17])
}

pub fn valid_output_arp_size(output: SW::SizedEthernetMessage_Impl) -> bool
{
  output.sz == 64u16
}

pub fn valid_output_ipv4_size(
  input: SW::RawEthernetMessage,
  output: SW::SizedEthernetMessage_Impl) -> bool
{
  (input.len() == 1600) &&
    (output.sz == two_bytes_to_u16_be(input[16], input[17]) + 14u16)
}

pub fn tx_allow_outbound_frame(aframe: SW::RawEthernetMessage) -> bool
{
  valid_arp(aframe) || valid_ipv4(aframe)
}

pub fn ipv4_is_tcp(aframe: SW::RawEthernetMessage) -> bool
{
  (aframe.len() == 1600) &&
    (aframe[23] == 6u8)
}

pub fn ipv4_is_udp(aframe: SW::RawEthernetMessage) -> bool
{
  (aframe.len() == 1600) &&
    (aframe[23] == 17u8)
}

pub fn tcp_is_valid_port(aframe: SW::RawEthernetMessage) -> bool
{
  (aframe.len() == 1600) &&
    (two_bytes_to_u16_be(aframe[36], aframe[37]) == TCP_ALLOWED_PORTS()[0])
}

pub fn udp_is_valid_port(aframe: SW::RawEthernetMessage) -> bool
{
  (aframe.len() == 1600) &&
    (two_bytes_to_u16_be(aframe[36], aframe[37]) == UDP_ALLOWED_PORTS()[0])
}

pub fn udp_is_mavlink_src_port(aframe: SW::RawEthernetMessage) -> bool
{
  (aframe.len() == 1600) &&
    (two_bytes_to_u16_be(aframe[34], aframe[35]) == 14550u16)
}

pub fn udp_is_mavlink_dst_port(aframe: SW::RawEthernetMessage) -> bool
{
  (aframe.len() == 1600) &&
    (two_bytes_to_u16_be(aframe[36], aframe[37]) == 14562u16)
}

pub fn udp_is_mavlink(aframe: SW::RawEthernetMessage) -> bool
{
  udp_is_mavlink_src_port(aframe) && udp_is_mavlink_dst_port(aframe)
}

pub fn frame_has_ipv4_tcp_on_allowed_port_quant(aframe: SW::RawEthernetMessage) -> bool
{
  (aframe.len() == 1600) &&
    (0..=TCP_ALLOWED_PORTS().len() - 1).any(|i| TCP_ALLOWED_PORTS()[i] == two_bytes_to_u16_be(aframe[36], aframe[37]))
}

pub fn udp_is_valid_direct_dst_port(aframe: SW::RawEthernetMessage) -> bool
{
  (aframe.len() == 1600) &&
    (0..=UDP_ALLOWED_PORTS().len() - 1).any(|i| UDP_ALLOWED_PORTS()[i] == two_bytes_to_u16_be(aframe[36], aframe[37]))
}

pub fn valid_ipv4_tcp(aframe: SW::RawEthernetMessage) -> bool
{
  frame_is_wellformed_eth2(aframe) &&
    (frame_has_ipv4(aframe) &&
      (wellformed_ipv4_frame(aframe) && ipv4_is_tcp(aframe)))
}

pub fn valid_ipv4_udp(aframe: SW::RawEthernetMessage) -> bool
{
  frame_is_wellformed_eth2(aframe) &&
    (frame_has_ipv4(aframe) &&
      (wellformed_ipv4_frame(aframe) && ipv4_is_udp(aframe)))
}

pub fn valid_ipv4_tcp_port(aframe: SW::RawEthernetMessage) -> bool
{
  valid_ipv4_tcp(aframe) && frame_has_ipv4_tcp_on_allowed_port_quant(aframe)
}

pub fn valid_ipv4_udp_port(aframe: SW::RawEthernetMessage) -> bool
{
  valid_ipv4_udp(aframe) &&
    (udp_is_valid_direct_dst_port(aframe) && !(udp_is_mavlink(aframe)))
}

pub fn valid_ipv4_udp_mavlink(aframe: SW::RawEthernetMessage) -> bool
{
  valid_ipv4_udp(aframe) && udp_is_mavlink(aframe)
}

pub fn rx_allow_outbound_frame(aframe: SW::RawEthernetMessage) -> bool
{
  valid_arp(aframe) ||
    (valid_ipv4_udp_mavlink(aframe) || valid_ipv4_udp_port(aframe))
}

pub fn input_eq_mav_output_headers(
  aframe: SW::RawEthernetMessage,
  headers: SW::EthIpUdpHeaders) -> bool
{
  (aframe.len() == 1600) &&
    (0..=headers.len() - 1).all(|i| headers[i] == aframe[i])
}

pub fn input_eq_mav_output_payload(
  aframe: SW::RawEthernetMessage,
  payload: SW::UdpPayload,
  headers: SW::EthIpUdpHeaders) -> bool
{
  (aframe.len() == 1600) &&
    (payload.len() == 1558) &&
    (0..=payload.len() - 1).all(|i| aframe[i + headers.len()] == payload[i])
}

pub fn input_eq_mav_output(
  aframe: SW::RawEthernetMessage,
  output: SW::UdpFrame_Impl) -> bool
{
  input_eq_mav_output_headers(aframe, output.headers) && input_eq_mav_output_payload(aframe, output.payload, output.headers)
}

pub fn mav_input_headers_eq_output(
  headers: SW::EthIpUdpHeaders,
  aframe: SW::RawEthernetMessage) -> bool
{
  (aframe.len() == 1600) &&
    (0..=headers.len() - 1).all(|i| headers[i] == aframe[i])
}

pub fn mav_input_payload_eq_output(
  payload: SW::UdpPayload,
  headers: SW::EthIpUdpHeaders,
  aframe: SW::RawEthernetMessage) -> bool
{
  (aframe.len() == 1600) &&
    (payload.len() == 1558) &&
    (0..=payload.len() - 1).all(|i| aframe[i + headers.len()] == payload[i])
}

pub fn mav_input_eq_output(
  input: SW::UdpFrame_Impl,
  aframe: SW::RawEthernetMessage) -> bool
{
  mav_input_headers_eq_output(input.headers, aframe) && mav_input_payload_eq_output(input.payload, input.headers, aframe)
}
// END MARKER GUMBO RUST MARKER

verus! {
  // BEGIN MARKER GUMBO VERUS MARKER
  pub open spec fn TCP_ALLOWED_PORTS_spec() -> SW::u16Array
  {
    [5760u16]
  }

  pub open spec fn UDP_ALLOWED_PORTS_spec() -> SW::u16Array
  {
    [68u16]
  }

  pub open spec fn two_bytes_to_u16_le_spec(
    byte0: u8,
    byte1: u8) -> u16
  {
    (((byte1) as u16) * 256u16 + ((byte0) as u16)) as u16
  }

  pub open spec fn two_bytes_to_u16_be_spec(
    byte0: u8,
    byte1: u8) -> u16
  {
    (((byte0) as u16) * 256u16 + ((byte1) as u16)) as u16
  }

  pub open spec fn three_bytes_to_u32_spec(
    byte0: u8,
    byte1: u8,
    byte2: u8) -> u32
  {
    (((byte2) as u32) * 65536u32 + (((byte1) as u32) * 256u32 + ((byte0) as u32))) as u32
  }

  pub open spec fn frame_is_wellformed_eth2_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    valid_frame_ethertype_spec(aframe) && valid_frame_dst_addr_spec(aframe)
  }

  pub open spec fn valid_frame_ethertype_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    frame_has_ipv4_spec(aframe) ||
      (frame_has_arp_spec(aframe) || frame_has_ipv6_spec(aframe))
  }

  pub open spec fn valid_frame_dst_addr_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      !((aframe[0] == 0u8) &&
        ((aframe[1] == 0u8) &&
          ((aframe[2] == 0u8) &&
            ((aframe[3] == 0u8) &&
              ((aframe[4] == 0u8) &&
                (aframe[5] == 0u8))))))
  }

  pub open spec fn frame_has_ipv4_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      ((aframe[12] == 8u8) &&
        (aframe[13] == 0u8))
  }

  pub open spec fn frame_has_ipv6_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      ((aframe[12] == 134u8) &&
        (aframe[13] == 221u8))
  }

  pub open spec fn frame_has_arp_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      ((aframe[12] == 8u8) &&
        (aframe[13] == 6u8))
  }

  pub open spec fn arp_has_ipv4_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      ((aframe[16] == 8u8) &&
        (aframe[17] == 0u8))
  }

  pub open spec fn arp_has_ipv6_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      ((aframe[16] == 134u8) &&
        (aframe[17] == 221u8))
  }

  pub open spec fn valid_arp_ptype_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    arp_has_ipv4_spec(aframe) || arp_has_ipv6_spec(aframe)
  }

  pub open spec fn valid_arp_op_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      ((aframe[20] == 0u8) &&
        ((aframe[21] == 1u8) ||
          (aframe[21] == 2u8)))
  }

  pub open spec fn valid_arp_htype_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      ((aframe[14] == 0u8) &&
        (aframe[15] == 1u8))
  }

  pub open spec fn wellformed_arp_frame_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    valid_arp_op_spec(aframe) &&
      (valid_arp_htype_spec(aframe) && valid_arp_ptype_spec(aframe))
  }

  pub open spec fn valid_ipv4_length_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      (two_bytes_to_u16_be_spec(aframe[16], aframe[17]) <= 9000u16)
  }

  pub open spec fn valid_ipv4_protocol_spec(aframe: SW::RawEthernetMessage) -> bool
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

  pub open spec fn valid_ipv4_vers_ihl_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      (aframe[14] == 69u8)
  }

  pub open spec fn wellformed_ipv4_frame_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    valid_ipv4_protocol_spec(aframe) &&
      (valid_ipv4_length_spec(aframe) && valid_ipv4_vers_ihl_spec(aframe))
  }

  pub open spec fn valid_ipv6_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    frame_is_wellformed_eth2_spec(aframe) && frame_has_ipv6_spec(aframe)
  }

  pub open spec fn valid_arp_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    frame_is_wellformed_eth2_spec(aframe) &&
      (frame_has_arp_spec(aframe) && wellformed_arp_frame_spec(aframe))
  }

  pub open spec fn valid_ipv4_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    frame_is_wellformed_eth2_spec(aframe) &&
      (frame_has_ipv4_spec(aframe) && wellformed_ipv4_frame_spec(aframe))
  }

  pub open spec fn ipv4_length_spec(aframe: SW::RawEthernetMessage) -> u16
  {
    two_bytes_to_u16_le_spec(aframe[16], aframe[17])
  }

  pub open spec fn valid_output_arp_size_spec(output: SW::SizedEthernetMessage_Impl) -> bool
  {
    output.sz == 64u16
  }

  pub open spec fn valid_output_ipv4_size_spec(
    input: SW::RawEthernetMessage,
    output: SW::SizedEthernetMessage_Impl) -> bool
  {
    (input.len() == 1600) &&
      (output.sz == two_bytes_to_u16_be_spec(input[16], input[17]) + 14u16)
  }

  pub open spec fn tx_allow_outbound_frame_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    valid_arp_spec(aframe) || valid_ipv4_spec(aframe)
  }

  pub open spec fn ipv4_is_tcp_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      (aframe[23] == 6u8)
  }

  pub open spec fn ipv4_is_udp_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      (aframe[23] == 17u8)
  }

  pub open spec fn tcp_is_valid_port_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      (two_bytes_to_u16_be_spec(aframe[36], aframe[37]) == TCP_ALLOWED_PORTS_spec()[0])
  }

  pub open spec fn udp_is_valid_port_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      (two_bytes_to_u16_be_spec(aframe[36], aframe[37]) == UDP_ALLOWED_PORTS_spec()[0])
  }

  pub open spec fn udp_is_mavlink_src_port_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      (two_bytes_to_u16_be_spec(aframe[34], aframe[35]) == 14550u16)
  }

  pub open spec fn udp_is_mavlink_dst_port_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      (two_bytes_to_u16_be_spec(aframe[36], aframe[37]) == 14562u16)
  }

  pub open spec fn udp_is_mavlink_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    udp_is_mavlink_src_port_spec(aframe) && udp_is_mavlink_dst_port_spec(aframe)
  }

  pub open spec fn frame_has_ipv4_tcp_on_allowed_port_quant_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      exists|i:int| 0 <= i <= TCP_ALLOWED_PORTS_spec().len() - 1 && #[trigger] TCP_ALLOWED_PORTS_spec()[i] == two_bytes_to_u16_be_spec(aframe[36], aframe[37])
  }

  pub open spec fn udp_is_valid_direct_dst_port_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      exists|i:int| 0 <= i <= UDP_ALLOWED_PORTS_spec().len() - 1 && #[trigger] UDP_ALLOWED_PORTS_spec()[i] == two_bytes_to_u16_be_spec(aframe[36], aframe[37])
  }

  pub open spec fn valid_ipv4_tcp_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    frame_is_wellformed_eth2_spec(aframe) &&
      (frame_has_ipv4_spec(aframe) &&
        (wellformed_ipv4_frame_spec(aframe) && ipv4_is_tcp_spec(aframe)))
  }

  pub open spec fn valid_ipv4_udp_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    frame_is_wellformed_eth2_spec(aframe) &&
      (frame_has_ipv4_spec(aframe) &&
        (wellformed_ipv4_frame_spec(aframe) && ipv4_is_udp_spec(aframe)))
  }

  pub open spec fn valid_ipv4_tcp_port_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    valid_ipv4_tcp_spec(aframe) && frame_has_ipv4_tcp_on_allowed_port_quant_spec(aframe)
  }

  pub open spec fn valid_ipv4_udp_port_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    valid_ipv4_udp_spec(aframe) &&
      (udp_is_valid_direct_dst_port_spec(aframe) && !(udp_is_mavlink_spec(aframe)))
  }

  pub open spec fn valid_ipv4_udp_mavlink_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    valid_ipv4_udp_spec(aframe) && udp_is_mavlink_spec(aframe)
  }

  pub open spec fn rx_allow_outbound_frame_spec(aframe: SW::RawEthernetMessage) -> bool
  {
    valid_arp_spec(aframe) ||
      (valid_ipv4_udp_mavlink_spec(aframe) || valid_ipv4_udp_port_spec(aframe))
  }

  pub open spec fn input_eq_mav_output_headers_spec(
    aframe: SW::RawEthernetMessage,
    headers: SW::EthIpUdpHeaders) -> bool
  {
    (aframe.len() == 1600) &&
      forall|i:int| 0 <= i <= headers.len() - 1 ==> #[trigger] headers[i] == aframe[i]
  }

  pub open spec fn input_eq_mav_output_payload_spec(
    aframe: SW::RawEthernetMessage,
    payload: SW::UdpPayload,
    headers: SW::EthIpUdpHeaders) -> bool
  {
    (aframe.len() == 1600) &&
      (payload.len() == 1558) &&
      forall|i:int| 0 <= i <= payload.len() - 1 ==> #[trigger] aframe[i + headers.len()] == payload[i]
  }

  pub open spec fn input_eq_mav_output_spec(
    aframe: SW::RawEthernetMessage,
    output: SW::UdpFrame_Impl) -> bool
  {
    input_eq_mav_output_headers_spec(aframe, output.headers) && input_eq_mav_output_payload_spec(aframe, output.payload, output.headers)
  }

  pub open spec fn mav_input_headers_eq_output_spec(
    headers: SW::EthIpUdpHeaders,
    aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      forall|i:int| 0 <= i <= headers.len() - 1 ==> #[trigger] headers[i] == aframe[i]
  }

  pub open spec fn mav_input_payload_eq_output_spec(
    payload: SW::UdpPayload,
    headers: SW::EthIpUdpHeaders,
    aframe: SW::RawEthernetMessage) -> bool
  {
    (aframe.len() == 1600) &&
      (payload.len() == 1558) &&
      forall|i:int| 0 <= i <= payload.len() - 1 ==> #[trigger] aframe[i + headers.len()] == payload[i]
  }

  pub open spec fn mav_input_eq_output_spec(
    input: SW::UdpFrame_Impl,
    aframe: SW::RawEthernetMessage) -> bool
  {
    mav_input_headers_eq_output_spec(input.headers, aframe) && mav_input_payload_eq_output_spec(input.payload, input.headers, aframe)
  }
  // END MARKER GUMBO VERUS MARKER
}
