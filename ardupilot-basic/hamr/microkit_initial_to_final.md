# Comparison: `microkit_initial` to `microkit` (Final)

This document details what needs to be changed in `microkit_initial` to match the finished `microkit` project. The `microkit_initial` represents what HAMR code generation produces (with verus attribute syntax and a 30ms pacer slot). The `microkit` (final) version includes implemented behavior code, Vest-based MAVLink parsing integration, the `firewall_core` library, VMM integration for ArduPilot, and adjusted build infrastructure.

**Verus syntax note**: The initial version uses `#[verus_verify]` / `#[verus_spec(...)]` attribute syntax so that `cargo mutants` can instrument the behavior code (mutants cannot work with the `verus!` macro). The final version uses the `verus! { }` macro wrapper instead. This syntactic difference applies across all component `_app.rs` files and is not called out again per-component below.

## Overview of Differences

| Area | `microkit_initial` | `microkit` (final) |
|------|--------------------|--------------------|
| Extra crates | — | `firewall_core`, `mavlink_parser_vest` |
| Extra top-level dirs | — | `vmm/`, `src/`, `include/` |
| Extra top-level files | — | `custom.mk` |
| ArduPilot component | Skeleton app code (mocked) | VMM-backed Linux guest |
| LowLevelEthernetDriver | Skeleton app code | Full Ethernet driver using `eth-driver-core`, `smoltcp`, seL4 driver interfaces |
| RxFirewall | Skeleton app code | Full Ethernet frame parser/router using `firewall_core` |
| TxFirewall | Skeleton app code | Full Ethernet frame parser/forwarder using `firewall_core` |
| MavlinkFirewall | Skeleton app code | Full MAVLink parser using `mavlink_parser_vest` (Vest) |
| Schedule | Same structure | Same (comments stripped in final) |
| `microkit.system` | No VMM/hardware regions | VMM, guest RAM, GEM MMIO, DMA memory regions |
| GUMBO spec functions | Scaffold defaults in `_app.rs` | Strengthened developer Verus specs in `_app.rs` |
| Test files | Identical GUMBOX PropTest scaffolding | Same scaffolding (no additional manual tests added) |

---

## 1. ArduPilot Component (Mocked)

The ArduPilot component (`seL4_ArduPilot_ArduPilot`) is present in the model and both projects have its C bridge code under `components/`. In the final version it hosts a VMM running a Linux guest with ArduPilot. **For this exercise, the ArduPilot will be mocked** — QEMU ARM does not support ARM SMC forwarding, so rather than using a VMM we will mock ArduPilot behavior.

In the final project the ArduPilot protection domain in `microkit.system` has:
- `smc="true"` attribute on the protection domain
- A `<virtual_machine>` block with a vcpu and guest RAM/serial/MMC/GIC mappings
- IRQ entries for UART (53) and MMC (81)
- Memory regions: `guest_ram` (1GB at `0x8_0000_0000`), `serial`, `mmc`, `gic_vcpu`
- The `vmm/` directory containing the C-based VMM implementation

**For the mock version**: None of the VMM infrastructure needs to be added. Instead, implement basic loopback or static behavior in the ArduPilot component's `_app.rs` so the Tx ports produce frames for the pipeline to process.

---

## 2. New Crate: `firewall_core`

**Path**: `crates/firewall_core/`

A no_std Rust+Verus crate (~1,469 lines) providing Ethernet frame parsing with Verus-verified postconditions. Used by both `seL4_RxFirewall_RxFirewall` and `seL4_TxFirewall_TxFirewall`.

**Contents**:
- `src/lib.rs` — Top-level types: `PacketType` (Arp/Ipv4/Ipv6), `EthFrame`, `Ipv4Packet`, `Ipv4ProtoPacket`, plus the `EthFrame::parse()` function with verified postconditions that connect parsing results back to the GUMBO spec functions (`valid_arp_spec`, `valid_ipv4_spec`, etc.)
- `src/net.rs` — Low-level network type definitions: `EthernetRepr`, `Arp`, `Ipv4Repr`, `TcpRepr`, `UdpRepr`, `IpProtocol`, `EtherType`, `ArpOp`, etc. with byte-level parsing from raw frames

**Cargo.toml dependencies**: `vstd`, `verus_builtin`, `verus_builtin_macros`

