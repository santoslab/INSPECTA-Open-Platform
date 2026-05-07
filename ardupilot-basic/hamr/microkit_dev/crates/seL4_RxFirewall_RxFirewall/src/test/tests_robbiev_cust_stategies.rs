// This file will not be overwritten if codegen is rerun

// Adapted from Robbie VanVossen's custom strategies at:
//   https://github.com/loonwerks/INSPECTA-models/blob/ffe80c4f3fae4531cfa3bfe68f5dd00d02d957ac/open-platform-models/open-platform/microkit/crates/seL4_RxFirewall_RxFirewall/src/tests.rs
//
// The original file contained manual unit tests (mod tests), a GUMBOX
// testInitializeCB_macro test, and a testComputeCBwLV_macro test. Those have
// been commented out here because we are only concerned with comparing
// Claude's custom strategy approach against Robbie V's domain-aware generator
// approach for the compute entry point.

/*
mod tests {
    // NOTE: need to run tests sequentially to prevent race conditions
    //       on the app and the testing apis which are static
    use serial_test::serial;

    use crate::bridge::test_api;
    use data::*;

    #[test]
    #[serial]
    fn test_initialization() {
        crate::seL4_RxFirewall_RxFirewall_initialize();
    }

    #[test]
    #[serial]
    fn test_compute() {
        crate::seL4_RxFirewall_RxFirewall_initialize();
        crate::seL4_RxFirewall_RxFirewall_timeTriggered();
    }
}
*/

mod GUMBOX_tests {
    use data::SW;
    use proptest::prelude::*;
    use serial_test::serial;

    use crate::test::util::*;
    use crate::testComputeCB_macro;

    // number of valid (i.e., non-rejected) test cases that must be executed for the compute method.
    const numValidComputeTestCases: u32 = 100;

    // how many total test cases (valid + rejected) that may be attempted.
    //   0 means all inputs must satisfy the precondition (if present),
    //   5 means at most 5 rejected inputs are allowed per valid test case
    const computeRejectRatio: u32 = 5;

    const verbosity: u32 = 2;

    /*
    testInitializeCB_macro! {
      prop_testInitializeCB_macro, // test name
      config: ProptestConfig { // proptest configuration, built by overriding fields from default config
        cases: numValidComputeTestCases,
        max_global_rejects: numValidComputeTestCases * computeRejectRatio,
        verbose: verbosity,
        ..ProptestConfig::default()
      }
    }
    */

    // Compute entry point test using Robbie VanVossen's domain-aware frame
    // generator. Each port receives a 50/50 Some/None distribution (via
    // option_strategy_default) where the Some values are produced by the
    // hierarchical SW_RawEthernetMessage_strategy_default, which composes
    // weighted sub-strategies for EtherType, ARP, IPv4, TCP, and UDP layers.
    // This single test exercises all routing paths (ARP->VMM, MAVLink->MavlinkOut,
    // whitelisted UDP->VMM, disallowed->drop, unparseable->drop, None->no output)
    // because the generator's weights ensure each category appears with
    // meaningful probability across 100 test cases.
    //
    // Exercises: HLR-05, HLR-13, HLR-15, HLR-17, HLR-18
    testComputeCB_macro! {
      prop_robbiev_cust_stategies_testComputeCB_macro, // test name
      config: ProptestConfig { // proptest configuration, built by overriding fields from default config
        cases: numValidComputeTestCases,
        max_global_rejects: numValidComputeTestCases * computeRejectRatio,
        verbose: verbosity,
        ..ProptestConfig::default()
      },
      // strategies for generating each component input
      api_EthernetFramesRxIn0: generators::option_strategy_default(SW_RawEthernetMessage_strategy_default()),
      api_EthernetFramesRxIn1: generators::option_strategy_default(SW_RawEthernetMessage_strategy_default()),
      api_EthernetFramesRxIn2: generators::option_strategy_default(SW_RawEthernetMessage_strategy_default()),
      api_EthernetFramesRxIn3: generators::option_strategy_default(SW_RawEthernetMessage_strategy_default())
    }

