use serial_test::serial;
use proptest::prelude::*;

use crate::test::util::*;
use crate::testComputeCB_macro;
use data::SW;

const NUM_CASES: u32 = 100;
const REJECT_RATIO: u32 = 5;
const VERBOSITY: u32 = 2;

fn random_frame() -> impl Strategy<Value = SW::RawEthernetMessage> {
  generators::SW_RawEthernetMessage_strategy_default()
}

// Valid ARP frame: non-zero dst MAC, EtherType 0x0806,
// HardwareType=Ethernet(0x0001), ptype=IPv4(0x0800), ArpOp=Request(0x0001)
fn arp_frame_strategy() -> impl Strategy<Value = SW::RawEthernetMessage> {
  random_frame().prop_map(|mut frame| {
    frame[0] = 0xff;  frame[1] = 0xff;  frame[2] = 0xff;
    frame[3] = 0xff;  frame[4] = 0xff;  frame[5] = 0xff;
    frame[12] = 0x08; frame[13] = 0x06;
    frame[14] = 0x00; frame[15] = 0x01; // HardwareType = Ethernet
    frame[16] = 0x08; frame[17] = 0x00; // ptype = IPv4 (not ARP)
    frame[20] = 0x00; frame[21] = 0x01; // ArpOp = Request
    frame
  })
}

// Valid MAVLink UDP frame: IPv4 + UDP + src_port=14550 + dst_port=14562
fn mavlink_udp_frame_strategy() -> impl Strategy<Value = SW::RawEthernetMessage> {
  random_frame().prop_map(|mut frame| {
    frame[0] = 0x02; frame[1] = 0x00; frame[2] = 0x00;
    frame[3] = 0x00; frame[4] = 0x00; frame[5] = 0x01;
    frame[12] = 0x08; frame[13] = 0x00; // IPv4
    frame[14] = 0x45;                    // version + IHL
    frame[16] = 0x00; frame[17] = 0x3c; // length 60 (<=9000)
    frame[23] = 0x11;                    // UDP
    frame[34] = 0x38; frame[35] = 0xD6; // src 14550
    frame[36] = 0x38; frame[37] = 0xE2; // dst 14562
    frame
  })
}

// Valid whitelisted UDP frame: IPv4 + UDP + dst_port=68 (DHCP), src != 14550
fn whitelisted_udp_frame_strategy() -> impl Strategy<Value = SW::RawEthernetMessage> {
  random_frame().prop_map(|mut frame| {
    frame[0] = 0x02; frame[1] = 0x00; frame[2] = 0x00;
    frame[3] = 0x00; frame[4] = 0x00; frame[5] = 0x01;
    frame[12] = 0x08; frame[13] = 0x00; // IPv4
    frame[14] = 0x45;
    frame[16] = 0x00; frame[17] = 0x3c;
    frame[23] = 0x11;                    // UDP
    frame[34] = 0x00; frame[35] = 0x43; // src 67 (DHCP server)
    frame[36] = 0x00; frame[37] = 0x44; // dst 68 (DHCP client)
    frame
  })
}

// Disallowed frames: parse successfully but get dropped by the routing logic.
// Mix of IPv6, IPv4 TCP, and IPv4 UDP on non-whitelisted/non-mavlink ports.
fn disallowed_frame_strategy() -> impl Strategy<Value = SW::RawEthernetMessage> {
  prop_oneof![
    // IPv6
    random_frame().prop_map(|mut frame| {
      frame[0] = 0x02; frame[1] = 0x00; frame[2] = 0x00;
      frame[3] = 0x00; frame[4] = 0x00; frame[5] = 0x01;
      frame[12] = 0x86; frame[13] = 0xDD;
      frame
    }),
    // IPv4 TCP (port 80)
    random_frame().prop_map(|mut frame| {
      frame[0] = 0x02; frame[1] = 0x00; frame[2] = 0x00;
      frame[3] = 0x00; frame[4] = 0x00; frame[5] = 0x01;
      frame[12] = 0x08; frame[13] = 0x00;
      frame[14] = 0x45;
      frame[16] = 0x00; frame[17] = 0x3c;
      frame[23] = 0x06; // TCP
      frame[34] = 0x10; frame[35] = 0x00; // src 4096
      frame[36] = 0x00; frame[37] = 0x50; // dst 80
      frame
    }),
    // IPv4 UDP on non-whitelisted port (dst=9999, src=1234)
    random_frame().prop_map(|mut frame| {
      frame[0] = 0x02; frame[1] = 0x00; frame[2] = 0x00;
      frame[3] = 0x00; frame[4] = 0x00; frame[5] = 0x01;
      frame[12] = 0x08; frame[13] = 0x00;
      frame[14] = 0x45;
      frame[16] = 0x00; frame[17] = 0x3c;
      frame[23] = 0x11; // UDP
      frame[34] = 0x04; frame[35] = 0xD2; // src 1234
      frame[36] = 0x27; frame[37] = 0x0F; // dst 9999
      frame
    }),
  ]
}