**Why it exists**: The GUMBO spec functions (in `GumboLib`) reason about raw byte arrays. `firewall_core` bridges that gap — it parses raw Ethernet frames into structured types and its postconditions prove that the structured parsing matches the byte-level GUMBO specifications. This lets the app code work with structured types while still satisfying the byte-level GUMBO contracts.

---

## 3. New Crate: `mavlink_parser_vest`

**Path**: `crates/mavlink_parser_vest/`

A Rust crate (~2,211 lines) providing MAVLink v1/v2 message parsing using the Vest parser combinator framework with Verus verification.

**Contents**:
- `src/lib.rs` — Re-exports
- `src/mavlink.rs` — Verus-verified MAVLink parser (~2,015 lines): message structure types (`MavlinkMsg`, `MavlinkMsgMsg`), message ID enums (`MessageIdsV1`, `MessageIdsV2`), `MavCmd` enum (including `FlashBootloader`), and both spec-level (`spec_mavlink_msg`) and exec-level (`parse_mavlink_msg`) parsers
- `src/mavlink.vest` — Vest grammar definition (~190 lines)

**Cargo.toml dependencies**: `vstd`, `verus_builtin`, `verus_builtin_macros`, `vest_lib` (from dornerworks/vest git repo)

**Why it exists**: The MavlinkFirewall needs to parse MAVLink messages to determine if they contain blacklisted commands (specifically `MAV_CMD_FLASH_BOOTLOADER`). Vest provides verified parser combinators that produce both spec-level and exec-level parsers from a single grammar definition, enabling the Verus proofs.

---

## 4. Component: `seL4_LowLevelEthernetDriver_LowLevelEthernetDriver`

### 4a. Application Code (`_app.rs`)

The initial version has a standard skeleton. The final version is a **complete rewrite** that:

- Adds `eth-driver-core` sub-crate (`core/` directory within the component crate) for DMA-based Ethernet driver
- Uses `smoltcp` for network device abstraction (`Device`, `RxToken`, `TxToken`)
- Uses `sel4_driver_interfaces::HandleInterrupt` and `sel4_microkit_base::memory_region_symbol!` for hardware access
- Adds a `config.rs` submodule with DMA size constant and seL4 logging configuration
- The `new()` constructor initializes the hardware driver with memory-mapped DMA regions
- `initialize()` acknowledges the driver IRQ
- `timeTriggered()` loops over 4 message slots: receives Ethernet frames via DMA and puts them on Rx ports, reads Tx ports and transmits frames via DMA
- `notify()` is gated behind `#[cfg(feature = "sel4")]`

### 4b. Cargo.toml Changes

Add dependencies:
```toml
eth-driver-core = { path = "core" }
sel4-microkit-base = { git = "https://github.com/seL4/rust-sel4" }
sel4-driver-interfaces = { git = "https://github.com/seL4/rust-sel4" }
smoltcp = { version = "0.10.0", default-features = false, features = ["proto-ipv4", "medium-ethernet", "socket-raw"] }
```

Pin `vstd`/`verus_builtin`/`verus_builtin_macros` to `0.0.0-2026-01-11-0057` (remove `=` prefix from version specifiers).

### 4c. Config Submodule

Add `src/component/seL4_LowLevelEthernetDriver_LowLevelEthernetDriver_app/config.rs` with:
- DMA size constant (`DRIVER_DMA = 0x20_0000`)
- seL4 logging configuration (log level, `debug_print!` backend)

### 4d. `eth-driver-core` Sub-Crate

Add the `core/` directory within the component crate containing the low-level Ethernet DMA driver implementation.

---

## 5. Component: `seL4_RxFirewall_RxFirewall`

### 5a. Application Code (`_app.rs`)

The initial version has a skeleton. The final version:

- Adds `use firewall_core::{EthFrame, IpProtocol, ...}` and `use GumboLib::*`
- Adds helper functions with Verus contracts:
  - `udp_headers_from_raw_eth()` — extracts UDP headers from raw frame (with loop invariant proving byte equality)
  - `udp_payload_from_raw_eth()` — extracts UDP payload from raw frame (with loop invariant)
  - `udp_frame_from_raw_eth()` — combines headers + payload into `UdpFrame_Impl`
  - `port_allowed()` — linear search through allowed port list (with loop invariant)
  - `udp_port_allowed()` / `tcp_port_allowed()` — wrappers for config-specific port arrays
  - `can_send_to_vmm()` — determines if a packet should go to VMM (ARP or whitelisted UDP)
  - `can_send_to_mavlink()` — determines if a packet is MAVLink UDP (src:14550, dst:14562)
  - `get_frame_packet()` — parses raw frame via `firewall_core::EthFrame::parse()` with postconditions linking to GUMBO specs
