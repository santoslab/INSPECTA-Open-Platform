# ArduPilot Firewall System Architecture

This document describes the architecture and message flow of the ArduPilot firewall system as implemented in the `microkit/` (final) project. The system runs on the seL4 verified microkernel using the Microkit framework, with five components isolated in separate protection domains.

## ArduPilot

ArduPilot is a widely used open-source autopilot software suite for drones and other unmanned vehicles. The name comes from its origins on the Arduino hardware platform. ArduPilot runs directly on the drone as the flight controller firmware -- it reads sensors, controls motors and servos, executes flight plans, and manages all aspects of vehicle operation. It communicates with a remote ground control station (GCS) over the network using the MAVLink protocol, allowing a human operator to monitor telemetry, send commands, and upload waypoints.

In the production configuration of this system, ArduPilot runs inside a Virtual Machine Monitor (VMM) within its own seL4 protection domain. The VMM hosts the unmodified open-source ArduPilot software, isolating it from the rest of the system while allowing it to send and receive network traffic through the firewall pipeline. The network traffic flowing through the firewalls is between the ground control station and ArduPilot on the drone. The VMM uses ARM SMC (Secure Monitor Call) forwarding to manage the boundary between ArduPilot's virtual environment and the seL4 microkernel.

**Note:** In this configuration, ArduPilot is mocked rather than running under the VMM. QEMU for ARM does not support ARM SMC forwarding, so the model uses `ArduPilot_MOCK` instead of the `ArduPilot` variant that would use VMM integration. The mocked version has the same port interfaces as the VMM-hosted version, so the firewall components and their contracts are identical in both configurations.

## System Overview

The system provides a defense-in-depth network firewall for the ArduPilot autopilot controller. All Ethernet traffic entering or leaving ArduPilot passes through multiple independent firewalls, each operating at a different protocol layer. The firewalls are formally specified with GUMBO contracts and verified with Verus, providing high assurance that the filtering logic is correct.

## Components

The system has five components, each running in its own seL4 protection domain:

| Component | Domain | Role |
|-----------|--------|------|
| ArduPilot | 2 | Autopilot application (mocked) |
| TxFirewall | 3 | Outbound packet validation |
| LowLevelEthernetDriver | 4 | Hardware DMA interface |
| RxFirewall | 5 | Inbound packet classification and routing |
| MavlinkFirewall | 6 | Application-layer MAVLink command filtering |

All components are periodic (1000 ms logical period) and communicate through event data ports. Each port connection is replicated four times (ports 0--3), allowing up to four messages per scheduling cycle.

## Data Types

Messages flow through the system in several representations:

- **`RawEthernetMessage`** -- A 1600-byte unsigned-8 array. The universal wire format for full Ethernet frames.
- **`UdpFrame_Impl`** -- A struct splitting the frame into `headers` (42-byte `EthIpUdpHeaders` array covering Ethernet + IP + UDP headers) and `payload` (1558-byte `UdpPayload` array). Used to carry MAVLink-bearing UDP frames between RxFirewall and MavlinkFirewall so the firewall can inspect the payload without re-parsing headers.
- **`SizedEthernetMessage_Impl`** -- A struct pairing a `RawEthernetMessage` with a `u16` size field. Used on the transmit path so the Ethernet driver knows the actual frame length to send via DMA.

## Message Flow

### Inbound Path (Network to ArduPilot)

```
                                    +---> VmmOut -------> ArduPilot.FirewallRx
Network --> EthernetDriver.Rx ----> RxFirewall
                                    +---> MavlinkOut --> MavlinkFirewall --> ArduPilot.MavlinkRx
```

1. **LowLevelEthernetDriver** receives raw Ethernet frames from the network via memory-mapped DMA. Each frame is placed on one of four output ports (`EthernetFramesRx0`--`Rx3`) as a `RawEthernetMessage`.

2. **RxFirewall** reads each incoming `RawEthernetMessage` and classifies it using `firewall_core::EthFrame::parse()`, which performs structured parsing of the Ethernet, IP, and transport-layer headers. Based on the parse result, RxFirewall makes one of three routing decisions:

   - **ARP frames** or **whitelisted UDP** (destination port on the allowed list, currently port 68/DHCP): forwarded unchanged to ArduPilot via the `VmmOut` ports.
   - **MAVLink UDP** (source port 14550, destination port 14562): the raw frame is split into a `UdpFrame_Impl` (headers and payload separated) and forwarded to MavlinkFirewall via the `MavlinkOut` ports.
   - **Everything else** (unrecognized protocols, non-whitelisted ports, malformed frames): silently dropped.

   For each input port slot, the frame goes to exactly one destination or is dropped -- never both `VmmOut` and `MavlinkOut` for the same slot.

