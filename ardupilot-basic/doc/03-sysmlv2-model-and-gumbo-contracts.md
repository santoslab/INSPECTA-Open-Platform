# SysMLv2 Model and GUMBO Contracts

This document describes the SysMLv2 architecture model for the ardupilot-basic system, the GUMBO contract language used to specify firewall behavior, and the GumboLib predicate library that encodes the byte-level validation checks.

## Model File Organization

| File | Purpose |
|------|---------|
| `SW.sysml` | All software components: data types, thread definitions, process definitions, system assembly, GUMBO contracts |
| `GumboLib.sysml` | Reusable GUMBO predicate library for frame validation and port checking |
| `Platform.sysml` | Processor definition (`Frame_Period = 1850ms`, `Clock_Period = 2ms`) and top-level system binding |

## Architecture Model Structure

### Data Types (`SW.sysml`, lines 7-43)

See [00-overview.md](00-overview.md) for the complete data types table. The key types are `RawEthernetMessage` (1600-byte array), `UdpFrame_Impl` (headers + payload split), and `SizedEthernetMessage_Impl` (frame + size).

### Thread Definitions

Each firewall is modeled as an AADL Thread with typed ports and GUMBO contracts:

| Thread | Dispatch | Period | Ports In | Ports Out |
|--------|----------|--------|----------|-----------|
| `LowLevelEthernetDriver` | Periodic | 1000ms | 4x `SizedEthernetMessage_Impl` | 4x `RawEthernetMessage` |
| `RxFirewall` | Periodic | 1000ms | 4x `RawEthernetMessage` | 4x `RawEthernetMessage` (VMM) + 4x `UdpFrame_Impl` (Mavlink) |
| `TxFirewall` | Periodic | 1000ms | 4x `RawEthernetMessage` | 4x `SizedEthernetMessage_Impl` |
| `MavlinkFirewall` | Periodic | 1000ms | 4x `UdpFrame_Impl` | 4x `RawEthernetMessage` |
| `ArduPilot` | Periodic | 1000ms | 4x `RawEthernetMessage` (Firewall) + 4x `RawEthernetMessage` (Mavlink) | 4x `RawEthernetMessage` |

All threads use `EventDataPort` for communication, meaning data is only present when an event occurs (modeled as `Option<T>` in Rust).

### Process Definitions and Scheduling Domains

Each thread is wrapped in a Process that assigns it to a seL4 scheduling domain:

| Process | Thread | Domain |
|---------|--------|--------|
| `ArduPilot_seL4` | `ArduPilot` | 2 |
| `TxFirewall_seL4` | `TxFirewall` | 3 |
| `LowLevelEthernetDriver_seL4` | `LowLevelEthernetDriver` | 4 |
| `RxFirewall_seL4` | `RxFirewall` | 5 |
| `MavlinkFirewall_seL4` | `MavlinkFirewall` | 6 |

### System Assembly (`SW.sysml`, lines 616-679)

The `seL4` system definition instantiates all five processes and defines the connections:
- `LowLevelEthernetDriver.EthernetFramesRx{0-3}` -> `RxFirewall.EthernetFramesRxIn{0-3}`
- `RxFirewall.VmmOut{0-3}` -> `ArduPilot.FirewallRx{0-3}`
- `RxFirewall.MavlinkOut{0-3}` -> `MavlinkFirewall.In{0-3}`
- `MavlinkFirewall.Out{0-3}` -> `ArduPilot.MavlinkRx{0-3}`
- `ArduPilot.EthernetFramesTx{0-3}` -> `TxFirewall.EthernetFramesTxIn{0-3}`
- `TxFirewall.EthernetFramesTxOut{0-3}` -> `LowLevelEthernetDriver.EthernetFramesTx{0-3}`

## GUMBO Contract Language

GUMBO (GUMBO Unified Model-Based Obligations) is a contract language embedded in SysMLv2 models using `language "GUMBO" /*{ ... }*/` blocks. GUMBO contracts specify component behavior at the model level; HAMR translates them to Verus contracts on the generated Rust code.

### Contract Sections

**`integration`** -- Constraints on port values that hold between components:
- `assume`: What a component may assume about its inputs (guaranteed by the sending component).
- `guarantee`: What a component promises about its outputs (must be satisfied by the implementation).

**`compute`** -- Per-dispatch-cycle contracts:
- `guarantee`: What the component guarantees about outputs given inputs after each dispatch.

**`functions`** -- Helper predicate definitions:
- Regular `def`: A concrete function with a body.
- `@spec def`: An uninterpreted function whose semantics must be supplied by the developer in Verus. Used when the behavior is too complex for GUMBO (e.g., parsing a MAVLink message).

### Key GUMBO Operators

