# Refactorings and Configuration Changes

Changes made to the ardupilot-basic project beyond the initial reference configuration.

## ArduPilot_MOCK Instead of ArduPilot

The `ArduPilot_seL4` process instantiates `ArduPilot_MOCK` rather than the full `ArduPilot` thread. In production, `ArduPilot` runs as an unmodified Linux binary inside a Virtual Machine Monitor (VMM) protection domain and uses ARM Secure Monitor Calls (SMC) to interact with the seL4 kernel. QEMU's ARM emulation does not support SMC forwarding, so the VMM-hosted ArduPilot cannot run in the simulator. `ArduPilot_MOCK` is a plain HAMR periodic thread with the same port interface that stands in for ArduPilot during development and QEMU-based testing.

## Verus Attribute Syntax

The HAMR configuration (`Platform.sysml`) uses the `--verus-attribute-syntax` codegen option:

```
//@ HAMR: --platform Microkit --verus-attribute-syntax --sel4-output-dir hamr/microkit_dev
```

This generates Verus contracts using Rust attribute syntax (`#[requires(...)]`, `#[ensures(...)]`) instead of inline `requires`/`ensures` blocks. The attribute form is required for compatibility with `cargo-mutants`, which cannot parse the inline Verus contract syntax.

## Frame Period: 1850ms to 1950ms

The processor `Frame_Period` was increased from 1850ms to 1950ms in `Platform.sysml` to account for HAMR codegen now allocating 30ms (previously 10ms) for each pacer domain slot. The schedule has 5 pacer slots, so the total pacer overhead increased by 5 x 20ms = 100ms:

| Slot | Before (10ms pacer) | After (30ms pacer) |
|------|--------------------|--------------------|
| Pacer x 5 | 50ms | 150ms |
| ArduPilot_MOCK (domain 2) | 600ms | 600ms |
| LowLevelEthernetDriver (domain 3) | 300ms | 300ms |
| RxFirewall (domain 4) | 300ms | 300ms |
| MavlinkFirewall (domain 5) | 300ms | 300ms |
| TxFirewall (domain 6) | 300ms | 300ms |
| **Total** | **1850ms** | **1950ms** |

## Bug Fix: LowLevelEthernetDriver Port 3 Assume (SW.sysml)

The GUMBO integration assume for port 3 of `LowLevelEthernetDriver` had `or` where `and` (`&`) was intended, making the constraint weaker than the corresponding assumes on ports 0-2.

**Before (buggy):**
```
assume valid_tx_message_port3:
    (valid_arp(EthernetFramesTx3.amessage) and
    valid_output_arp_size(EthernetFramesTx3)) or
    (valid_ipv4(EthernetFramesTx3.amessage) or    <-- should be 'and'
    valid_output_ipv4_size(EthernetFramesTx3.amessage, EthernetFramesTx3))
```

**After (fixed):**
```
assume valid_tx_message_port3:
    (valid_arp(EthernetFramesTx3.amessage) &
    valid_output_arp_size(EthernetFramesTx3)) |
    (valid_ipv4(EthernetFramesTx3.amessage) &
    valid_output_ipv4_size(EthernetFramesTx3.amessage, EthernetFramesTx3))
```

Ports 0-2 require that an IPv4 message satisfies both `valid_ipv4` and `valid_output_ipv4_size`. The `or` on port 3 allowed a message to pass the assume by satisfying either predicate alone.

## Short-Circuit to Logical Operator Conversion (SW.sysml)

Converted GUMBO contracts from short-circuit KerML operators (`and`, `or`, `implies`) to their logical equivalents (`&`, `|`, `not A | B`) wherever safe. The motivation is that Slang/Logika encodes short-circuit operators as ITE (if-then-else) in SMT2 queries, causing branching that increases solver complexity.

### Rules applied

- **Integration constraints** operate on unwrapped values (always present), so all operators were converted: `and` to `&`, `or` to `|`.
- **Compute contracts**: `HasEvent(X) and f(X)` must keep `and` because `f(X)` accesses the unwrapped event data port value and must not be evaluated when `HasEvent(X)` is false. Similarly, `implies` was kept where the consequent accesses port values.
- `NoSend(port)` and `HasEvent(port)` are always safe to evaluate, so conjunctions of these predicates were converted to `&`/`|`.
- `not HasEvent(X) implies NoSend(Y)` was rewritten as `HasEvent(X) | NoSend(Y)` (equivalent, avoids ITE from `implies`).

### Conversions by component

**LowLevelEthernetDriver integration** (4 assumes, 4 guarantees): all `and` to `&`, all `or` to `|`.

**RxFirewall integration** (4 assumes): all `or` to `|`.

**RxFirewall compute** (20 guarantees across 4 ports):
- hlr_05/18/13: last `and NoSend(...)` to `& NoSend(...)` in consequent
- hlr_15: `NoSend(A) and NoSend(B)` to `NoSend(A) & NoSend(B)`
- hlr_17: `not HasEvent(X) implies (NoSend(A) and NoSend(B))` to `HasEvent(X) | (NoSend(A) & NoSend(B))`

**TxFirewall integration** (4 guarantees): all `and` to `&`, all `or` to `|`.

**TxFirewall compute** (4 of 16 guarantees):
- hlr_16: `not HasEvent(X) implies NoSend(Y)` to `HasEvent(X) | NoSend(Y)`

**MavlinkFirewall compute** (4 of 16 guarantees):
- hlr_21: `not HasEvent(X) implies NoSend(Y)` to `HasEvent(X) | NoSend(Y)`

## HLR Deep Links in GUMBO Guarantee Descriptions (SW.sysml)

Added optional description strings to all `guarantee hlr_*` clauses in the GUMBO compute contracts, linking each to the corresponding page of the natural-language high-level requirements document (`requirements/Inspecta-HLRs.pdf`) hosted via GitHub Pages.

Example:
```
guarantee hlr_05_rx0_can_send_arp_to_vmm
    "https://jasonbelt.github.io/inspecta-open-platform/ardupilot-basic/requirements/Inspecta-HLRs.pdf#page=3":
```

| HLR | Requirement | Page |
|-----|-------------|------|
| hlr_05 | Rx: copy ARP to VMM output | 3 |
| hlr_13 | Rx: copy whitelisted UDP to VMM output | 3 |
| hlr_18 | Rx: copy MAVLink UDP to MavlinkFirewall output | 4 |
| hlr_15 | Rx: do not copy disallowed frame | 4 |
| hlr_17 | Rx: no output on empty input | 4 |
| hlr_07 | Tx: copy ARP frame | 5 |
| hlr_12 | Tx: copy IPv4 frame | 5 |
| hlr_14 | Tx: do not copy disallowed frame | 6 |
| hlr_16 | Tx: no output on empty input | 6 |
| hlr_19 | Mav: drop flash_bootloader command | 6 |
| hlr_20 | Mav: drop malformed MAVLink message | 7 |
| hlr_21 | Mav: no output on empty input | 7 |
| hlr_22 | Mav: copy well-formed, non-blacklisted message | 7 |

### Operators intentionally kept as short-circuit

- `HasEvent(X) and pred(X)` -- gates access to unwrapped event data port values
- `implies` where the consequent accesses port values (hlr_05/07/12/13/18/22)
- `implies` where the antecedent contains short-circuit `and` (hlr_14/15/19/20) -- converting just moves the ITE under `not`, no net benefit
