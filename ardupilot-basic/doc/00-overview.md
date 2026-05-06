# ArduPilot-Basic System Overview

## Purpose

The ardupilot-basic system demonstrates how the INSPECTA tool chain secures a legacy UAV autonomy application (ArduPilot) by mediating all network traffic between the UAV and external ground control stations. Three firewall components -- RX Firewall, TX Firewall, and Mavlink Firewall -- enforce security policies on ethernet frames flowing in and out of the ArduPilot virtual machine. Each firewall runs in its own seL4 protection domain, providing spatial isolation enforced by the verified seL4 microkernel.

## System Architecture

The system uses the seL4 Microkit framework to partition components into isolated protection domains. All inter-component communication occurs through HAMR-generated message-passing infrastructure over shared memory regions.

```
                        Inbound (from network)
                        =====================

  Ground Control         LowLevel              RX                ArduPilot
  Station (GCS)   --->  EthernetDriver  --->  Firewall  ------>  VM
                        (Domain 4)            (Domain 5)   |    (Domain 2)
                                                           |
                                                           |    Mavlink
                                                           +--> Firewall ---> ArduPilot VM
                                                                (Domain 6)

                        Outbound (to network)
                        =====================

  ArduPilot             TX                    LowLevel          Ground Control
  VM           --->   Firewall   --->       EthernetDriver ---> Station (GCS)
  (Domain 2)          (Domain 3)            (Domain 4)
```

### Data Flow

**Inbound path (network to UAV):**
1. The **LowLevelEthernetDriver** receives raw ethernet frames from the physical interface and forwards them to the RX Firewall.
2. The **RX Firewall** classifies each frame:
   - **ARP frames** and **allowed UDP port frames** are forwarded directly to the ArduPilot VM.
   - **MAVLink UDP frames** (identified by source port 14550 and destination port 14562) are split into headers and payload, then forwarded to the Mavlink Firewall for deep packet inspection.
   - **All other frames** are dropped.
3. The **Mavlink Firewall** parses the UDP payload as a MAVLink message:
   - **Malformed messages** are dropped.
   - **Blacklisted commands** (currently `MAV_CMD_FLASH_BOOTLOADER`) are dropped.
   - **Well-formed, non-blacklisted messages** are reassembled into a raw ethernet frame and forwarded to the ArduPilot VM.

**Outbound path (UAV to network):**
1. The **ArduPilot VM** sends raw ethernet frames to the TX Firewall.
2. The **TX Firewall** validates each frame:
   - **Well-formed ARP frames** are forwarded with a fixed size of 64 bytes.
   - **Well-formed IPv4 frames** are forwarded with the size computed from the IPv4 total length field plus the 14-byte ethernet header.
   - **All other frames** are dropped.
3. The **LowLevelEthernetDriver** transmits the validated, sized frames onto the physical network.

### seL4 Protection Domains

| Domain | Component | Role |
|--------|-----------|------|
| 2 | ArduPilot | Legacy UAV application running in a virtual machine |
| 3 | TxFirewall | Validates outbound frames from the ArduPilot VM |
| 4 | LowLevelEthernetDriver | Interfaces with the physical ethernet hardware |
| 5 | RxFirewall | Classifies and routes inbound frames |
| 6 | MavlinkFirewall | Deep inspection of MAVLink messages within UDP payloads |

### 4-Port Replication

Each component has 4 replicated port sets (ports 0 through 3), supporting up to 4 independent ethernet channels. The firewall logic is identical across all 4 channels; the GUMBO contracts and implementation code are replicated per channel. In this documentation, we present the logic once (typically for port 0) and note that ports 1-3 follow the same pattern.

## Data Types

The following data types are defined in `SW.sysml` and used throughout the firewall components:

| Type | Structure | Size | Description |
|------|-----------|------|-------------|
| `RawEthernetMessage` | `[u8; 1600]` | 1600 bytes | Fixed-size buffer holding a raw ethernet frame. The 1600-byte size accommodates standard MTU frames with padding. |
| `EthIpUdpHeaders` | `[u8; 42]` | 42 bytes | The first 42 bytes of a UDP-over-IPv4 ethernet frame: 14 bytes ethernet header + 20 bytes IPv4 header + 8 bytes UDP header. |
| `UdpPayload` | `[u8; 1558]` | 1558 bytes | The remaining bytes of a `RawEthernetMessage` after removing the 42-byte `EthIpUdpHeaders`. |
| `UdpFrame_Impl` | `{ headers: EthIpUdpHeaders, payload: UdpPayload }` | 1600 bytes | A raw ethernet frame split into protocol headers and UDP payload. Used to pass MAVLink data from RX Firewall to Mavlink Firewall. |
| `SizedEthernetMessage_Impl` | `{ amessage: RawEthernetMessage, sz: u16 }` | 1602 bytes | A raw ethernet frame paired with its actual message size. Used by TX Firewall to communicate valid frame length to the ethernet driver. |

## Requirements

The formal requirements for all three firewall components are documented in [`requirements/Inspecta-HLRs.pdf`](../requirements/Inspecta-HLRs.pdf). The requirements define byte-level checks on ethernet frames and MAVLink messages that each firewall must enforce.

## Document Roadmap

| Document | Description |
|----------|-------------|
| [01 - Ethernet Frame Formats](01-ethernet-frame-formats.md) | Background on Ethernet II, ARP, IPv4, UDP, and TCP frame layouts. Covers only the fields checked by the firewalls. |
| [02 - MAVLink Message Format](02-mavlink-message-format.md) | Background on MAVLink v1 and v2 packet formats. Includes a walkthrough of the Vest parser specification. |
| [03 - SysMLv2 Model and GUMBO Contracts](03-sysmlv2-model-and-gumbo-contracts.md) | The SysMLv2 architecture model, the GUMBO contract language, and the GumboLib predicate library used across all firewall components. |
| [04 - RX Firewall](04-rx-firewall.md) | Requirements, GUMBO contracts, and implementation walkthrough for the RX Firewall. |
| [05 - TX Firewall](05-tx-firewall.md) | Requirements, GUMBO contracts, and implementation walkthrough for the TX Firewall. |
| [06 - Mavlink Firewall](06-mavlink-firewall.md) | Requirements, GUMBO contracts, Vest specification, and implementation walkthrough for the Mavlink Firewall. |

## Key Source Files

| File | Role |
|------|------|
| `SW.sysml` | System architecture model: data types, thread definitions, process definitions, connections, GUMBO contracts |
| `GumboLib.sysml` | GUMBO library: reusable predicate functions for frame validation, port checking, and output equality |
| `Platform.sysml` | Processor definition and system-level binding |
| `hamr/microkit/crates/firewall_core/` | Hand-built Rust library for parsing ethernet, ARP, IPv4, TCP, and UDP headers with Verus verification |
| `hamr/microkit/crates/mavlink_parser_vest/` | Vest-generated MAVLink parser with verified combinators |
| `hamr/microkit/crates/GumboLib/` | HAMR-generated Rust realization of GumboLib predicates (executable + Verus spec functions) |
| `hamr/microkit/crates/seL4_RxFirewall_RxFirewall/` | RX Firewall component crate |
| `hamr/microkit/crates/seL4_TxFirewall_TxFirewall/` | TX Firewall component crate |
| `hamr/microkit/crates/seL4_MavlinkFirewall_MavlinkFirewall/` | Mavlink Firewall component crate |