| Operator | Meaning | Usage |
|----------|---------|-------|
| `HasEvent(port)` | True if the port has a value this dispatch cycle | Guards access to EventDataPort values |
| `NoSend(port)` | Guarantees no value is sent on an output port | Used in drop/no-input scenarios |
| `implies` | Short-circuit logical implication | `condition implies consequence` |
| `and` | Short-circuit logical AND | Guards field access after `HasEvent` |
| `not` | Logical negation | |

**Important:** Use `and` (short-circuit) rather than `&` (evaluates both sides) when combining `HasEvent` with field access. With `&`, both sides are evaluated even when `HasEvent` is false, which would attempt to access a non-existent message.

### Contract Pattern for Firewall Components

All three firewalls follow the same GUMBO contract pattern for each port. Using port 0 as an example:

```
compute
    guarantee allow_case:
        (HasEvent(Input0) and <allowed_condition>) implies
            (HasEvent(Output0) and <output_correctness>);
    guarantee disallow_case:
        (HasEvent(Input0) and not <allowed_condition>) implies
            NoSend(Output0);
    guarantee no_input:
        (not HasEvent(Input0)) implies NoSend(Output0);
```

This pattern ensures completeness: every possible input scenario (allowed input, disallowed input, no input) has a specified output behavior.

## GumboLib Predicate Library

The `GumboLib.sysml` file defines reusable predicate functions organized by category. Each GUMBO predicate corresponds to a check described in [01-ethernet-frame-formats.md](01-ethernet-frame-formats.md).

### Byte Conversion Utilities

| Function | Signature | Description |
|----------|-----------|-------------|
| `two_bytes_to_u16_be` | `(u8, u8) -> u16` | Big-endian conversion: `byte0 * 256 + byte1` |
| `two_bytes_to_u16_le` | `(u8, u8) -> u16` | Little-endian conversion: `byte1 * 256 + byte0` |
| `three_bytes_to_u32` | `(u8, u8, u8) -> u32` | `byte2 * 65536 + byte1 * 256 + byte0` |

### Ethernet Header Predicates

| Function | Checks | Bytes |
|----------|--------|-------|
| `valid_frame_ethertype(f)` | EtherType is IPv4, ARP, or IPv6 | 12-13 |
| `valid_frame_dst_addr(f)` | Destination MAC is not all zeros | 0-5 |
| `frame_is_wellformed_eth2(f)` | `valid_frame_ethertype AND valid_frame_dst_addr` | 0-13 |
| `frame_has_ipv4(f)` | EtherType = `0x0800` | 12-13 |
| `frame_has_arp(f)` | EtherType = `0x0806` | 12-13 |
| `frame_has_ipv6(f)` | EtherType = `0x86DD` | 12-13 |

### ARP Predicates

| Function | Checks | Bytes |
|----------|--------|-------|
| `valid_arp_htype(f)` | Hardware type = `0x0001` (Ethernet) | 14-15 |
| `valid_arp_ptype(f)` | Protocol type is IPv4 or IPv6 | 16-17 |
| `arp_has_ipv4(f)` | ARP protocol type = `0x0800` | 16-17 |
| `arp_has_ipv6(f)` | ARP protocol type = `0x86DD` | 16-17 |
| `valid_arp_op(f)` | Operation = Request (`0x0001`) or Reply (`0x0002`) | 20-21 |
| `wellformed_arp_frame(f)` | `valid_arp_op AND valid_arp_htype AND valid_arp_ptype` | 14-21 |

### IPv4 Predicates

| Function | Checks | Bytes |
|----------|--------|-------|
| `valid_ipv4_vers_ihl(f)` | Version+IHL = `0x45` (v4, no options) | 14 |
| `valid_ipv4_length(f)` | Total length <= 9000 | 16-17 |
| `valid_ipv4_protocol(f)` | Protocol is one of 10 allowed values | 23 |
| `wellformed_ipv4_frame(f)` | `valid_ipv4_protocol AND valid_ipv4_length AND valid_ipv4_vers_ihl` | 14-23 |

### Transport Layer Predicates

| Function | Checks | Bytes |
|----------|--------|-------|
| `ipv4_is_tcp(f)` | Protocol byte = 6 | 23 |
| `ipv4_is_udp(f)` | Protocol byte = 17 | 23 |
| `tcp_is_valid_port(f)` | TCP destination port = 5760 | 36-37 |
| `udp_is_valid_port(f)` | UDP destination port = 68 | 36-37 |
| `udp_is_valid_direct_dst_port(f)` | UDP dest port in `UDP_ALLOWED_PORTS` (quantified) | 36-37 |
| `frame_has_ipv4_tcp_on_allowed_port_quant(f)` | TCP dest port in `TCP_ALLOWED_PORTS` (quantified) | 36-37 |

### MAVLink Identification Predicates

| Function | Checks | Bytes |
|----------|--------|-------|
| `udp_is_mavlink_src_port(f)` | UDP source port = 14550 | 34-35 |
| `udp_is_mavlink_dst_port(f)` | UDP destination port = 14562 | 36-37 |
| `udp_is_mavlink(f)` | `udp_is_mavlink_src_port AND udp_is_mavlink_dst_port` | 34-37 |

