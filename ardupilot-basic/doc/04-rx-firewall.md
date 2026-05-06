# RX Firewall

The RX Firewall classifies and routes inbound ethernet frames from the physical network. It has three routing destinations: the ArduPilot VM (via VMM output ports), the Mavlink Firewall (via Mavlink output ports), and the drop action (no output). This is the most complex firewall component, as it must distinguish between ARP, allowed UDP, and MAVLink UDP traffic.

## Requirements

From `requirements/Inspecta-HLRs.pdf`, Section 1.0:

| HLR ID | Requirement | Summary |
|--------|-------------|---------|
| RC_INSPECTA_00-HLR-5 | Copy through any ARP frame to VMM output | Forward well-formed ARP frames directly to the ArduPilot VM |
| RC_INSPECTA_00-HLR-13 | Copy through allowed UDP port frames to VMM output | Forward frames with UDP destination port in the whitelist (not MAVLink) to the VM |
| RC_INSPECTA_00-HLR-18 | Copy through mavlink UDP port frames to mavlink_firewall output | Forward MAVLink UDP frames (src=14550, dst=14562) to the Mavlink Firewall |
| RC_INSPECTA_00-HLR-15 | Do not copy disallowed frame | Drop frames that don't match any allowed category |
| RC_INSPECTA_00-HLR-17 | No output on empty input | Produce no output when the input port has no data |

### Routing Decision

The RX Firewall classifies each frame into exactly one category:

| Category | Condition | Destination |
|----------|-----------|-------------|
| Valid ARP | Well-formed Ethernet + ARP EtherType + well-formed ARP | VMM output |
| Valid MAVLink UDP | Well-formed Ethernet + IPv4 + well-formed IPv4 + UDP protocol + src=14550 + dst=14562 | Mavlink output (as `UdpFrame_Impl`) |
| Valid allowed-port UDP | Well-formed Ethernet + IPv4 + well-formed IPv4 + UDP protocol + dst port in whitelist + NOT MAVLink | VMM output |
| Disallowed | None of the above | Dropped |

### Port Configuration

| Port List | Values | Source |
|-----------|--------|--------|
| `UDP_ALLOWED_PORTS` | [68] | DHCP client |
| `TCP_ALLOWED_PORTS` | [5760] | MAVLink TCP (currently inactive) |

See [01-ethernet-frame-formats.md](01-ethernet-frame-formats.md) for complete byte-level details of each check.

## GUMBO Contracts

### Integration Assumes (`SW.sysml`, lines 182-208)

The RX Firewall assumes that each input frame is one of the categories that the `LowLevelEthernetDriver` can produce:

```gumbo
assume valid_message_port0:
    GumboLib::valid_arp(EthernetFramesRxIn0)
    or GumboLib::valid_ipv4_udp_mavlink(EthernetFramesRxIn0)
    or GumboLib::valid_ipv4_udp_port(EthernetFramesRxIn0)
    or not GumboLib::rx_allow_outbound_frame(EthernetFramesRxIn0);
```

This is a tautology (it says the frame is either allowed or not allowed), but it makes the categorization explicit and mirrors the guarantees from the `LowLevelEthernetDriver`.

### Compute Guarantees (`SW.sysml`, lines 210-271)

Shown for port 0 (ports 1-3 are identical):

**HLR-5: Forward ARP to VMM** (`hlr_05_rx0_can_send_arp_to_vmm`)
```gumbo
(HasEvent(EthernetFramesRxIn0) and GumboLib::valid_arp(EthernetFramesRxIn0)) implies
    (HasEvent(VmmOut0) and (EthernetFramesRxIn0 == VmmOut0) and NoSend(MavlinkOut0));
```
Valid ARP -> forward unchanged to VMM, nothing to Mavlink Firewall.