- Spec functions: `packet_is_whitelisted_tcp`, `packet_is_whitelisted_udp`, `packet_is_mavlink_udp`, `ipv4_udp_on_allowed_port_quant`, `ipv4_tcp_on_allowed_port_quant`
- `timeTriggered()` implements the routing logic: for each of the 4 Rx input ports, parse the frame, route MAVLink UDP to `MavlinkOut`, route ARP/whitelisted UDP to `VmmOut`, drop everything else
- The GUMBO MARKER methods block is **commented out** — the app uses `GumboLib::*` spec functions directly instead of local copies
- Includes an inline `config` module with TCP/UDP allowed port arrays (also available as a separate file at `config.rs`)

### 5b. Cargo.toml Changes

Add dependency:
```toml
firewall-core = { path = "../firewall_core" }
```

Remove `=` prefix from `verus_builtin`/`verus_builtin_macros` version specifiers.

---

## 6. Component: `seL4_TxFirewall_TxFirewall`

### 6a. Application Code (`_app.rs`)

The initial version has a skeleton. The final version:

- Adds `use firewall_core::{...}` and `use GumboLib::*`
- Adds helper functions:
  - `can_send_packet()` — returns `Some(size)` for ARP (64 bytes) or IPv4 (header length + Ethernet size), `None` for IPv6
  - `get_frame_packet()` — parses raw frame via `firewall_core::EthFrame::parse()` with postconditions linking to GUMBO specs (`valid_arp_spec`, `valid_ipv4_spec`, `ipv4_length_bytes_match`)
- `timeTriggered()` implements: for each of the 4 Tx input ports, parse the frame, if valid ARP or IPv4 then wrap in `SizedEthernetMessage_Impl` with computed size and forward to output, otherwise drop
- Adds `mod config` pointing to a config file with TCP/UDP allowed port arrays
- The GUMBO MARKER methods block is **commented out** — uses `GumboLib::*` instead

### 6b. Cargo.toml Changes

Add dependency:
```toml
firewall-core = { path = "../firewall_core" }
```

---

## 7. Component: `seL4_MavlinkFirewall_MavlinkFirewall`

### 7a. Application Code (`_app.rs`)

The initial version has a skeleton with scaffold GUMBO spec function defaults. The final version:

- Adds `use mavlink_parser_vest::{parse_mavlink_msg, MavCmd, MavlinkMsg, ...}`
- Adds a heap allocator (required by Vest): `sel4-dlmalloc` with `one-shot-mutex`, 16KB static heap
- **Strengthens the developer Verus spec functions**:
  - `msg_is_wellformed__developer_verus` — now checks `spec_mavlink_msg().spec_parse(payload@).is_some()` (was `true`)
  - `msg_is_mav_cmd_flash_bootloader__developer_verus` — now checks parsed message for `FlashBootloader` command (was `true`)
- **Strengthens the developer GUMBOX exec functions**:
  - `msg_is_wellformed__developer_gumbox` — now calls `parse_mavlink_msg(&payload).is_ok()` (was `true`)
  - `msg_is_mav_cmd_flash_bootloader__developer_gumbox` — now calls `can_send(payload)` (was `true`)
- Adds exec helper functions:
  - `raw_eth_from_udp_frame()` — converts `UdpFrame_Impl` back to `RawEthernetMessage` (with loop invariant proving `mav_input_eq_output_spec`)
  - `can_send()` — parses MAVLink and returns true if wellformed and not blacklisted
  - `ex_msg_is_blacklisted()` / `msg_is_flash_bootloader()` — checks if parsed MAVLink message is `FlashBootloader`
  - `payload_get_cmd()` — extracts command field from CommandInt/CommandLong payload bytes
- Adds spec helper functions: `spec_msg_is_flash_bootloader`, `spec_msg_v1_is_flash_bootloader`, `spec_msg_v2_is_flash_bootloader`, `spec_payload_get_cmd`
- `timeTriggered()` implements: for each of the 4 input ports, check `can_send`, if allowed then reconstruct the raw Ethernet frame and forward to output

### 7b. Cargo.toml Changes

Pin `vstd`/`verus_builtin`/`verus_builtin_macros` to `0.0.0-2026-01-11-0057` version.