3. **MavlinkFirewall** receives `UdpFrame_Impl` messages and inspects the payload using the Vest-based MAVLink parser. It applies application-layer filtering:

   - **Well-formed, non-blacklisted MAVLink**: the `UdpFrame_Impl` is reassembled into a `RawEthernetMessage` (headers concatenated with payload) and forwarded to ArduPilot via the `Out` ports (which connect to `ArduPilot.MavlinkRx`).
   - **Blacklisted commands** (currently `MAV_CMD_FLASH_BOOTLOADER` in either MAVLink v1 `CommandInt`/`CommandLong` or v2 equivalents): dropped.
   - **Malformed MAVLink** (fails to parse): dropped.

4. **ArduPilot** receives network traffic on two sets of input ports: `FirewallRx0`--`Rx3` for general traffic (ARP, whitelisted UDP) and `MavlinkRx0`--`Rx3` for MAVLink traffic that has passed application-layer inspection.

### Outbound Path (ArduPilot to Network)

```
ArduPilot.Tx --> TxFirewall --> EthernetDriver.Tx --> Network
```

1. **ArduPilot** places outbound `RawEthernetMessage` frames on its `EthernetFramesTx0`--`Tx3` ports.

2. **TxFirewall** reads each frame and validates it using `firewall_core::EthFrame::parse()`:

   - **Valid ARP**: wrapped in a `SizedEthernetMessage_Impl` with `sz = 64` (minimum Ethernet frame size) and forwarded.
   - **Valid IPv4**: wrapped with `sz = IP total length + 14` (Ethernet header size) and forwarded.
   - **Invalid** (IPv6, malformed, etc.): dropped.

   The size field is critical: it tells the Ethernet driver how many bytes of the 1600-byte buffer actually contain frame data.

3. **LowLevelEthernetDriver** reads each `SizedEthernetMessage_Impl` from its `EthernetFramesTx0`--`Tx3` input ports and transmits the first `sz` bytes of `amessage` via DMA to the physical network interface.

## Why Three Firewalls

The three firewalls implement defense in depth, with each operating at a different layer of the protocol stack:

### RxFirewall -- Layer 2/3/4 Classification

RxFirewall performs structural classification of inbound traffic. It parses Ethernet headers to identify the frame type, then IP and transport headers to identify protocols and port numbers. Its decisions are based purely on protocol structure and port whitelists:

- Is this a valid ARP frame?
- Is this an IPv4/UDP packet on a whitelisted port?
- Is this a MAVLink-bearing UDP packet (specific source/destination ports)?

Frames that do not match any allowed category are dropped before reaching ArduPilot. MAVLink traffic is separated onto a dedicated path for deeper inspection.

### MavlinkFirewall -- Layer 7 Application Inspection

MavlinkFirewall performs application-layer inspection of MAVLink messages. While RxFirewall only checks that a UDP packet is addressed to the MAVLink port, MavlinkFirewall actually parses the MAVLink protocol payload and inspects message content:

- Is the MAVLink message well-formed (valid framing, known version)?
- Does it carry a blacklisted command (e.g., `FlashBootloader`, which could be used to overwrite ArduPilot firmware)?

This separation means that even if an attacker crafts a UDP packet addressed to the MAVLink port, the packet's MAVLink content is scrutinized before it can reach ArduPilot. The firewall is extensible -- additional commands can be added to the blacklist.

### TxFirewall -- Layer 2/3 Outbound Validation

TxFirewall validates outbound traffic from ArduPilot before it reaches the physical network. This prevents a compromised or misbehaving ArduPilot from:

- Sending malformed Ethernet frames
- Sending protocols other than ARP and IPv4
- Sending frames with incorrect size metadata

It also computes the actual frame size from protocol headers, ensuring the Ethernet driver transmits only meaningful bytes rather than the full 1600-byte buffer.

### Why Not One Firewall?

Separation into three components provides several benefits:

1. **Isolation**: Each firewall runs in its own seL4 protection domain. A vulnerability in the MAVLink parser (which is the most complex parser) cannot compromise the L2/L3 filtering logic.