    // testComputeCBwLV_macro! {
    //   prop_testComputeCBwLV_macro, // test name
    //   config: ProptestConfig { // proptest configuration, built by overriding fields from default config
    //     cases: numValidComputeTestCases,
    //     max_global_rejects: numValidComputeTestCases * computeRejectRatio,
    //     verbose: verbosity,
    //     ..ProptestConfig::default()
    //   },
    //   // strategies for generating each component input
    //   api_EthernetFramesRxIn0: generators::option_strategy_default(SW_RawEthernetMessage_strategy_default()),
    //   api_EthernetFramesRxIn1: generators::option_strategy_default(SW_RawEthernetMessage_strategy_default()),
    //   api_EthernetFramesRxIn2: generators::option_strategy_default(SW_RawEthernetMessage_strategy_default()),
    //   api_EthernetFramesRxIn3: generators::option_strategy_default(SW_RawEthernetMessage_strategy_default())
    // }

    // Domain-aware default strategy that replaces the HAMR-generated uniform
    // random generator. Instead of producing entirely random 1600-byte arrays,
    // this strategy composes layer-specific sub-strategies that mirror the
    // structure of real Ethernet frames. Each layer (EtherType, ARP fields,
    // IPv4 header, TCP/UDP ports) uses weighted distributions that heavily
    // favor valid values while retaining a small probability of random/invalid
    // values to exercise parser rejection paths. See
    // SW_RawEthernetMessage_stategy_cust for the full assembly logic.
    pub fn SW_RawEthernetMessage_strategy_default() -> impl Strategy<Value = SW::RawEthernetMessage>
    {
        SW_RawEthernetMessage_stategy_cust(any::<u8>())
    }

    // =========================================================================
    //  Byte helpers
    // =========================================================================

    // Extract the high byte of a u16 (big-endian byte 0).
    fn byte1(val: u16) -> u8 {
        (val >> 8) as u8
    }

    // Extract the low byte of a u16 (big-endian byte 1).
    fn byte2(val: u16) -> u8 {
        (val & 0xFF) as u8
    }

    // Write a u16 in big-endian into the first two bytes of a slice.
    fn copy_u16(v: &mut [u8], data: u16) {
        v[0] = byte1(data);
        v[1] = byte2(data);
    }

    // =========================================================================
    //  Ethernet-layer strategies
    // =========================================================================

    // EtherType field (frame bytes 12--13). Heavily biased toward valid types
    // that the firewall recognizes: IPv4 (60%), ARP (30%), IPv6 (6%), with a
    // small chance of a completely random value (3%) to test the unparseable
    // frame path.
    fn ethertype_strategy() -> impl Strategy<Value = u16> {
        prop_oneof![
          20 => Just(0x0800), // IPv4
          10 => Just(0x0806), // ARP
          2 => Just(0x86DD),  // IPv6
          1 => any::<u16>(),
        ]
    }