Add dependencies:
```toml
mavlink_parser_vest = { path = "../mavlink_parser_vest" }
one-shot-mutex = "0.2.1"
sel4-dlmalloc = { git = "https://github.com/seL4/rust-sel4" }
vest_lib = { git = "https://github.com/dornerworks/vest", rev = "open-platform-v2.0", default-features = false }
```

---

## 8. `microkit.system` Changes

The Microkit system description XML has the following differences for the final version:

1. **ArduPilot protection domain**: Add `smc="true"` attribute, `<virtual_machine>` block, guest RAM mapping, IRQ entries (for real VMM — skip for mock)
2. **LowLevelEthernetDriver protection domain**: Add memory-mapped hardware regions:
   - `gem_mmio` (GEM Ethernet controller registers at `0xFF0E_0000`)
   - `net_driver_dma` (DMA region at `0x8000_0000`, 2MB)
   - `setvar_vaddr` bindings for `gem_register_block` and `net_driver_dma_vaddr`/`net_driver_dma_paddr`
3. **Memory region declarations** at the bottom: Add `gem_mmio`, `net_driver_dma`, `guest_ram`, `serial`, `mmc`, `gic_vcpu`

**For the mock version**: Items 1 and 3 (VMM-related) are not needed. Item 2 (Ethernet hardware) would only be needed for real hardware deployment.

---

## 9. `microkit.schedule.xml`

The schedules are structurally identical. The only difference is that the initial version includes component-name comments (e.g., `<!-- pacer -->`, `<!-- seL4_ArduPilot_ArduPilot_MON -->`) which the final version strips. No functional change needed.

---

## 10. Build Infrastructure

### `custom.mk`

The initial version uses the auto-generated `system.mk`. The final version has a hand-edited `custom.mk` that includes:
- VMM build target and linking rules for `seL4_ArduPilot_ArduPilot.elf` (linking VMM object files)
- All the standard component build rules (same as `system.mk` but with VMM additions)

**For the mock version**: The standard `system.mk` should be sufficient. No `custom.mk` needed unless the ArduPilot mock requires special build steps.

### `src/` and `include/` directories

The final version moves shared C source and header files to top-level `src/` and `include/` directories (as opposed to the `types/` and `util/` layout in the initial version). This includes `printf.c`, queue implementations, and type headers. This appears to be a reorganization for the VMM build — the initial layout in `types/` and `util/` should work for the non-VMM case.

### `vmm/` directory

Contains the C-based VMM implementation (Makefile, vmm.c, vmm_config.h, virtio networking, board configs). **Not needed for the mock version.**

---

## 11. GumboLib

The `GumboLib` crate is **essentially identical** between the two projects (only trivial whitespace/comment differences). The GUMBO spec functions for the firewall rules (`valid_arp_spec`, `valid_ipv4_spec`, `tx_allow_outbound_frame_spec`, `rx_allow_outbound_frame_spec`, `input_eq_mav_output_spec`, `mav_input_eq_output_spec`, etc.) are auto-generated from the model and are the same in both.

---

## Summary: Steps to Go from Initial to Final

1. **Create `crates/firewall_core/`** — Ethernet frame parsing library with Verus postconditions linking to GumboLib specs
2. **Create `crates/mavlink_parser_vest/`** — MAVLink v1/v2 parser using Vest with Verus verification
3. **Implement `seL4_RxFirewall_RxFirewall_app.rs`** — Frame classification and routing logic using `firewall_core`, with verified helper functions for UDP frame extraction
4. **Implement `seL4_TxFirewall_TxFirewall_app.rs`** — Frame validation and size computation using `firewall_core`
5. **Implement `seL4_MavlinkFirewall_MavlinkFirewall_app.rs`** — MAVLink parsing and blacklist filtering using `mavlink_parser_vest`, strengthen GUMBO spec function implementations
6. **Mock `seL4_LowLevelEthernetDriver_LowLevelEthernetDriver_app.rs`** — Instead of real hardware driver, provide mock behavior for testing
7. **Mock `seL4_ArduPilot_ArduPilot`** — Instead of VMM with Linux guest, provide mock behavior that generates traffic for the firewall pipeline
8. **Update Cargo.toml files** — Add `firewall-core` dependency to RxFirewall and TxFirewall; add `mavlink_parser_vest`, `vest_lib`, `one-shot-mutex`, `sel4-dlmalloc` to MavlinkFirewall
9. **Add config modules** — `config.rs` files for RxFirewall and TxFirewall with allowed port arrays