2. **Simpler verification**: Each component's GUMBO contracts and Verus proofs are scoped to a single concern. The RxFirewall's contracts reason about Ethernet/IP/UDP structure; the MavlinkFirewall's contracts reason about MAVLink message format and command semantics.

3. **Different trust boundaries**: RxFirewall trusts nothing from the network. MavlinkFirewall trusts that RxFirewall has already validated the Ethernet/IP/UDP structure (the `UdpFrame_Impl` carries pre-split headers and payload). TxFirewall trusts nothing from ArduPilot.

## MAVLink and Vest

### What Is MAVLink

MAVLink (Micro Air Vehicle Link) is a lightweight binary protocol for communicating between drones (or other unmanned vehicles) and ground control stations. It carries telemetry data, commands, waypoints, and configuration messages. MAVLink messages are typically transported over UDP and come in two versions:

- **MAVLink v1**: Fixed-length header with an 8-bit message ID, supporting up to 256 message types.
- **MAVLink v2**: Extended header with a 24-bit message ID, supporting over 16 million message types, plus message signing and other features.

In this system, the ground control station sends MAVLink messages to ArduPilot over UDP (source port 14550, destination port 14562). Among the hundreds of MAVLink commands, some are safety-critical -- for example, `MAV_CMD_FLASH_BOOTLOADER` (command 42650) instructs the autopilot to reflash its firmware. The MavlinkFirewall exists specifically to intercept and block such commands.

### Why Vest

Vest is a verified parser combinator framework for Rust. It generates both a **specification-level** parser (for reasoning in Verus proofs) and an **executable** parser (for runtime use) from a single grammar definition. Vest proves that the executable parser's behavior matches the specification parser.

This dual-level approach is essential for the MavlinkFirewall's verification story:

- **Spec-level parser** (`spec_mavlink_msg()`): Used in Verus `spec` functions to reason about what a byte sequence *means* as a MAVLink message. The GUMBO-derived spec function `msg_is_wellformed__developer_verus` is defined as `spec_mavlink_msg().spec_parse(payload@).is_some()` -- a payload is well-formed if the spec parser successfully parses it.

- **Exec-level parser** (`parse_mavlink_msg()`): Used at runtime to actually parse the payload. Vest guarantees that if the exec parser returns `Ok((_, msg))`, then `spec_mavlink_msg().spec_parse(payload@) == Some((_, msg@))`.

Without Vest, the developer would need to hand-write both a spec-level description of "valid MAVLink" and a runtime parser, then manually prove they agree. Vest automates this correspondence.

### The MAVLink Grammar

The MAVLink grammar is defined in `mavlink.vest` (~190 lines), a Vest grammar file that specifies the binary layout of MAVLink v1 and v2 messages:

```
mavlink_msg = {
    msg: choose {
        MavLink1 = mavlink1_msg,
        MavLink2 = mavlink2_msg,
    }
}
```

Each variant defines the header fields (start byte, payload length, sequence number, system/component IDs, message ID), a variable-length payload, and checksums. Vest compiles this grammar into Rust code that provides both spec and exec parsers.

## Parsing: Structured vs. Byte-Level

The system uses two complementary parsing approaches, connected by formal proofs.

### GumboLib: Byte-Level Specification Functions

GumboLib contains spec functions that reason directly about raw byte arrays. These are the ground truth for GUMBO contracts. For example, `valid_arp_spec(frame)` checks:

- Bytes 12--13 (ethertype) == `0x0806`
- Bytes 14--15 (hardware type) == `0x0001`
- Bytes 16--17 (protocol type) == `0x0800`
- Byte 18 (hardware address length) == `6`
- Byte 19 (protocol address length) == `4`

Similarly, `valid_ipv4_udp_mavlink_spec(frame)` checks the ethertype indicates IPv4, the IP protocol field indicates UDP, and the UDP source/destination ports match the MAVLink port pair. These specs are auto-generated by HAMR and express the component interface contracts at the byte level.

### firewall_core: Structured Parsing with Proofs

The `firewall_core` crate provides `EthFrame::parse()`, which takes a `RawEthernetMessage` reference and returns an `Option<EthFrame>` containing structured types (`PacketType::Arp`, `PacketType::Ipv4`, etc.) with parsed header fields. This is the parsing code that actually runs at runtime.

The key innovation is that `EthFrame::parse()` carries Verus postconditions that **bridge** the structured parse result back to the byte-level specs:

```rust
fn get_frame_packet(frame: &RawEthernetMessage) -> Option<EthFrame>
    ensures
        valid_arp_spec(*frame) == firewall_core::res_is_arp(result),
        valid_ipv4_udp_spec(*frame) == firewall_core::res_is_udp(result),
        valid_ipv4_tcp_spec(*frame) == firewall_core::res_is_tcp(result),
        // ...
```

This means: "the frame satisfies the byte-level ARP spec if and only if the parser returned an ARP result." The developer writes application logic against the structured types (pattern matching on `PacketType::Arp`, checking `udp.dst_port`, etc.), and Verus uses the postconditions to prove the application logic satisfies the byte-level GUMBO contracts.

### Vest Parsing: Verified Grammar-Based Parsing

For MAVLink, the parsing challenge is more complex: the protocol has variable-length payloads, two version formats, and checksums. Rather than hand-writing a parser and proving it correct against byte-level specs, the MavlinkFirewall uses Vest.

The GUMBO contracts define two `@spec` functions for the MavlinkFirewall: `msg_is_wellformed` and `msg_is_mav_cmd_flash_bootloader`. In the final implementation, the developer supplies Verus definitions for these:

- `msg_is_wellformed__developer_verus(payload)` = `spec_mavlink_msg().spec_parse(payload@).is_some()`
- `msg_is_mav_cmd_flash_bootloader__developer_verus(payload)` = parse with spec parser, then check if the result is a `CommandInt` or `CommandLong` with `FlashBootloader` command ID

At runtime, `parse_mavlink_msg(&payload)` (the Vest exec parser) performs the actual parsing. Vest's guarantee that exec matches spec means the runtime code inherits the spec-level reasoning for free.

### How the Three Parsing Approaches Connect

```
Layer 2-4 (RxFirewall, TxFirewall):
  GUMBO contracts (byte-level GumboLib specs)
       ↕  proved equivalent by firewall_core postconditions
  firewall_core structured parsing (runtime)

Layer 7 (MavlinkFirewall):
  GUMBO contracts (byte-level GumboLib specs)
       ↕  developer-supplied spec functions
  Vest spec parser (spec-level MAVLink grammar)
       ↕  proved equivalent by Vest framework
  Vest exec parser (runtime)
```

At layers 2--4, `firewall_core` directly bridges between byte-level specs and structured types. At layer 7, Vest provides an additional layer: the developer links GUMBO specs to the Vest spec parser, and Vest links the spec parser to the exec parser. In both cases, the end result is the same -- runtime code that is formally proven to satisfy byte-level interface contracts.

### Manual Byte-Array Manipulation

Despite using structured parsers for classification, some operations still require manual byte-array copying with loop invariants:

- **`udp_frame_from_raw_eth`** (RxFirewall): Splits a 1600-byte `RawEthernetMessage` into a `UdpFrame_Impl` by copying bytes 0--41 into `headers` and bytes 42--1599 into `payload`. Loop invariants prove the copy preserves byte values.

- **`raw_eth_from_udp_frame`** (MavlinkFirewall): Reassembles a `UdpFrame_Impl` back into a `RawEthernetMessage` by concatenating `headers` and `payload`. The loop invariant proves `GumboLib::mav_input_eq_output_spec(input, output)` -- that the reassembled frame is byte-identical to the original decomposition.

These operations are necessary because the HAMR port types are fixed-size arrays, and converting between representations requires element-by-element copying with formal proof that the transformation is faithful.

## Scheduling

The system uses a static cyclic schedule with a 1950 ms hyper-period. Each component gets dedicated time slots, interleaved with 30 ms pacer slots:

| Slot | Component | Duration |
|------|-----------|----------|
| Pacer | (scheduling tick) | 30 ms |
| ArduPilot | Domain 2 | 600 ms |
| Pacer | | 30 ms |
| TxFirewall | Domain 3 | 300 ms |
| Pacer | | 30 ms |
| EthernetDriver | Domain 4 | 300 ms |
| Pacer | | 30 ms |
| RxFirewall | Domain 5 | 300 ms |
| Pacer | | 30 ms |
| MavlinkFirewall | Domain 6 | 300 ms |

The ordering reflects the data flow: ArduPilot runs first (producing outbound traffic), then TxFirewall validates it, then the Ethernet driver sends the validated outbound frames and receives new inbound frames, then RxFirewall classifies inbound traffic, then MavlinkFirewall inspects MAVLink messages. By the next cycle, ArduPilot has fresh validated input available.
