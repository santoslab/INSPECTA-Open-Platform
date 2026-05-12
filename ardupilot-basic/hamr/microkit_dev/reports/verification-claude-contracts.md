# RxFirewall Verus Verification Report

Verus formal verification of the RxFirewall component (`seL4_RxFirewall_RxFirewall`)
and its dependency `firewall_core`. The goal was to verify that the Rust
implementation satisfies the HAMR-generated GUMBO postconditions on the
`timeTriggered` entry point — 20 guarantees across 4 Rx port channels covering
ARP forwarding, MAVLink UDP routing, allowed-port UDP forwarding, disallow
filtering, and no-input passivity.

> **Branch note:** Robbie V's original Verus contracts are on the `main`
> branch. Claude's contracts described in this report are on the
> **`claude-verus-verification`** branch. To view Claude's versions of the
> contract files:
>
> ```
> git checkout claude-verus-verification
> ```
>
> The key files that differ between branches are:
> - `hamr/microkit_dev/crates/firewall_core/src/net.rs`
> - `hamr/microkit_dev/crates/firewall_core/src/lib.rs`
> - `hamr/microkit_dev/crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs`

## Table of Contents

- [Background](#background)
- [Setup: Stripping Existing Contracts](#setup-stripping-existing-contracts)
- [Phase 1: Initial Attempt — assume_specification](#phase-1-initial-attempt--assume_specification)
- [Phase 2: Contracts on firewall_core](#phase-2-contracts-on-firewall_core)
  - [net.rs — Network Type Parsers](#netrs--network-type-parsers)
  - [lib.rs — EthFrame::parse](#librs--ethframeparse)
- [Phase 3: Contracts on the App](#phase-3-contracts-on-the-app)
  - [Changes Required in firewall_core](#changes-required-in-firewall_core)
  - [Changes in seL4_RxFirewall_RxFirewall_app.rs](#changes-in-sel4_rxfirewall_rxfirewall_apprs)
  - [The hlr_15 Disallow Problem](#the-hlr_15-disallow-problem)
- [Verification Results](#verification-results)
- [Function Contract Summary](#function-contract-summary)
  - [firewall_core::net.rs](#firewall_corenetrs)
  - [firewall_core::lib.rs](#firewall_corelibrs)
  - [seL4_RxFirewall_RxFirewall_app.rs](#sel4_rxfirewall_rxfirewall_apprs-1)
- [Key Challenges and Lessons](#key-challenges-and-lessons)
- [Comparison: Robbie V's Contracts vs. Claude's Contracts](#comparison-robbie-vs-contracts-vs-claudes-contracts)
  - [Overall Architecture](#overall-architecture)
  - [Line Counts](#line-counts)
  - [net.rs Differences](#netrs-differences)
  - [lib.rs Differences](#librs-differences)
  - [app.rs Differences](#apprs-differences)
  - [Summary of Tradeoffs](#summary-of-tradeoffs)

---

## Background

The RxFirewall is a periodic HAMR thread component that inspects raw Ethernet
frames from 4 input ports and routes them to VMM or MAVLink output ports based
on protocol classification:

- **ARP frames** → forwarded to VmmOut (unchanged)
- **IPv4 UDP MAVLink** (src=14550, dst=14562) → forwarded to MavlinkOut (split into headers + payload)
- **IPv4 UDP on allowed port** (dst port in `[68]`) → forwarded to VmmOut (unchanged)
- **Everything else** → dropped silently

HAMR generates 20 `ensures` clauses on `timeTriggered` (5 per Rx channel)
from the GUMBO model-level contracts. These reference GumboLib spec functions
(`valid_arp_spec`, `valid_ipv4_udp_mavlink_spec`, `valid_ipv4_udp_port_spec`,
`rx_allow_outbound_frame_spec`) that operate on raw byte arrays. The Rust
implementation uses `firewall_core` to parse those bytes into structured types
(`EthFrame`, `PacketType`, `Arp`, `Ipv4Packet`, `UdpRepr`, etc.) and makes
routing decisions on the parsed structures.

The fundamental verification challenge is **bridging two abstraction levels**:
the GUMBO spec predicates reason about raw bytes (e.g., "byte 12 == 8 and
byte 13 == 6 means ARP"), while the implementation reasons about parsed Rust
enums (e.g., `PacketType::Arp(_)`). Every function in the call chain needs
contracts that connect these levels so Verus can verify the top-level
postconditions.

### Source Files

| File | Role |
|------|------|
| `crates/firewall_core/src/net.rs` | Network type parsers: `Address`, `EthernetRepr`, `Arp`, `Ipv4Repr`, `UdpRepr`, `TcpRepr`, enums (`EtherType`, `IpProtocol`, `ArpOp`, `HardwareType`) |
| `crates/firewall_core/src/lib.rs` | `EthFrame::parse` — top-level frame parser composing net.rs parsers; spec helper functions (`is_arp_packet`, `is_ipv4_udp_packet`, `get_udp_src_port`, etc.) |
| `crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs` | Component application logic: `timeTriggered`, `get_frame_packet`, `can_send_to_mavlink`, `can_send_to_vmm`, `udp_frame_from_raw_eth`, `port_allowed`, etc. |
| `crates/GumboLib/src/lib.rs` | HAMR-generated GUMBO spec functions (not modified) |

---

## Setup: Stripping Existing Contracts

**Date:** 2026-05-11

The starting codebase contained Verus contracts written by Robbie V across
`firewall_core` (`net.rs` and `lib.rs`) and the app
(`seL4_RxFirewall_RxFirewall_app.rs`). These contracts included 55 `open spec
fn` definitions in `net.rs` (1042 lines), 14 `open spec fn` definitions and
biconditional postconditions on `EthFrame::parse` in `lib.rs` (472 lines), and
extensive contracts in the app (475 lines) including spec functions, loop
invariants, and postconditions bridging GumboLib predicates to `firewall_core`
result predicates.

The user asked Claude to strip all Verus contracts from `firewall_core`
entirely and to strip all user-supplied contracts from the app, preserving only
the HAMR-generated `requires`/`ensures` on `timeTriggered` (the 8 `requires`
clauses and 20 `ensures` clauses between `BEGIN MARKER` / `END MARKER`
comments). The goal was to have Claude re-derive all contracts from scratch
based solely on reading the implementation code.

---

## Phase 1: Initial Attempt — assume_specification

**Date:** 2026-05-11

With the contracts stripped, the user asked Claude to add whatever Verus
artifacts were needed to make `make verus` succeed on the RxFirewall crate.

Claude's first approach was to mark `firewall_core`'s parser functions with
`assume_specification` — effectively telling Verus "assume the firewall_core
parsing functions satisfy whatever postconditions are claimed, without checking
the implementation." This made it trivial to get the app-layer contracts to
verify: if you *assume* that `EthFrame::parse` correctly returns ARP when the
bytes say ARP, and correctly returns UDP with the right ports when the bytes
say UDP, then the `timeTriggered` postconditions follow straightforwardly from
the routing logic.

The user immediately rejected this approach: *"isn't the assume_specification
a big cheat? I need the actual code as supplied by the user to be checked
against verus specs."* The entire point of the exercise was to verify that the
**implementation** is correct, not to assume it correct and verify only the
wiring. This attempt was discarded without being committed.

The user then redirected the approach: start with `firewall_core` in
isolation, add real contracts to every function, and make them verify without
any `assume_specification` or `external_body` (except for logging). Only after
`firewall_core` verifies on its own should Claude move on to the app.

---

## Phase 2: Contracts on firewall_core

**Date:** 2026-05-11 (committed `9724177` at 13:13 CDT)  
**Duration:** Single session, approximately 4 hours

Claude worked bottom-up, starting with the lowest-level parsers in `net.rs`
and building upward. The user taught Claude the Verus attribute syntax
incrementally — first `Address::from_bytes`, then `Address::is_empty`, then
correcting the loop invariant and named-return syntax before proceeding to the
remaining functions. Key constraints: **no `#[verus_verify(external_body)]`**
on non-logging functions and **no `assume_specification`** — every function
body must be verified against its contract.

### net.rs — Network Type Parsers

Each parser function received contracts that express postconditions in terms of
the raw input bytes, not in terms of abstract spec predicates. Instead of
defining a parallel spec world, contracts directly relate parsed results to
byte positions.

**Enum conversions** (`EtherType`, `IpProtocol`, `ArpOp`, `HardwareType`)
required both `TryFrom` implementations and Verus `TryFromSpecImpl` trait
implementations so that Verus could reason about `try_from` calls:

```rust
#[cfg(verus_keep_ghost)]
verus! {
    impl TryFromSpecImpl<u16> for EtherType {
        open spec fn obeys_try_from_spec() -> bool { true }
        open spec fn try_from_spec(value: u16) -> Result<Self, ()> {
            match value {
                0x0800u16 => Ok(EtherType::Ipv4),
                0x0806u16 => Ok(EtherType::Arp),
                0x86DDu16 => Ok(EtherType::Ipv6),
                _ => Err(()),
            }
        }
    }
}
```

Corresponding `spec` functions (e.g., `spec_ether_type_from_u16`) were defined
to allow postconditions to reference the conversion logic directly.

**Byte-level parser functions** (`Address::from_bytes`, `Ipv4Address::from_bytes`,
`EthernetRepr::parse`, `Arp::parse`, `Ipv4Repr::parse`, `UdpRepr::parse`,
`TcpRepr::parse`) each received postconditions connecting parsed fields to
specific byte positions. For example, `EthernetRepr::parse`:

```rust
#[verus_spec(r =>
    requires frame.len() >= 14,
    ensures
        r.is_some() ==> (
            r.unwrap().dst_addr.0@ =~= frame@.subrange(0, 6 as int)
            && r.unwrap().src_addr.0@ =~= frame@.subrange(6, 12 as int)
            && spec_ether_type_from_u16(...) == Some(r.unwrap().ethertype)
        ),
        // Forward completeness: valid conditions ==> parse succeeds
        ... ==> r.is_some(),
        // Reverse: parse success ==> dst not all zeros
        r.is_some() ==> !(frame@[0] == 0u8 && ... && frame@[5] == 0u8),
)]
```

A critical pattern emerged: every parser needed both **forward** and
**reverse** postconditions:

- **Forward (completeness):** "if the bytes satisfy these conditions, the
  parse succeeds" — needed so callers can prove that valid frames produce
  `Some` results
- **Reverse:** "if the parse succeeded, the bytes satisfy these conditions" —
  needed so callers can prove that parsed results imply byte-level predicates

Without reverse postconditions, Phase 3 would have been impossible — the app
needs to know that when `EthFrame::parse` returns `Some(ARP)`, the raw bytes
actually satisfy GumboLib's `valid_arp_spec` byte conditions.

**Loop invariants** were required for all byte-copying loops
(`Address::from_bytes`, `Ipv4Address::from_bytes`) using attribute syntax:

```rust
#[verus_spec(
    invariant
        0 <= i <= 6,
        self.0@.len() == 6,
        forall|j: int| 0 <= j < i as int ==> self.0@[j] == 0u8,
    decreases 6 - i,
)]
while i < 6 { ... }
```

### lib.rs — EthFrame::parse

`EthFrame::parse` composes the individual parsers and required the most
extensive postconditions — 20 `ensures` clauses covering:

1. **Address/ethertype extraction** — parsed addresses match byte subranges
2. **Variant correspondence** — ethertype determines `PacketType` variant
3. **UDP/TCP port correspondence** — parsed ports match frame byte positions
4. **IPv4 protocol byte correspondence** — protocol byte maps to parsed enum
5. **ARP completeness** — byte conditions sufficient for ARP parse success
6. **IPv4 UDP completeness** — byte conditions sufficient for UDP parse success
7. **Reverse ARP** — ARP result implies ethertype bytes, htype bytes, ptype
   bytes, op bytes all have specific values
8. **Reverse IPv4** — IPv4 result implies ethertype bytes, vers_ihl byte,
   length in range
9. **Reverse IPv4 UDP** — UDP result implies protocol byte == 17

The completeness and reverse postconditions are what connect the parsed
structure back to raw bytes, enabling the app layer to bridge to GumboLib's
byte-level spec predicates.

**Result:** 35 verified, 0 errors in `firewall_core`. After stripping Robbie
V's contracts and adding Claude's, net.rs went from 1042 lines to 819 lines
and lib.rs went from 472 to 379 lines. The reduction came from replacing the
previous approach's 55 abstract `open spec fn` definitions with direct
byte-level postconditions on the parser functions themselves.

---

## Phase 3: Contracts on the App

**Date:** 2026-05-12 (committed `a8d3d83` at 10:08 CDT)  
**Duration:** Approximately 6 hours across context continuations

With `firewall_core` verified in isolation, the user asked Claude to add
contracts to the app-layer functions so that `timeTriggered`'s 20
HAMR-generated postconditions verify. This required changes in *both*
`firewall_core` and the app — the Phase 2 postconditions were necessary for
`firewall_core` to verify on its own but were not sufficient for the app to
verify against the GUMBO spec predicates.

### Changes Required in firewall_core

The Phase 2 postconditions on `EthFrame::parse` were necessary but not
sufficient for the app. Two categories of additional postconditions were needed:

**1. Spec helper functions** — `is_arp_packet`, `is_ipv4_udp_packet`,
`get_udp_src_port`, `get_udp_dst_port` were added to `lib.rs` as `open spec
fn` definitions. These provide a vocabulary for the app-layer contracts to
reference parsed packet structure without destructuring in postconditions:

```rust
pub open spec fn is_arp_packet(pkt: PacketType) -> bool {
    match pkt { PacketType::Arp(_) => true, _ => false }
}
pub open spec fn get_udp_dst_port(pkt: PacketType) -> u16 {
    match pkt {
        PacketType::Ipv4(ip) => match ip.protocol {
            Ipv4ProtoPacket::Udp(udp) => udp.dst_port, _ => 0u16,
        }, _ => 0u16,
    }
}
```

**2. Additional reverse postconditions on EthFrame::parse** — Phase 2 had
reverse postconditions for individual byte fields, but the app needed
*composite* reverse postconditions that matched the structure of GumboLib
predicates. For example, GumboLib's `valid_arp_spec` checks: dst addr not all
zeros, ethertype == ARP (bytes 12-13), htype == Ethernet (bytes 14-15), ptype
== Ipv4 or Ipv6 (bytes 16-17), op == Request or Reply (bytes 20-21). A single
"ARP result implies byte 12 == 8 and byte 13 == 6" postcondition was not
enough — the app also needed to know about htype, ptype, op, and dst addr
bytes from the reverse direction.

These additions brought `lib.rs` from 379 lines (Phase 2) to 526 lines.
Net.rs grew from 819 to 891 lines due to additional `ensures` clauses on
`Arp::parse` (reverse byte correspondence for htype, ptype, op fields).

### Changes in seL4_RxFirewall_RxFirewall_app.rs

**Spec functions** — Two `open spec fn` definitions were added for the
app-layer routing predicates, providing a spec-level mirror of the exec
functions `can_send_to_mavlink` and `can_send_to_vmm`:

```rust
pub open spec fn spec_can_send_to_mavlink(packet: PacketType) -> bool {
    match packet {
        PacketType::Ipv4(ip) => match ip.protocol {
            Ipv4ProtoPacket::Udp(udp) =>
                udp.src_port == MAV_UDP_SRC_PORT && udp.dst_port == MAV_UDP_DST_PORT,
            _ => false,
        }, _ => false,
    }
}
```

**Exec function contracts** — Each helper function received an `ensures`
clause connecting its return value to the corresponding spec function:

| Function | Ensures |
|----------|---------|
| `can_send_to_mavlink` | `r == spec_can_send_to_mavlink(*packet)` |
| `can_send_to_vmm` | `r == spec_can_send_to_vmm(*packet)` |
| `udp_port_allowed` | `r == spec_port_in_list(config::udp::ALLOWED_PORTS@, port)` |
| `tcp_port_allowed` | `r == spec_port_in_list(config::tcp::ALLOWED_PORTS@, port)` |
| `port_allowed` | `r == spec_port_in_list(allowed_ports@, port)` |
| `udp_frame_from_raw_eth` | `r.headers@ =~= value@.subrange(0, HDR_DIM)` and `r.payload@ =~= value@.subrange(HDR_DIM, MSG_DIM)` |
| `udp_headers_from_raw_eth` | `r@ =~= value@.subrange(0, HDR_DIM)` |
| `udp_payload_from_raw_eth` | `r@ =~= value@.subrange(HDR_DIM, MSG_DIM)` |

The byte-copy functions (`udp_headers_from_raw_eth`, `udp_payload_from_raw_eth`)
needed loop invariants tracking the copy progress.

### The hlr_15 Disallow Problem

The most difficult postconditions to verify were the four `hlr_15` disallow
guarantees (one per Rx channel):

```
api.EthernetFramesRxIn0.is_some()
    && !(GumboLib::rx_allow_outbound_frame_spec(api.EthernetFramesRxIn0.unwrap()))
==> (api.VmmOut0.is_none() && api.MavlinkOut0.is_none())
```

This says: if the input frame is *not* an allowed outbound frame, then neither
output port is written. Proving this requires showing the **contrapositive**:
every code path that *does* write an output port implies
`rx_allow_outbound_frame_spec` holds.

The `timeTriggered` body writes output ports in two places:
- `can_send_to_mavlink` → `put_MavlinkOut`
- `can_send_to_vmm` → `put_VmmOut`

So `get_frame_packet` needed postconditions stating:

```rust
// Sending to mavlink implies frame is allowed
r.is_some() && spec_can_send_to_mavlink(r.unwrap().eth_type) ==>
    GumboLib::rx_allow_outbound_frame_spec(*frame),
// Sending to vmm implies frame is allowed
r.is_some() && spec_can_send_to_vmm(r.unwrap().eth_type) ==>
    GumboLib::rx_allow_outbound_frame_spec(*frame),
```

The mavlink case verified automatically — Verus could see that
`spec_can_send_to_mavlink` implies UDP with specific ports, which implies
`valid_ipv4_udp_mavlink_spec`, which is a disjunct of
`rx_allow_outbound_frame_spec`.

The VMM case did **not** verify automatically. The problem was a
**cross-crate constant equivalence** issue. `spec_can_send_to_vmm` references
`config::udp::ALLOWED_PORTS@` (defined in the app crate), while
`GumboLib::udp_is_valid_direct_dst_port_spec` references
`GumboLib::UDP_ALLOWED_PORTS_spec()` (defined in the GumboLib crate). Both
contain `[68u16]`, but Z3 could not equate them across crate boundaries.

Additionally, `udp_is_valid_direct_dst_port_spec` uses an existential
quantifier:

```rust
exists|i:int| 0 <= i <= UDP_ALLOWED_PORTS_spec().len() - 1
    && UDP_ALLOWED_PORTS_spec()[i] == two_bytes_to_u16_be_spec(aframe[36], aframe[37])
```

Z3 struggled to construct the witness (`i = 0`) needed to prove this existential.

**Solution:** `get_frame_packet` was moved from attribute syntax into a
`verus!` block to enable a `proof` block with concrete assertions:

```rust
proof {
    let allowed_ports_view = config::udp::ALLOWED_PORTS@;
    assert(allowed_ports_view.len() == 1);
    assert(allowed_ports_view[0int] == 68u16);
    let gumbo_ports_view = GumboLib::UDP_ALLOWED_PORTS_spec()@;
    assert(gumbo_ports_view.len() == 1);
    assert(gumbo_ports_view[0int] == 68u16);
}
```

These assertions make both port list constants concrete in the proof context,
allowing Z3 to see they contain the same value and construct the existential
witness.

**Note on attribute syntax:** The `proof` keyword is only recognized inside
`verus!` macro blocks. An earlier attempt to add proof hints while keeping
`get_frame_packet` in attribute syntax (`#[verus_spec]`) produced compilation
errors. This was the only function that required the `verus!` macro; all others
use attribute syntax.

---

## Verification Results

### firewall_core crate

```
Verification results: 35 verified, 0 errors
```

### seL4_RxFirewall_RxFirewall crate

```
Verification results: 34 verified, 0 errors
Tests: 7 passed, 0 failed
```

**Combined: 69 verification conditions, 0 errors.**

---

## Function Contract Summary

### firewall_core::net.rs

| Function | Lines | Ensures Summary |
|----------|-------|-----------------|
| `Ipv4Address::from_bytes` | 24-42 | Result bytes match input subrange `[0,4)` |
| `Address::from_bytes` | 62-79 | Result bytes match input subrange `[0,6)` |
| `Address::is_empty` | 85-102 | Returns true iff all 6 bytes are zero |
| `u16_from_be_bytes` | 116-118 | Result == `bytes[0]*256 + bytes[1]` |
| `EtherType::from_bytes` | 156-159 | Result == `spec_ether_type_from_u16(byte pair)` |
| `EtherType::try_from` | 167-176 | Matches spec conversion |
| `EthernetRepr::parse` | 247-256 | Addresses match subranges; ethertype from bytes 12-13; forward completeness; reverse dst-not-zero |
| `ArpOp::from_bytes` | 293-296 | Result == `spec_arp_op_from_u16(byte pair)` |
| `ArpOp::try_from` | 304-310 | Matches spec conversion |
| `HardwareType::from_bytes` | 370-373 | Result == `spec_hardware_type_from_u16(byte pair)` |
| `HardwareType::try_from` | 381-388 | Matches spec conversion |
| `Arp::parse` | 489-513 | Field bytes match; completeness; reverse htype/ptype/op bytes |
| `Arp::allowed_ptype` | 522-528 | Returns false iff ptype is ARP |
| `Ipv4Repr::parse` | 668-678 | Length from bytes; protocol from byte 9; vers_ihl == 69; completeness |
| `TcpRepr::parse` | 704-707 | dst_port from bytes 2-3 |
| `UdpRepr::parse` | 735-739 | src_port from bytes 0-1; dst_port from bytes 2-3 |

### firewall_core::lib.rs

| Function | Lines | Ensures Summary |
|----------|-------|-----------------|
| `EthFrame::parse` | 220-256 | 20 ensures clauses: address extraction, variant correspondence (ARP/IPv4/IPv6), UDP port correspondence, IPv4 protocol byte, ARP completeness, IPv4 UDP completeness, reverse postconditions (ARP ethertype/htype/ptype/op bytes, IPv4 ethertype/vers_ihl/length, UDP protocol byte) |

Spec functions (not verified — ghost code):
`is_arp_packet`, `is_ipv4_packet`, `is_ipv4_udp_packet`, `get_udp_src_port`,
`get_udp_dst_port`, `spec_ether_type_from_u16`, `spec_arp_op_from_u16`,
`spec_hardware_type_from_u16`, `spec_ip_protocol_from_u8`

### seL4_RxFirewall_RxFirewall_app.rs

| Function | Lines | Ensures Summary |
|----------|-------|-----------------|
| `timeTriggered` | 150-203 | 20 HAMR-generated ensures (hlr_05 ARP, hlr_18 MAVLink UDP, hlr_13 allowed-port UDP, hlr_15 disallow, hlr_17 no-input — per Rx channel) |
| `get_frame_packet` | 284-337 | 9 ensures: address match, ARP forward/reverse, IPv4 UDP forward/reverse + port correspondence, mavlink-implies-allowed, vmm-implies-allowed. Contains proof block for cross-crate constant bridging |
| `can_send_to_mavlink` | 343-350 | `r == spec_can_send_to_mavlink(*packet)` |
| `can_send_to_vmm` | 413-434 | `r == spec_can_send_to_vmm(*packet)` |
| `udp_frame_from_raw_eth` | 357-361 | Headers and payload subranges match |
| `udp_headers_from_raw_eth` | 367-384 | Result bytes == input `[0, HDR_DIM)` (with loop invariant) |
| `udp_payload_from_raw_eth` | 390-407 | Result bytes == input `[HDR_DIM, MSG_DIM)` (with loop invariant) |
| `udp_port_allowed` | 440-442 | `r == spec_port_in_list(udp::ALLOWED_PORTS@, port)` |
| `tcp_port_allowed` | 448-449 | `r == spec_port_in_list(tcp::ALLOWED_PORTS@, port)` |
| `port_allowed` | 456-471 | `r == spec_port_in_list(allowed_ports@, port)` (with loop invariant) |
| `log_info` | 218-221 | `external_body` (logging) |
| `log_warn_channel` | 224-227 | `external_body` (logging) |
| `info_protocol` | 475-477 | `external_body` (logging) |

Spec functions (ghost code):
`spec_port_in_list`, `spec_can_send_to_mavlink`, `spec_can_send_to_vmm`

---

## Key Challenges and Lessons

### 1. assume_specification is not verification

Claude's first instinct when asked to "make verus succeed" was to mark
`firewall_core` functions as `assume_specification` — telling Verus to trust
the parser without checking it. This is the path of least resistance but
defeats the purpose: it verifies only that the routing logic is wired correctly
*assuming* the parser is correct, without actually checking the parser. The
user's rejection of this approach forced the bottom-up strategy that produced
genuine end-to-end verification.

### 2. Reverse postconditions are essential

Forward postconditions ("valid bytes → parse succeeds with correct fields") are
the natural ones to write but are insufficient alone. The `hlr_15` disallow
guarantees require proving that if output *was* produced, then the frame *was*
allowed — which needs reverse postconditions ("parse succeeded with ARP →
bytes have ARP ethertype, valid htype, valid ptype, valid op"). Without
reverse postconditions on `EthFrame::parse` and its sub-parsers, there is no
way to reconstruct the byte-level predicates that GumboLib uses.

### 3. Cross-crate constant equivalence needs proof hints

Z3 cannot automatically equate `config::udp::ALLOWED_PORTS` (defined as
`[68u16]` in the app crate) with `GumboLib::UDP_ALLOWED_PORTS_spec()` (defined
as `[68u16]` in the GumboLib crate), even though they are identical. A `proof`
block with explicit assertions about length and element values was needed to
make both concrete in Z3's context. This was the only place in the entire
verification that required a proof hint.

### 4. Existential witnesses must be discoverable

GumboLib's `udp_is_valid_direct_dst_port_spec` uses
`exists|i:int| ... && PORTS[i] == port`. Even after bridging the constant
values, Z3 needed the concrete port list to be "opened up" (length == 1,
element[0] == 68) before it could construct the witness `i = 0`. Abstract
existentials over opaque sequences are a known difficulty for SMT solvers.

### 5. Verus attribute syntax vs. verus! macro

The `#[verus_spec]` attribute syntax was used for all functions except
`get_frame_packet`, which required a `proof` block. The `proof` keyword is
only recognized inside `verus!` macro blocks — attempting to use it with
attribute syntax produces a compilation error. This is the only construct
encountered that required falling back to the macro.

### 6. No assume_specification needed

The entire verification chain — from individual byte parsers through composed
frame parsing through routing decisions through the 20 `timeTriggered`
postconditions — was completed without any `assume_specification` or
`external_body` annotations on non-logging functions. Every function body is
verified against its contract.

---

## Comparison: Robbie V's Contracts vs. Claude's Contracts

Robbie V's contracts are on the `main` branch (commit `26bab7d`). Claude's
contracts are on the `claude-verus-verification` branch (commits `9724177`
and `a8d3d83`). Both verify successfully with 0 errors. This section compares
the two approaches across all three files.

### Overall Architecture

The approaches differ fundamentally in how they bridge the gap between raw
bytes (GumboLib predicates) and parsed Rust structures (firewall_core types).

**Robbie V: Abstract predicate layer with biconditionals.** Robbie V built an
intermediate spec-function vocabulary in `net.rs` — 55 `open spec fn`
definitions that give names to byte-level conditions (e.g.,
`frame_arp_subrange`, `valid_arp_frame`, `wellformed_arp_packet`). Parser
postconditions are biconditional: `valid_arp_frame(frame) == res_is_arp(r)`.
This means the postcondition asserts *equivalence* — the frame is valid ARP
if and only if the result is ARP. The app-layer `get_frame_packet` then
bridges GumboLib predicates to these intermediate spec functions, which in
turn are equivalent to the parse result predicates.

**Claude: Direct byte-level postconditions with forward + reverse.** Claude
added no abstract predicate layer in `net.rs`. Parser postconditions directly
state which byte positions correspond to which parsed fields. Two directions
are stated separately: forward ("if bytes satisfy X, parse succeeds") and
reverse ("if parse succeeded with variant Y, bytes have specific values").
The app-layer `get_frame_packet` bridges directly between GumboLib predicates
and firewall_core result predicates using forward/reverse ensures.

### Line Counts

| File | Robbie V | Claude | Delta |
|------|----------|--------|-------|
| `net.rs` | 1042 | 891 | -151 |
| `lib.rs` | 472 | 526 | +54 |
| `app.rs` | 475 | 478 | +3 |
| **Total** | **1989** | **1895** | **-94** |

Claude's approach is ~5% shorter overall. The reduction in `net.rs` (no
predicate layer) is partially offset by a larger `lib.rs` (more ensures
clauses on `EthFrame::parse`).

### net.rs Differences

#### Spec function count

Robbie V defined 55 `open spec fn` in `net.rs`, creating a multi-layered
abstraction hierarchy. For example, ARP validity was decomposed into:
`frame_arp_subrange` → `valid_arp_op_subrange` → `wellformed_arp_packet` →
`wellformed_arp_frame` → `valid_arp_frame` (the last in `lib.rs`). Each layer
added a named predicate that composed lower ones.

Claude defined 4 standalone spec functions (`spec_ether_type_from_u16`,
`spec_arp_op_from_u16`, `spec_hardware_type_from_u16`,
`spec_ip_protocol_from_u8`) — one per enum conversion — and placed
postconditions directly on the parser functions.

#### Postcondition style

Robbie V used biconditional postconditions on sub-parsers:

```rust
// Robbie V: Arp::parse — biconditional
ensures wellformed_arp_packet(packet@) == r.is_some(),
```

Claude used separate forward and reverse postconditions:

```rust
// Claude: Arp::parse — forward
(spec_hardware_type_from_u16(...).is_some()
 && spec_ether_type_from_u16(...).is_some()
 && ...) ==> r.is_some(),
// Claude: Arp::parse — reverse
r.is_some() ==> (packet@[0] == 0u8 && packet@[1] == 1u8),  // htype bytes
r.is_some() ==> ...,  // ptype bytes, op bytes
```

The biconditional approach is more concise and logically stronger — it
captures both directions in a single clause. However, it requires the named
predicate (`wellformed_arp_packet`) to exist, which pulls in the 55-function
abstraction layer.

#### Byte comparison style

Robbie V used `Seq` subrange comparisons:

```rust
frame@.subrange(12, 14) =~= seq![8u8, 6u8]  // ARP ethertype
```

Claude used individual byte comparisons:

```rust
frame@[12 as int] == 8u8 && frame@[13 as int] == 6u8  // ARP ethertype
```

The subrange style is more readable for multi-byte fields. The individual
byte style is more explicit and avoids constructing intermediate `Seq` values.

#### Precondition style

Robbie V used exact-length preconditions:

```rust
requires data@.len() == 4,  // Ipv4Address::from_bytes
requires data@.len() == 6,  // Address::from_bytes
```

Claude used minimum-length preconditions:

```rust
requires data.len() >= 4,  // Ipv4Address::from_bytes
requires data.len() >= 6,  // Address::from_bytes
```

The minimum-length style is strictly weaker and therefore easier for callers
to satisfy. The exact-length style is unnecessary for these functions since
they only read the first N bytes.

#### FromSpecImpl and TryFromSpecImpl

Robbie V provided both `TryFromSpecImpl` and `FromSpecImpl` for all enum
types. `FromSpecImpl` gives the reverse mapping (e.g., `EtherType::Arp` →
`0x0806u16`), which Verus can use to reason about serialization.

Claude provided only `TryFromSpecImpl`. The reverse direction was not needed
because Claude's postconditions directly state byte values rather than
referencing a `FromSpec` conversion.

#### Address::is_empty

Robbie V added a precondition `requires self.0@.len() == 6`, which is
necessarily true since `Address` wraps `[u8; 6]` — a fixed-size array always
has length 6. Claude omitted this precondition.

### lib.rs Differences

#### EthFrame::parse postconditions

Robbie V's `EthFrame::parse` had 8 `ensures` clauses, all expressed in terms
of the spec functions defined in `lib.rs`:

```rust
ensures
    valid_arp_frame(frame) == res_is_arp(r),
    valid_ipv4_frame(frame) == res_is_ipv4(r),
    valid_ipv6_frame(frame) == res_is_ipv6(r),
    valid_tcp_frame(frame) == res_is_tcp(r),
    valid_udp_frame(frame) == res_is_udp(r),
    valid_tcp_frame(frame) ==> tcp_port_bytes_match(frame, r),
    valid_udp_frame(frame) ==> udp_port_bytes_match(frame, r),
    valid_ipv4_frame(frame) ==> ipv4_length_bytes_match(frame, r),
```

Claude's `EthFrame::parse` had 20 `ensures` clauses expressed in terms of
direct byte conditions and the 5 spec helper functions (`is_arp_packet`,
`is_ipv4_packet`, etc.):

```rust
ensures
    // address extraction (3 clauses)
    r.is_some() ==> r.unwrap().header.dst_addr.0@ =~= frame@.subrange(0, 6),
    // variant correspondence (3 clauses)
    r.is_some() && r.unwrap().header.ethertype == EtherType::Arp
        ==> is_arp_packet(r.unwrap().eth_type),
    // UDP port correspondence (1 clause)
    r.is_some() && is_ipv4_udp_packet(r.unwrap().eth_type) ==> (
        get_udp_src_port(r.unwrap().eth_type) == frame@[34] * 256 + frame@[35]),
    // forward completeness (2 clauses for ARP, IPv4 UDP)
    (byte conditions) ==> r.is_some() && is_arp_packet(...),
    // reverse postconditions (8 clauses)
    r.is_some() && is_arp_packet(...) ==> frame@[12] == 8u8 && ...,
    ...
```

The biconditional approach collapses forward and reverse into a single `==`,
while Claude's approach states them as separate implications. The biconditional
is stronger (it implies both directions) but requires the named predicates.

#### Spec function organization

Robbie V defined 14 `open spec fn` in `lib.rs`: `valid_arp_frame`,
`valid_ipv4_frame`, `valid_ipv6_frame`, `valid_tcp_frame`,
`valid_udp_frame`, `res_is_arp`, `res_is_ipv4`, `res_is_ipv6`, `res_is_tcp`,
`res_is_udp`, `tcp_port_bytes_match`, `udp_port_bytes_match`,
`ipv4_length_bytes_match`, `ipv4_valid_length`. These compose the `net.rs`
predicates into frame-level validity checks.

Claude defined 5 `open spec fn`: `is_arp_packet`, `is_ipv4_packet`,
`is_ipv4_udp_packet`, `get_udp_src_port`, `get_udp_dst_port`. These
operate only on parsed `PacketType` values, not on raw bytes.

#### u16 byte computation

Robbie V used an indirect spec function:

```rust
net::spec_u16_from_be_bytes(frame@.subrange(34, 36))  // src port
```

Claude used direct arithmetic:

```rust
(frame@[34 as int] as u16) * 256 + (frame@[35 as int] as u16)  // src port
```

Both are equivalent; the direct form avoids an indirection at the cost of
being more verbose.

#### Type definition placement

Robbie V's original contracts were written entirely inside `verus!` macro
blocks. Prior to Claude's contract work, the codebase was refactored to use
Verus attribute syntax (`#[verus_verify]`, `#[verus_spec]`) where possible so
that `cargo mutants` could perform mutation testing on the exec code — the
`verus!` macro wraps function bodies in a way that prevents the mutation
testing tool from instrumenting them. This refactoring moved type definitions,
`impl` blocks, and parser functions out of `verus!` blocks, leaving only
constants and spec functions inside `verus!` blocks.

Claude's contracts were written against this already-refactored codebase and
follow the same attribute-syntax-first convention.

### app.rs Differences

#### get_frame_packet postconditions

Robbie V used biconditional ensures bridging GumboLib predicates to
firewall_core result predicates:

```rust
ensures
    GumboLib::valid_arp_spec(*frame) == firewall_core::res_is_arp(r),
    GumboLib::valid_ipv4_udp_spec(*frame) == firewall_core::res_is_udp(r),
    GumboLib::valid_ipv4_tcp_spec(*frame) == firewall_core::res_is_tcp(r),
    GumboLib::valid_ipv4_tcp_spec(*frame) ==> firewall_core::tcp_port_bytes_match(frame, r),
    GumboLib::valid_ipv4_udp_spec(*frame) ==> firewall_core::udp_port_bytes_match(frame, r),
```

Claude used separate forward and reverse implications:

```rust
ensures
    GumboLib::valid_arp_spec(*frame) ==> (
        r.is_some() && firewall_core::is_arp_packet(r.unwrap().eth_type)),
    r.is_some() && firewall_core::is_arp_packet(r.unwrap().eth_type) ==>
        GumboLib::valid_arp_spec(*frame),
    GumboLib::valid_ipv4_udp_spec(*frame) ==> (
        r.is_some() && firewall_core::is_ipv4_udp_packet(r.unwrap().eth_type)
        && (port correspondence)),
    r.is_some() && firewall_core::is_ipv4_udp_packet(r.unwrap().eth_type) ==>
        GumboLib::valid_ipv4_udp_spec(*frame),
    // + mavlink-implies-allowed, vmm-implies-allowed
```

Claude additionally added two postconditions that Robbie V did not:

```rust
r.is_some() && spec_can_send_to_mavlink(r.unwrap().eth_type) ==>
    GumboLib::rx_allow_outbound_frame_spec(*frame),
r.is_some() && spec_can_send_to_vmm(r.unwrap().eth_type) ==>
    GumboLib::rx_allow_outbound_frame_spec(*frame),
```

These directly support the `hlr_15` disallow guarantees.

#### Cross-crate constant equivalence

Robbie V solved the `config::udp::ALLOWED_PORTS` vs.
`GumboLib::UDP_ALLOWED_PORTS_spec()` mismatch with a **precondition** on
`can_send_to_vmm`:

```rust
requires
    config::udp::ALLOWED_PORTS =~= GumboLib::UDP_ALLOWED_PORTS_spec(),
```

This pushes the proof obligation to the caller. The caller (`timeTriggered`)
must establish this precondition, which Verus can satisfy because both
constants are visible to the verifier at the call site.

Claude solved it with a **proof block** in `get_frame_packet`:

```rust
proof {
    let allowed_ports_view = config::udp::ALLOWED_PORTS@;
    assert(allowed_ports_view.len() == 1);
    assert(allowed_ports_view[0int] == 68u16);
    let gumbo_ports_view = GumboLib::UDP_ALLOWED_PORTS_spec()@;
    assert(gumbo_ports_view.len() == 1);
    assert(gumbo_ports_view[0int] == 68u16);
}
```

This handles the obligation locally, at the cost of requiring the `verus!`
macro for `get_frame_packet` instead of attribute syntax.

#### Spec functions

Robbie V defined 5 spec functions in the app: `packet_is_mavlink_udp`,
`packet_is_whitelisted_udp`, `packet_is_whitelisted_tcp`,
`ipv4_udp_on_allowed_port_quant`, `ipv4_tcp_on_allowed_port_quant`.

Claude defined 3: `spec_can_send_to_mavlink`, `spec_can_send_to_vmm`,
`spec_port_in_list`.

Robbie V's `packet_is_whitelisted_tcp` and `ipv4_tcp_on_allowed_port_quant`
have no callers in the verified code — TCP filtering is not exercised by the
current `timeTriggered` implementation (TCP packets are dropped by
`can_send_to_vmm`). They appear to be forward-looking definitions for a
future where TCP allowlisting is used.

#### Config module placement

Robbie V defined the `config` module inside a `verus!` block. Claude defined
it outside with nested `verus!` blocks only for the constant arrays.

#### port_allowed ensures style

Both implementations are identical in structure — a linear scan with the same
loop invariant. The only difference is the ensures predicate name:

```rust
// Robbie V
r == config::udp::ALLOWED_PORTS@.contains(port)

// Claude
r == spec_port_in_list(allowed_ports@, port)
```

Robbie V used `Seq::contains` directly; Claude defined `spec_port_in_list`
as an explicit existential. Both are equivalent — `Seq::contains` is defined
as an existential in vstd.

### Summary of Tradeoffs

| Dimension | Robbie V | Claude |
|-----------|----------|--------|
| **Abstraction** | 55 named spec predicates creating multi-layer hierarchy | 9 spec functions; postconditions are direct |
| **Postcondition style** | Biconditional (`==`) | Separate forward (`==>`) and reverse (`==>`) |
| **Byte comparison** | Seq subranges (`frame@.subrange(12,14) =~= seq![8,6]`) | Individual bytes (`frame@[12] == 8u8 && frame@[13] == 6u8`) |
| **Constant bridging** | Precondition on `can_send_to_vmm` | Proof block in `get_frame_packet` |
| **Spec trait impls** | Both `TryFromSpecImpl` and `FromSpecImpl` | `TryFromSpecImpl` only |
| **Preconditions** | Exact length (`data@.len() == 4`) | Minimum length (`data.len() >= 4`) |
| **Syntax** | All attribute syntax | Attribute syntax + one `verus!` block |
| **Unused code** | TCP allowlist spec fns (forward-looking) | None |

**Robbie V's strengths:** The biconditional approach is more elegant — a
single `valid_arp_frame(frame) == res_is_arp(r)` captures both directions.
The named predicate layer provides a stable vocabulary for reasoning about
frame validity that could be reused by other components. The `requires`
approach for constant bridging keeps everything in attribute syntax.

**Claude's strengths:** The direct postcondition approach requires no
intermediate vocabulary, making it easier to audit which byte positions
correspond to which parsed fields. Forward completeness postconditions
(absent in Robbie V's contracts) explicitly state when parsing is guaranteed
to succeed. The proof-block approach for constant bridging is self-contained
rather than pushing obligations to callers.
