# Documentation Summary: ArduPilot-Basic Firewall Components

**Date:** 2026-04-11

## What Was Done

Seven markdown documentation files were created in `ardupilot-basic/doc/` to provide background and traceability documentation for the three firewall components in the ardupilot-basic system. The documentation covers the full chain from network protocol background through formal requirements, SysMLv2 models with GUMBO contracts, to verified Rust implementations.

## Files Created

### Background Documents

| File | Description |
|------|-------------|
| `00-overview.md` | System architecture, data flow diagram, seL4 protection domain assignments, data types table, and document roadmap |
| `01-ethernet-frame-formats.md` | Ethernet II, ARP, IPv4, UDP, and TCP frame layouts with byte-offset tables. Covers only the fields that the firewalls inspect. Includes a consolidated byte-offset reference table and cross-references to GumboLib predicates |
| `02-mavlink-message-format.md` | MAVLink v1 and v2 packet formats, relevant message IDs, the MAV_CMD_FLASH_BOOTLOADER blacklist, and a complete walkthrough of the Vest parser specification (`mavlink.vest`) including DSL syntax explanation |

### Model and Specification Documents

| File | Description |
|------|-------------|
| `03-sysmlv2-model-and-gumbo-contracts.md` | SysMLv2 model structure (threads, processes, ports, connections), GUMBO contract language primer, and a comprehensive catalog of all GumboLib predicates organized by category (ethernet, ARP, IPv4, UDP/TCP, MAVLink, composite decisions, output equality, size validation) |

### Component Documents

| File | Description |
|------|-------------|
| `04-rx-firewall.md` | RX Firewall: HLR-5/13/18/15/17 requirements, GUMBO integration assumes and compute guarantees, `firewall_core` library usage, application code walkthrough (routing logic, frame splitting), traceability matrix |
| `05-tx-firewall.md` | TX Firewall: HLR-7/12/14/16 requirements, GUMBO integration and compute guarantees, `firewall_core` usage, application code walkthrough (frame parsing, size computation), traceability matrix |
| `06-mavlink-firewall.md` | Mavlink Firewall: HLR-19/20/21/22 requirements, uninterpreted GUMBO `@spec def` functions, developer-supplied Verus spec functions linking to Vest-generated combinators, application code walkthrough (parsing, blacklist check, payload command extraction), Vest cross-reference, traceability matrix |

## Key Design Decisions

1. **Background separated from component docs.** Ethernet and MAVLink format details are documented once in their own files, avoiding repetition across the three component documents.

2. **4-port replication presented once.** Each component has 4 identical port sets (ports 0-3). The GUMBO contracts and code are shown for port 0, with a note that ports 1-3 are identical.

3. **Traceability matrices in every component doc.** Each component document ends with a table mapping: HLR requirement -> GUMBO guarantee name -> Verus `ensures` clause -> runtime code path.

4. **Vest specification documented, not generated code.** For the Mavlink Firewall, the `.vest` file is documented as the specification of record. The auto-generated Rust parser code is not documented inline.

5. **`payload_get_cmd` workaround called out.** The manual extraction of the MAVLink command field at payload bytes 28-29 (due to Vest's current inability to handle dependent payload types with MAVLink v2 truncation) is documented transparently.

## Source Files Referenced

All documentation was derived from the following source files:

| File | Referenced In |
|------|--------------|
| `SW.sysml` | All documents (data types, thread defs, GUMBO contracts) |
| `GumboLib.sysml` | 01 (byte checks), 03 (predicate catalog), 04/05/06 (contracts) |
| `Platform.sysml` | 00 (processor config) |
| `requirements/Inspecta-HLRs.pdf` | 04, 05, 06 (HLR requirements) |
| `hamr/microkit/crates/firewall_core/src/lib.rs` | 04, 05 (ethereum parsing library) |
| `hamr/microkit/crates/firewall_core/src/net.rs` | 04, 05 (protocol type definitions) |
| `hamr/microkit/crates/mavlink_parser_vest/src/mavlink.vest` | 02, 06 (Vest specification) |
| `hamr/microkit/crates/seL4_RxFirewall_RxFirewall/src/component/*_app.rs` | 04 (RX Firewall implementation) |
| `hamr/microkit/crates/seL4_TxFirewall_TxFirewall/src/component/*_app.rs` | 05 (TX Firewall implementation) |
| `hamr/microkit/crates/seL4_MavlinkFirewall_MavlinkFirewall/src/component/*_app.rs` | 06 (Mavlink Firewall implementation) |
| `hamr/microkit/crates/GumboLib/src/lib.rs` | 03 (GumboLib Rust realization) |

## Notable Observations

- **TCP routing is inactive.** The GumboLib predicates and GUMBO contracts define TCP port checking (`tcp_is_valid_port`, `frame_has_ipv4_tcp_on_allowed_port_quant`), and the RX Firewall has `TCP_ALLOWED_PORTS = [5760]` defined. However, the TCP routing path is commented out in the RX Firewall implementation. This is noted in the documentation.

- **Vest payload limitation.** The Vest specification parses MAVLink payloads as raw byte arrays rather than structured types (command_long, command_int) because MAVLink v2 truncates trailing zero bytes in payloads. This requires the `payload_get_cmd` workaround to manually extract the command field.

- **Integration contract chain.** The LowLevelEthernetDriver's output guarantees become the RX Firewall's input assumptions, and the TX Firewall's output guarantees become the LowLevelEthernetDriver's input assumptions. This compositional reasoning chain is documented in the GUMBO contracts.

## Next Steps

Potential additions to this documentation set:
- Walkthrough of the `firewall_core` library implementation (the hand-built ethernet/ARP/IPv4 parser with Verus proofs)
- Test documentation: how PropTest and GUMBOX contract-based tests are used for each firewall component
- CI/build documentation: how `make verus` and `make test` are used to verify the firewalls