// Mixed strategy: weighted distribution across all frame categories plus None
fn mixed_frame_strategy() -> impl Strategy<Value = Option<SW::RawEthernetMessage>> {
  prop_oneof![
    1 => Just(None),
    2 => arp_frame_strategy().prop_map(Some),
    2 => mavlink_udp_frame_strategy().prop_map(Some),
    2 => whitelisted_udp_frame_strategy().prop_map(Some),
    2 => disallowed_frame_strategy().prop_map(Some),
    1 => generators::SW_RawEthernetMessage_strategy_default().prop_map(Some),
  ]
}

/// Weighted mix of all frame categories (ARP, MAVLink UDP, whitelisted UDP,
/// disallowed, random bytes) plus None inputs. Exercises every routing branch
/// and the no-input path in a single test.
testComputeCB_macro! {
  prop_claude_cust_strategies_mixed,
  config: ProptestConfig {
    cases: NUM_CASES,
    max_global_rejects: NUM_CASES * REJECT_RATIO,
    verbose: VERBOSITY,
    ..ProptestConfig::default()
  },
  api_EthernetFramesRxIn0: mixed_frame_strategy(),
  api_EthernetFramesRxIn1: mixed_frame_strategy(),
  api_EthernetFramesRxIn2: mixed_frame_strategy(),
  api_EthernetFramesRxIn3: mixed_frame_strategy()
}

/// All inputs are valid ARP frames. Verifies HLR-05: ARP frames are forwarded
/// unchanged to VmmOut via the can_send_to_vmm ARP path.
testComputeCB_macro! {
  prop_claude_cust_strategies_arp,
  config: ProptestConfig {
    cases: NUM_CASES,
    max_global_rejects: NUM_CASES * REJECT_RATIO,
    verbose: VERBOSITY,
    ..ProptestConfig::default()
  },
  api_EthernetFramesRxIn0: arp_frame_strategy().prop_map(Some),
  api_EthernetFramesRxIn1: arp_frame_strategy().prop_map(Some),
  api_EthernetFramesRxIn2: arp_frame_strategy().prop_map(Some),
  api_EthernetFramesRxIn3: arp_frame_strategy().prop_map(Some)
}

/// All inputs are MAVLink UDP frames (src port 14550, dst port 14562). Verifies
/// HLR-18: MAVLink UDP frames are split into header+payload and forwarded to
/// MavlinkOut via the can_send_to_mavlink path.
testComputeCB_macro! {
  prop_claude_cust_strategies_mavlink_udp,
  config: ProptestConfig {
    cases: NUM_CASES,
    max_global_rejects: NUM_CASES * REJECT_RATIO,
    verbose: VERBOSITY,
    ..ProptestConfig::default()
  },
  api_EthernetFramesRxIn0: mavlink_udp_frame_strategy().prop_map(Some),
  api_EthernetFramesRxIn1: mavlink_udp_frame_strategy().prop_map(Some),
  api_EthernetFramesRxIn2: mavlink_udp_frame_strategy().prop_map(Some),
  api_EthernetFramesRxIn3: mavlink_udp_frame_strategy().prop_map(Some)
}

/// All inputs are whitelisted UDP frames (dst port 68 / DHCP client). Verifies
/// HLR-13: non-MAVLink UDP frames on an allowed port are forwarded unchanged
/// to VmmOut via the can_send_to_vmm UDP allowlist path.
testComputeCB_macro! {
  prop_claude_cust_strategies_whitelisted_udp,
  config: ProptestConfig {
    cases: NUM_CASES,
    max_global_rejects: NUM_CASES * REJECT_RATIO,
    verbose: VERBOSITY,
    ..ProptestConfig::default()
  },
  api_EthernetFramesRxIn0: whitelisted_udp_frame_strategy().prop_map(Some),
  api_EthernetFramesRxIn1: whitelisted_udp_frame_strategy().prop_map(Some),
  api_EthernetFramesRxIn2: whitelisted_udp_frame_strategy().prop_map(Some),
  api_EthernetFramesRxIn3: whitelisted_udp_frame_strategy().prop_map(Some)
}

/// All inputs are parseable frames that should be dropped: IPv6, IPv4 TCP
/// (port 80), and IPv4 UDP on a non-whitelisted/non-MAVLink port (dst 9999).
/// Verifies HLR-15: disallowed frames produce no output on any port.
testComputeCB_macro! {
  prop_claude_cust_strategies_disallowed,
  config: ProptestConfig {
    cases: NUM_CASES,
    max_global_rejects: NUM_CASES * REJECT_RATIO,
    verbose: VERBOSITY,
    ..ProptestConfig::default()
  },
  api_EthernetFramesRxIn0: disallowed_frame_strategy().prop_map(Some),
  api_EthernetFramesRxIn1: disallowed_frame_strategy().prop_map(Some),
  api_EthernetFramesRxIn2: disallowed_frame_strategy().prop_map(Some),
  api_EthernetFramesRxIn3: disallowed_frame_strategy().prop_map(Some)
}