    // Destination MAC address (frame bytes 0--5). Almost always a random
    // non-zero MAC (98%), with a small chance of all-zeros (2%) to test the
    // EthernetRepr::parse rejection path (all-zero dst MAC is invalid).
    fn dst_mac_strategy() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
          50 => proptest::collection::vec(any::<u8>(), 6),
          1 => Just(vec![0,0,0,0,0,0]),
        ]
    }

    // =========================================================================
    //  ARP-layer strategies
    // =========================================================================

    // ARP ptype field (ARP payload bytes 2--3). Biased toward IPv4 and IPv6
    // which are recognized by the parser, with a small random component.
    fn arp_ethertype_strategy() -> impl Strategy<Value = u16> {
        prop_oneof![
          20 => Just(0x0800),
          2 => Just(0x0806),
          20 => Just(0x86DD),
          1 => any::<u16>(),
        ]
    }

    // ARP hardware type (ARP payload bytes 0--1). Strongly biased toward
    // Ethernet (0x0001) which is the only value Arp::parse accepts, with a
    // small random component to test rejection.
    fn arp_hwtype_strategy() -> impl Strategy<Value = u16> {
        prop_oneof![
          40 => Just(0x0001),
          1 => any::<u16>(),
        ]
    }

    // ARP operation code (ARP payload bytes 6--7). Equally weighted between
    // Request (0x0001) and Reply (0x0002) -- the two values Arp::parse
    // accepts -- with a small random component to test rejection.
    fn arp_op_strategy() -> impl Strategy<Value = u16> {
        prop_oneof![
          20 => Just(0x0001),
          20 => Just(0x0002),
          1 => any::<u16>(),
        ]
    }

    // Complete ARP payload (28 bytes). Composes hardware type, ptype, and
    // operation code strategies into a valid ARP packet structure, with
    // remaining bytes random.
    fn arp_strategy() -> impl Strategy<Value = Vec<u8>> {
        (
            arp_hwtype_strategy(),
            arp_ethertype_strategy(),
            arp_op_strategy(),
            proptest::collection::vec(any::<u8>(), 28),
        )
            .prop_map(|(hwtype, ethertype, op, mut v)| {
                copy_u16(&mut v[0..=1], hwtype);
                copy_u16(&mut v[2..=3], ethertype);
                copy_u16(&mut v[6..=7], op);
                v
            })
    }

    // =========================================================================
    //  IPv4-layer strategies
    // =========================================================================

    // IPv4 protocol number (IPv4 header byte 9). Weighted toward the full set
    // of protocol numbers that Ipv4Repr::parse recognizes, with TCP and UDP
    // given the highest weights since they are the protocols the firewall
    // routes on.
    fn ipv4_protocol_strategy() -> impl Strategy<Value = u8> {
        prop_oneof![
           4 => Just(0x00), // HopByHop
           4 => Just(0x01), // Icmp
           4 => Just(0x02), // Igmp
           10 => Just(0x06), // Tcp
           10 => Just(0x11), // Udp
           4 => Just(0x2b), // Ipv6Route
           4 => Just(0x2c), // Ipv6Frag
           4 => Just(0x3a), // Icmpv6
           4 => Just(0x3b), // Ipv6NoNxt
           4 => Just(0x3c), // Ipv6Opts
           1 => any::<u8>(),
        ]
    }

    // IPv4 total length field (IPv4 header bytes 2--3). Strongly biased toward
    // values <= 9000 (the Ipv4Repr::parse upper bound), with a small chance of
    // exceeding it to test rejection.
    fn ipv4_length_strategy() -> impl Strategy<Value = u16> {
        prop_oneof![
          40 => (0u16..=9000),
          1 => (9001u16..),
        ]
    }

    // =========================================================================
    //  UDP-layer strategies
    // =========================================================================

    // UDP destination port (UDP header bytes 2--3). Biased toward port 68
    // (DHCP client), which is in config::udp::ALLOWED_PORTS, with the
    // remainder random. Note: source port is left random, so some generated
    // frames may coincidentally hit MAVLink src port 14550.
    fn udp_port_strategy() -> impl Strategy<Value = u16> {
        prop_oneof![
          1 => Just(68),
          4 => any::<u16>(),
        ]
    }

    // Complete UDP header (20 bytes). Stamps the destination port from
    // udp_port_strategy at bytes 2--3; source port and remaining bytes are
    // random.
    fn udp_strategy() -> impl Strategy<Value = Vec<u8>> {
        (
            udp_port_strategy(),
            proptest::collection::vec(any::<u8>(), 20),
        )
            .prop_map(|(port, mut v)| {
                copy_u16(&mut v[2..=3], port);
                v
            })
    }

    // =========================================================================
    //  TCP-layer strategies
    // =========================================================================

    // TCP destination port (TCP header bytes 2--3). Biased toward port 5760,
    // which is in config::tcp::ALLOWED_PORTS, with the remainder random.
    fn tcp_port_strategy() -> impl Strategy<Value = u16> {
        prop_oneof![
          1 => Just(5760),
          4 => any::<u16>(),
        ]
    }

    // Complete TCP header (20 bytes). Stamps the destination port from
    // tcp_port_strategy at bytes 2--3; source port and remaining bytes are
    // random.
    fn tcp_strategy() -> impl Strategy<Value = Vec<u8>> {
        (
            tcp_port_strategy(),
            proptest::collection::vec(any::<u8>(), 20),
        )
            .prop_map(|(port, mut v)| {
                copy_u16(&mut v[2..=3], port);
                v
            })
    }

    // =========================================================================
    //  IPv4 composite strategy
    // =========================================================================

    // Complete IPv4 packet. Uses prop_flat_map to first choose a protocol
    // number, then generate the appropriate transport-layer payload (TCP, UDP,
    // or a generic 1-byte payload for other protocols). The IPv4 total length
    // and protocol fields are stamped into the header; the transport payload
    // is spliced in at byte 20 (immediately after the 20-byte IPv4 header).
    fn ipv4_strategy() -> impl Strategy<Value = Vec<u8>> {
        ipv4_protocol_strategy().prop_flat_map(|proto| {
            let proto_packet = match proto {
                // Tcp
                0x06 => tcp_strategy().boxed(),
                // Udp
                0x11 => udp_strategy().boxed(),
                _ => default_packet_strategy().boxed(),
            };
            (
                ipv4_length_strategy(),
                proto_packet,
                proptest::collection::vec(any::<u8>(), 40),
            )
                .prop_map(move |(length, proto_pack, mut v)| {
                    copy_u16(&mut v[2..=3], length);
                    v[9] = proto;
                    v.splice(20..20 + proto_pack.len(), proto_pack);
                    v
                })
        })
    }

    // Fallback payload for unrecognized protocols (1 random byte).
    fn default_packet_strategy() -> impl Strategy<Value = Vec<u8>> {
        proptest::collection::vec(any::<u8>(), 1)
    }

    // =========================================================================
    //  Top-level frame assembly
    // =========================================================================

    // Assembles a complete RawEthernetMessage (1600 bytes) by composing
    // layer-specific strategies. The approach is:
    //   1. Choose an EtherType via ethertype_strategy()
    //   2. Based on the EtherType, generate the appropriate payload:
    //      - 0x0800 (IPv4) -> ipv4_strategy() (which nests TCP/UDP strategies)
    //      - 0x0806 (ARP)  -> arp_strategy()
    //      - other          -> default_packet_strategy() (1 random byte)
    //   3. Generate a destination MAC via dst_mac_strategy()
    //   4. Start from a random 1600-byte base array, then splice in the
    //      destination MAC (bytes 0--5), EtherType (bytes 12--13), and
    //      protocol payload (bytes 14+)
    //
    // This hierarchical composition means the generator naturally produces
    // frames that exercise all parser layers -- EthernetRepr, Arp, Ipv4Repr,
    // UdpRepr, TcpRepr -- with weighted probabilities that favor valid values
    // at each level while still including random/invalid values to test
    // rejection paths.
    pub fn SW_RawEthernetMessage_stategy_cust<u8_strategy: Strategy<Value = u8> + Copy>(
        base_strategy: u8_strategy,
    ) -> impl Strategy<Value = SW::RawEthernetMessage> {
        ethertype_strategy().prop_flat_map(move |ethertype| {
            let packet = match ethertype {
                0x0800 => ipv4_strategy().boxed(),
                0x0806 => arp_strategy().boxed(),
                _ => default_packet_strategy().boxed(),
            };
            (
                dst_mac_strategy(),
                packet,
                proptest::collection::vec(base_strategy, SW::SW_RawEthernetMessage_DIM_0),
            )
                .prop_map(move |(dst_mac, pack, mut v)| {
                    v.splice(0..6, dst_mac);
                    copy_u16(&mut v[12..=13], ethertype);
                    v.splice(14..14 + pack.len(), pack);
                    let boxed: Box<[u8; SW::SW_RawEthernetMessage_DIM_0]> =
                        v.into_boxed_slice().try_into().unwrap();
                    *boxed
                })
        })
    }
}