### Composite Decision Predicates

| Function | Definition | Used By |
|----------|-----------|---------|
| `valid_arp(f)` | `frame_is_wellformed_eth2 AND frame_has_arp AND wellformed_arp_frame` | RX, TX |
| `valid_ipv4(f)` | `frame_is_wellformed_eth2 AND frame_has_ipv4 AND wellformed_ipv4_frame` | TX |
| `valid_ipv4_tcp(f)` | `frame_is_wellformed_eth2 AND frame_has_ipv4 AND wellformed_ipv4_frame AND ipv4_is_tcp` | RX |
| `valid_ipv4_udp(f)` | `frame_is_wellformed_eth2 AND frame_has_ipv4 AND wellformed_ipv4_frame AND ipv4_is_udp` | RX |
| `valid_ipv4_tcp_port(f)` | `valid_ipv4_tcp AND frame_has_ipv4_tcp_on_allowed_port_quant` | RX |
| `valid_ipv4_udp_port(f)` | `valid_ipv4_udp AND udp_is_valid_direct_dst_port AND NOT udp_is_mavlink` | RX |
| `valid_ipv4_udp_mavlink(f)` | `valid_ipv4_udp AND udp_is_mavlink` | RX |
| `rx_allow_outbound_frame(f)` | `valid_arp OR valid_ipv4_udp_mavlink OR valid_ipv4_udp_port` | RX |
| `tx_allow_outbound_frame(f)` | `valid_arp OR valid_ipv4` | TX |

### Output Size Predicates

| Function | Definition | Used By |
|----------|-----------|---------|
| `valid_output_arp_size(output)` | `output.sz == 64` | TX |
| `valid_output_ipv4_size(input, output)` | `output.sz == two_bytes_to_u16_be(input#(16), input#(17)) + 14` | TX |

### Output Equality Predicates

These predicates verify that outputs faithfully reproduce inputs:

| Function | Definition | Used By |
|----------|-----------|---------|
| `input_eq_mav_output_headers(f, headers)` | All 42 header bytes match between input frame and output | RX |
| `input_eq_mav_output_payload(f, payload, headers)` | Remaining 1558 payload bytes match | RX |
| `input_eq_mav_output(f, output)` | Headers and payload both match | RX |
| `mav_input_headers_eq_output(headers, f)` | Headers from UdpFrame match reassembled frame | Mavlink |
| `mav_input_payload_eq_output(payload, headers, f)` | Payload from UdpFrame matches reassembled frame | Mavlink |
| `mav_input_eq_output(input, f)` | Complete UdpFrame matches reassembled RawEthernetMessage | Mavlink |

### Mavlink Firewall Uninterpreted Functions

The MavlinkFirewall thread defines two `@spec def` functions in its GUMBO block (`SW.sysml`, lines 460-465):

```gumbo
@spec def msg_is_wellformed(msg: UdpPayload): Base_Types::Boolean;
@spec def msg_is_mav_cmd_flash_bootloader(msg: UdpPayload): Base_Types::Boolean;

def msg_is_blacklisted(msg: UdpPayload): Base_Types::Boolean :=
    msg_is_mav_cmd_flash_bootloader(msg);
```

These are **uninterpreted** -- GUMBO does not define what "wellformed" or "flash bootloader" means. The developer supplies the semantics in the Rust implementation via Verus spec functions that delegate to the Vest-generated parser. See [06-mavlink-firewall.md](06-mavlink-firewall.md) for details.

## GumboLib Rust Realization

HAMR generates a Rust crate (`hamr/microkit/crates/GumboLib/src/lib.rs`) that realizes the GumboLib predicates in two forms:

1. **Executable functions** (e.g., `valid_arp()`) -- Used at runtime for GUMBOX contract checking in tests.
2. **Verus spec functions** (e.g., `valid_arp_spec()`) -- Used in Verus `requires`/`ensures` clauses for compile-time verification.

Both forms are auto-generated from the GUMBO definitions in `GumboLib.sysml`. The spec functions carry the `_spec` suffix and are defined inside `verus!` blocks with `pub open spec fn`.

The Verus `ensures` clauses on component entry points (e.g., `timeTriggered`) reference the `_spec` variants. For example, the TX Firewall's guarantee that valid ARP frames are forwarded appears as:

```rust
// guarantee hlr_07_tx0_can_send_valid_arp
api.EthernetFramesTxIn0.is_some() && GumboLib::valid_arp_spec(api.EthernetFramesTxIn0.unwrap()) ==>
  api.EthernetFramesTxOut0.is_some() && ...
```

This correspondence between GUMBO-level predicates and Verus-level spec functions is the foundation of the assurance argument: the model-level contracts are mechanically translated into code-level verification conditions.