**HLR-18: Forward MAVLink UDP to Mavlink Firewall** (`hlr_18_rx0_can_send_mavlink_udp`)
```gumbo
(HasEvent(EthernetFramesRxIn0) and GumboLib::valid_ipv4_udp_mavlink(EthernetFramesRxIn0)) implies
    (HasEvent(MavlinkOut0)
        and GumboLib::input_eq_mav_output(EthernetFramesRxIn0, MavlinkOut0)
        and NoSend(VmmOut0));
```
MAVLink UDP -> split into `UdpFrame_Impl` (headers + payload) and send to Mavlink Firewall. The `input_eq_mav_output` predicate verifies that the split preserves all bytes: the first 42 bytes go to `headers` and the remaining 1558 bytes go to `payload`.

**HLR-13: Forward allowed UDP to VMM** (`hlr_13_rx0_can_send_ipv4_udp`)
```gumbo
(HasEvent(EthernetFramesRxIn0) and GumboLib::valid_ipv4_udp_port(EthernetFramesRxIn0)) implies
    (HasEvent(VmmOut0) and (EthernetFramesRxIn0 == VmmOut0) and NoSend(MavlinkOut0));
```
Non-MAVLink UDP with an allowed destination port -> forward unchanged to VMM.

**HLR-15: Drop disallowed** (`hlr_15_rx0_disallow`)
```gumbo
(HasEvent(EthernetFramesRxIn0)
    and not GumboLib::rx_allow_outbound_frame(EthernetFramesRxIn0)) implies
    (NoSend(VmmOut0) and NoSend(MavlinkOut0));
```

**HLR-17: No output on empty input** (`hlr_17_rx0_no_input`)
```gumbo
(not HasEvent(EthernetFramesRxIn0)) implies (NoSend(VmmOut0) and NoSend(MavlinkOut0));
```

## Implementation

**Source file:** `hamr/microkit/crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs`

### Key Functions

#### `get_frame_packet` (lines 207-222)

Parses a `RawEthernetMessage` using `firewall_core`, with ensures clauses bridging GumboLib specs to parse results:

```rust
fn get_frame_packet(frame: &SW::RawEthernetMessage) -> (r: Option<EthFrame>)
    ensures
        valid_arp_spec(*frame) == firewall_core::res_is_arp(r),
        valid_ipv4_udp_spec(*frame) == firewall_core::res_is_udp(r),
        valid_ipv4_tcp_spec(*frame) == firewall_core::res_is_tcp(r),
        valid_ipv4_tcp_spec(*frame) ==> firewall_core::tcp_port_bytes_match(frame, r),
        valid_ipv4_udp_spec(*frame) ==> firewall_core::udp_port_bytes_match(frame, r),
```

The additional postconditions for TCP and UDP port byte matching enable downstream code to verify that port-based routing decisions are correct.

#### `can_send_to_mavlink` (lines 187-197)

Checks if a parsed packet is a MAVLink UDP frame:

```rust
fn can_send_to_mavlink(packet: &PacketType) -> (r: bool)
    ensures
        (packet_is_mavlink_udp(packet)) == (r == true),
```

A packet is MAVLink UDP if it is IPv4 with UDP protocol and source port 14550, destination port 14562.

#### `can_send_to_vmm` (lines 142-177)

Checks if a parsed packet should be routed to the VMM:

```rust
fn can_send_to_vmm(packet: &PacketType) -> (r: bool)
    requires
        config::udp::ALLOWED_PORTS =~= UDP_ALLOWED_PORTS_spec(),
    ensures
        ((packet is Arp) || packet_is_whitelisted_udp(packet)) == (r == true),
```

A packet is sent to the VMM if it is ARP, or if it is UDP with a whitelisted destination port. The `requires` clause ties the runtime port configuration to the spec-level allowed ports list.

**Note:** The TCP path is commented out in the implementation (lines 153-159), though the GUMBO contracts and GumboLib predicates for TCP are still defined. This means the current implementation is more restrictive than what GUMBO allows.

#### `udp_frame_from_raw_eth` (lines 62-70)

Splits a `RawEthernetMessage` into a `UdpFrame_Impl`:

```rust
fn udp_frame_from_raw_eth(value: SW::RawEthernetMessage) -> (r: UdpFrame_Impl)
    ensures
        r.headers@ =~= value@.subrange(0, SW_EthIpUdpHeaders_DIM_0 as int),
        r.payload@ =~= value@.subrange(SW_EthIpUdpHeaders_DIM_0 as int, SW_RawEthernetMessage_DIM_0 as int),
```

This splits the 1600-byte frame into:
- `headers`: bytes 0-41 (the 42-byte Ethernet + IPv4 + UDP headers)
- `payload`: bytes 42-1599 (the 1558-byte UDP payload containing the MAVLink message)

The Verus postconditions prove that the split preserves all bytes, which is essential for satisfying the `input_eq_mav_output` GUMBO predicate.

#### `timeTriggered` (lines 233-380)

The main compute entry point. For each of the 4 ports:

```rust
if let Some(frame) = api.get_EthernetFramesRxIn0() {         // Check for input
    if let Some(eth) = Self::get_frame_packet(&frame) {        // Parse frame
        if can_send_to_mavlink(&eth.eth_type) {                // MAVLink UDP?
            let output = udp_frame_from_raw_eth(frame);
            api.put_MavlinkOut0(output);                       // -> Mavlink Firewall
        } else if can_send_to_vmm(&eth.eth_type) {            // ARP or allowed UDP?
            api.put_VmmOut0(frame);                            // -> VMM
        }
    }
}
```

The routing priority is:
1. **MAVLink UDP** (checked first): split and send to Mavlink Firewall
2. **ARP or allowed-port UDP**: forward unchanged to VMM
3. **Everything else**: dropped (no output API called)

This priority order matters: a MAVLink UDP frame would also pass the `can_send_to_vmm` check (since it's a valid UDP), but it should go to the Mavlink Firewall for deep inspection, not directly to the VM.

### The `firewall_core` Library

The RX Firewall uses the same `firewall_core::EthFrame::parse()` as the TX Firewall, but relies on additional postconditions for UDP/TCP port byte matching. The structured `UdpRepr` and `TcpRepr` types provide parsed port numbers that the routing functions check.

## Traceability Matrix

| HLR | GUMBO Guarantee | Verus Ensures | Code Path |
|-----|-----------------|---------------|-----------|
| HLR-5 | `hlr_05_rx{0-3}_can_send_arp_to_vmm` | `valid_arp_spec(input) ==> VmmOut.is_some() && VmmOut == input && MavlinkOut.is_none()` | `get_frame_packet` -> `can_send_to_mavlink` false -> `can_send_to_vmm` true (ARP) -> `put_VmmOut` |
| HLR-18 | `hlr_18_rx{0-3}_can_send_mavlink_udp` | `valid_ipv4_udp_mavlink_spec(input) ==> MavlinkOut.is_some() && input_eq_mav_output_spec(input, MavlinkOut) && VmmOut.is_none()` | `get_frame_packet` -> `can_send_to_mavlink` true -> `udp_frame_from_raw_eth` -> `put_MavlinkOut` |
| HLR-13 | `hlr_13_rx{0-3}_can_send_ipv4_udp` | `valid_ipv4_udp_port_spec(input) ==> VmmOut.is_some() && VmmOut == input && MavlinkOut.is_none()` | `get_frame_packet` -> `can_send_to_mavlink` false -> `can_send_to_vmm` true (allowed UDP) -> `put_VmmOut` |
| HLR-15 | `hlr_15_rx{0-3}_disallow` | `!rx_allow_outbound_frame_spec(input) ==> VmmOut.is_none() && MavlinkOut.is_none()` | `get_frame_packet` returns `None`, or both `can_send_to_mavlink` and `can_send_to_vmm` return false |
| HLR-17 | `hlr_17_rx{0-3}_no_input` | `!input.is_some() ==> VmmOut.is_none() && MavlinkOut.is_none()` | `get_EthernetFramesRxIn` returns `None` |
