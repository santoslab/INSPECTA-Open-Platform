# RxFirewall Testing Report

## Table of Contents

- [Default PropTest Generators](#default-proptest-generators)
- [Claude Custom Strategies](#claude-custom-strategies)
- [Robbie VanVossen Custom Strategies](#robbie-vanvossen-custom-strategies)
- [Comparison: Claude vs Robbie Custom Strategies](#comparison-claude-vs-robbie-custom-strategies)

---

## Default PropTest Generators

### Test Configuration

The RxFirewall was tested using the HAMR-generated GUMBOX PropTest infrastructure with
default random generators. The test configuration is:

| Parameter | Value |
|-----------|-------|
| Target test cases | 100 |
| Reject ratio | 5 (max 500 rejected inputs) |
| Generator strategy | `option_strategy_default(SW_RawEthernetMessage_strategy_default())` |
| Input ports | `EthernetFramesRxIn0`..`EthernetFramesRxIn3` (each `Option<RawEthernetMessage>`) |

The default generator produces uniformly random 1600-byte arrays wrapped in a 50/50
`Some`/`None` distribution. No domain-specific shaping (e.g., valid EtherType bytes,
non-zero destination MAC) is applied.

### Integration Constraints

After removing the tautological integration assumes from the SysMLv2 model (see
`hamr/refactorings.md`), the regenerated GUMBOX code has **no compute precondition**.
The `testComputeCB` harness accepts every generated input unconditionally and checks
only the postcondition (the conjunction of 20 GUMBO compute guarantees).

### Tests Executed

| Test | Entry Point | Description |
|------|-------------|-------------|
| `prop_testComputeCB_macro` | `timeTriggered` | Generates random inputs for all four Rx ports, checks GUMBO postconditions |

### PropTest Generation Statistics

| Entry Point | Valid (Passed) | Rejected (Precondition) | Failed (Postcondition) | Total Generated |
|-------------|---------------|------------------------|----------------------|-----------------|
| `timeTriggered` | 100 | 0 | 0 | 100 |

All 100 generated test vectors passed. No inputs were rejected (there is no
precondition after the integration assumes were removed) and no postcondition
violations were detected.

### Test Results

| Entry Point | Result | Cases |
|-------------|--------|-------|
| `timeTriggered` | **PASS** | 100/100 |

### Coverage Analysis: Entry Points

| Entry Point | Lines Hit | Total Instrumented Lines | Coverage |
|-------------|-----------|-------------------------|----------|
| `initialize` (lines 25--30) | 5 | 5 | **100%** |
| `timeTriggered` (lines 150--203) | 20 | 44 | **45%** |

Note: `initialize` is called once per test case by the `testComputeCB` harness as
setup, so it receives full coverage despite not being tested independently.

#### `initialize` Entry Point -- Full Coverage

The `initialize` entry point contains only a logging call and has no conditional
branches. All 5 instrumented lines are covered (100 hits each).

#### `timeTriggered` Entry Point -- Incomplete Coverage

24 of 44 instrumented lines are not covered. The uncovered lines follow an identical
pattern across all four Rx channels:

**Uncovered lines (Rx0, lines 159--164):**
```rust
159:        if can_send_to_mavlink(&eth.eth_type) {
160:          let output = udp_frame_from_raw_eth(frame);
161:          api.put_MavlinkOut0(output);
162:        } else if can_send_to_vmm(&eth.eth_type) {
163:          api.put_VmmOut0(frame);
164:        }
```

The same pattern repeats for Rx1 (lines 171--176), Rx2 (lines 183--188), and
Rx3 (lines 195--200).

**Covered lines** include the outer `if let Some(frame)` check on each channel,
the `get_frame_packet(&frame)` call, and the control flow rejoining after the
inner branches.

**Some/None distribution per channel:**

| Channel | `Some` Inputs | `None` Inputs |
|---------|--------------|--------------|
| Rx0 | 52 | 48 |
| Rx1 | 53 | 47 |
| Rx2 | 56 | 44 |
| Rx3 | 49 | 51 |
| **Total** | **210** | **190** |

#### Root Cause Analysis

The uncovered lines are the inner routing branches that execute only when
`get_frame_packet` returns `Some(eth)` -- meaning the raw byte array was
successfully parsed as a valid Ethernet frame by `firewall_core::EthFrame::parse`.

For `EthFrame::parse` to return `Some`, the input must satisfy all of the following:

1. **Non-zero destination MAC address** -- the first 6 bytes (`frame[0..6]`) must
   not all be zero. With uniformly random bytes the probability of all six being
   zero is (1/256)^6, so this condition is almost always met.

2. **Valid EtherType** -- bytes 12--13 must encode one of three recognized values
   in big-endian:
   - `0x0800` (IPv4) -- bytes `[0x08, 0x00]`
   - `0x0806` (ARP) -- bytes `[0x08, 0x06]`
   - `0x86DD` (IPv6) -- bytes `[0x86, 0xDD]`

   With uniformly random bytes, the probability of hitting any of these three
   values is 3/65536 (approximately 0.005%). Over 100 test cases with ~210 `Some`
   inputs, the expected number of frames with a valid EtherType is approximately
   0.01 -- effectively zero.

3. **Protocol-specific sub-parsing** -- even if the EtherType is valid, ARP
   frames require valid `HardwareType` and `ArpOp` fields, and IPv4 frames
   require a recognized IP protocol number. These add further filtering.

The net effect is that random 1600-byte arrays almost never produce a parseable
Ethernet frame. When `get_frame_packet` returns `None`, the component correctly
does nothing (no output on any port), which satisfies the GUMBO postconditions.
But the routing logic inside the `Some(eth)` branch -- MAVLink detection,
VMM forwarding, and frame splitting -- is never exercised.

### Coverage Analysis: Application Exec Helpers

Coverage of non-`verus!` exec helper functions in [`seL4_RxFirewall_RxFirewall_app.rs`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs).
Functions inside the `verus! { }` block (spec functions and config constants, lines
231--284) are excluded from this analysis.

| Function | Line | Entry Hits | Coverage | Notes |
|----------|------|-----------|----------|-------|
| [`new()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L14) | 14 | 100 | Full | Called once per test case |
| [`initialize()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L25) | 25 | 100 | Full | Logging only, no branches |
| [`timeTriggered()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L150) | 150 | 100 | Partial (45%) | Inner routing branches uncovered |
| [`get_frame_packet()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L304) | 304 | 210 | Partial | Always returns `None` (parse fails) |
| [`can_send_to_mavlink()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L320) | 320 | 0 | **None** | Never reached (no valid parse) |
| [`udp_frame_from_raw_eth()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L338) | 338 | 0 | **None** | Never reached |
| [`udp_headers_from_raw_eth()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L351) | 351 | 0 | **None** | Never reached |
| [`udp_payload_from_raw_eth()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L375) | 375 | 0 | **None** | Never reached |
| [`can_send_to_vmm()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L403) | 403 | 0 | **None** | Never reached |
| [`udp_port_allowed()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L432) | 432 | 0 | **None** | Never reached |
| [`tcp_port_allowed()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L442) | 442 | 0 | **None** | Never reached |
| [`port_allowed()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L453) | 453 | 0 | **None** | Never reached |
| [`info_protocol()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L473) | 473 | 0 | **None** | Never reached |
| [`log_info()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L218) | 218 | 310 | Full | Called from `initialize` (100) and `get_frame_packet` (210) |
**Summary:** 5 of 14 exec helper functions are reached. The 9 unreached functions
are all downstream of the `get_frame_packet` parse succeeding, which never happens
with random byte arrays.

**Overall file coverage:** 39/137 lines hit = **28.47%**

### Coverage Analysis: GUMBOX Contract Methods

Coverage of the executable GUMBO contract functions in
[`seL4_RxFirewall_RxFirewall_GUMBOX.rs`](../crates/seL4_RxFirewall_RxFirewall/src/bridge/seL4_RxFirewall_RxFirewall_GUMBOX.rs). Each GUMBOX guarantee function uses the
`implies!` macro (`!$lhs || $rhs`), which short-circuits: the consequent (right-hand
side) is only evaluated when the antecedent (left-hand side) is true.

#### Per-Guarantee Consequent Coverage

| Guarantee | Rx0 | Rx1 | Rx2 | Rx3 | Consequent Reached? |
|-----------|-----|-----|-----|-----|---------------------|
| `hlr_05` (ARP -> VMM) | 0 | 0 | 0 | 0 | **No** -- `valid_arp()` never true |
| `hlr_18` (MAVLink UDP) | 0 | 0 | 0 | 0 | **No** -- `valid_ipv4_udp_mavlink()` never true |
| `hlr_13` (Whitelisted UDP) | 0 | 0 | 0 | 0 | **No** -- `valid_ipv4_udp_port()` never true |
| `hlr_15` (Disallow) | 52 | 53 | 56 | 49 | **Yes** -- `!rx_allow_outbound_frame()` true for all `Some` inputs |
| `hlr_17` (No input) | 100 | 100 | 100 | 100 | **Fully covered** -- non-short-circuit `\|` evaluates both sides |

All 22 GUMBOX functions are called (100 invocations each). The function-entry
and antecedent-evaluation lines are always covered. However, the consequent lines
of `hlr_05`, `hlr_18`, and `hlr_13` are never reached because no random input
produces a valid ARP, MAVLink UDP, or whitelisted UDP frame.

The `hlr_15` (disallow) consequent is reached because random bytes naturally fail all
`rx_allow_outbound_frame` checks -- the `is_some() && !rx_allow_outbound_frame()`
antecedent evaluates to true for every `Some` input, triggering evaluation of the
consequent (`api_VmmOut.is_none() & api_MavlinkOut.is_none()`).

The `hlr_17` (no input) guarantee uses non-short-circuit `|` (bitwise OR), so both
sides are always evaluated regardless of `is_some()`.

**Overall GUMBOX file coverage:** 195/227 lines hit = **85.90%**, 324/396 regions
covered = **81.82%**

#### Sub-Expression Region Coverage

Line coverage alone cannot reveal whether all sub-expressions within a compound
boolean expression have been evaluated. For example, in `A && B || C`, line coverage
reports one count for the entire line, but `B` may never execute if `A` is always
false, and `C` may never execute if `A && B` is always true. LLVM's region coverage
instruments each sub-expression of short-circuit operators (`&&`, `||`) as a separate
region, producing per-sub-expression execution counts that expose these gaps.

The `implies!` macro expands to `!$lhs || $rhs`. With a compound antecedent of the
form `is_some() && validator(unwrap())`, the full expression becomes:

```
!(is_some() && validator(unwrap())) || consequent
```

LLVM instruments this as three regions:

| Region | Sub-Expression | Evaluates When |
|--------|---------------|----------------|
| R1 | `is_some()` | Always (function entry) |
| R2 | `validator(unwrap())` | Only when `is_some()` returns true (`&&` short-circuits on false) |
| R3 | consequent | Only when antecedent is true (`!antecedent` is false, so `\|\|` evaluates RHS) |

##### hlr_05 / hlr_18 / hlr_13 (ARP, MAVLink, Whitelisted UDP)

These three guarantee families share the same structure. The antecedent is
`is_some() && validator(unwrap())` where `validator` is `valid_arp`,
`valid_ipv4_udp_mavlink`, or `valid_ipv4_udp_port` respectively.

| Region | Sub-Expression | Rx0 | Rx1 | Rx2 | Rx3 |
|--------|---------------|-----|-----|-----|-----|
| R1 | `is_some()` | 100 | 100 | 100 | 100 |
| R2 | `validator(unwrap())` | 52 | 53 | 56 | 49 |
| R3 | consequent (output checks) | **0** | **0** | **0** | **0** |

R2 counts match the number of `Some` inputs per channel. The validator functions
are reached for every `Some` input but always return false (no random byte array
produces a valid EtherType). Since the full antecedent is never true,
`!antecedent` is always true, and `||` short-circuits -- R3 is never evaluated.

**Gap:** The consequent (output checks) of hlr_05, hlr_18, and hlr_13 is **dead
code** under default random generation. The guarantee that valid ARP frames are
forwarded to VmmOut, MAVLink frames to MavlinkOut, and whitelisted UDP frames
to VmmOut is never actually checked at runtime.

##### hlr_15 (Disallow)

The antecedent is `is_some() && !(rx_allow_outbound_frame(unwrap()))`.

| Region | Sub-Expression | Rx0 | Rx1 | Rx2 | Rx3 |
|--------|---------------|-----|-----|-----|-----|
| R1 | `is_some()` | 100 | 100 | 100 | 100 |
| R2 | `!(rx_allow_outbound_frame(unwrap()))` | 52 | 53 | 56 | 49 |
| R3 | `VmmOut.is_none() & MavlinkOut.is_none()` | **52** | **53** | **56** | **49** |

Every `Some` input fails `rx_allow_outbound_frame` (returns false, negated to true),
making the full antecedent true for all `Some` inputs. The consequent is evaluated
every time the antecedent is true. All three regions are covered.

However, this only tests the trivial disallow case (random unparseable bytes). It does
not test deliberately crafted disallowed frames (e.g., IPv6, TCP on non-whitelisted
port) where `rx_allow_outbound_frame` would return false for a *different* reason.

##### hlr_17 (No Input)

This guarantee uses non-short-circuit `|` (bitwise OR) directly, not the `implies!`
macro:

```rust
api_EthernetFramesRxIn0.is_some() |
  api_VmmOut0.is_none() & api_MavlinkOut0.is_none()
```

| Region | Sub-Expression | Rx0 | Rx1 | Rx2 | Rx3 |
|--------|---------------|-----|-----|-----|-----|
| R1 | `is_some()` | 100 | 100 | 100 | 100 |
| R2 | `VmmOut.is_none() & MavlinkOut.is_none()` | 100 | 100 | 100 | 100 |

Both sides are always evaluated because `|` (bitwise OR) does not short-circuit.
All regions are fully covered.

##### Region Coverage Summary

| Guarantee | Total Regions | Regions Covered | Coverage |
|-----------|--------------|-----------------|----------|
| `hlr_05` (×4 channels) | 48 | 32 | **67%** |
| `hlr_18` (×4 channels) | 56 | 32 | **57%** |
| `hlr_13` (×4 channels) | 48 | 32 | **67%** |
| `hlr_15` (×4 channels) | 32 | 32 | **100%** |
| `hlr_17` (×4 channels) | 24 | 24 | **100%** |
| `compute_CEP_T_Guar` | 22 | 22 | **100%** |
| `compute_CEP_Post` | 2 | 2 | **100%** |
| **Total** | **232** | **176** | **75.86%** |

Note: The per-guarantee region totals are from the LLVM coverage report. Region
counts differ from the 3-region (R1/R2/R3) simplification above because LLVM
also creates regions for function entry, parameter evaluation, and `&` (bitwise
AND) sub-expressions within the consequent.

#### Which GUMBO Guarantees Are Not Exercised

The following GUMBO compute guarantees are effectively untested because no generated
input triggers their antecedent conditions:

| Guarantee | Requirement | What It Specifies |
|-----------|-------------|-------------------|
| `hlr_05_rx{0..3}_can_send_arp_to_vmm` | HLR-05 | Valid ARP frames are forwarded unchanged to VmmOut |
| `hlr_18_rx{0..3}_can_send_mavlink_udp` | HLR-18 | MAVLink UDP frames are split and forwarded to MavlinkOut |
| `hlr_13_rx{0..3}_can_send_ipv4_udp` | HLR-13 | Whitelisted UDP frames are forwarded unchanged to VmmOut |
| `hlr_15_rx{0..3}_disallow` | HLR-15 | Disallowed frames produce no output |

Only `hlr_17_rx{0..3}_no_input` (no input implies no output) is routinely exercised,
since roughly half the generated inputs are `None` and the other half are unparseable
random bytes that also produce no output.

Note: `hlr_15` consequent lines are covered (the postcondition check runs), but the
guarantee is tested only in the trivial case where random bytes fail to parse -- not
with deliberately crafted disallowed frames (e.g., IPv6, TCP on non-whitelisted port).

### Coverage Analysis: `firewall_core` Crate

Coverage of exec (non-`verus!`) functions in the `firewall_core` crate. Spec functions
inside `verus! { }` blocks (starting at `lib.rs` line 117 and `net.rs` line 549) are
excluded. Unit tests in `lib.rs` (`eth_frame_tests` module) are also excluded from
this analysis -- only hits from the PropTest harness are counted.

#### [`firewall_core::lib.rs`](../crates/firewall_core/src/lib.rs) -- Frame Parser

| Function | Line | Entry Hits | Notes |
|----------|------|-----------|-------|
| [`EthFrame::parse()`](../crates/firewall_core/src/lib.rs#L78) | 78 | 210 | Entered 210 times, always returns `None` at line 79 |

`EthFrame::parse` calls `EthernetRepr::parse` on the first 14 bytes. Since
`EthernetRepr::parse` returns `None` for every random input (invalid EtherType),
execution never reaches the `match header.ethertype` dispatch (line 80 onward).

**lib.rs exec coverage:** 3/28 lines hit = **10.71%**

#### [`firewall_core::net.rs`](../crates/firewall_core/src/net.rs) -- Protocol Parsers

| Function | Line | Entry Hits | Notes |
|----------|------|-----------|-------|
| [`Ipv4Address::from_bytes()`](../crates/firewall_core/src/net.rs#L23) | 23 | 0 | Never called (no valid ARP or IPv4 parse) |
| [`Address::from_bytes()`](../crates/firewall_core/src/net.rs#L62) | 62 | 420 | Called twice per `EthernetRepr::parse` (dst + src MAC) |
| [`Address::is_empty()`](../crates/firewall_core/src/net.rs#L86) | 86 | 210 | Called once per parse (dst MAC zero check) |
| [`u16_from_be_bytes()`](../crates/firewall_core/src/net.rs#L116) | 116 | 210 | Called once per parse (EtherType bytes) |
| [`EtherType::from_bytes()`](../crates/firewall_core/src/net.rs#L144) | 144 | 210 | Called once per parse, always returns `None` |
| [`EtherType::try_from()`](../crates/firewall_core/src/net.rs#L154) | 154 | 210 | All 210 calls hit the `_ => Err(())` branch |
| `EtherType::from()` (into) | 167 | 0 | Never called |
| [`EthernetRepr::parse()`](../crates/firewall_core/src/net.rs#L202) | 202 | 210 | Called 210 times, always returns `None` at `?` on line 208 |
| [`ArpOp::from_bytes()`](../crates/firewall_core/src/net.rs#L240) | 240 | 0 | Never called |
| [`ArpOp::try_from()`](../crates/firewall_core/src/net.rs#L250) | 250 | 0 | Never called |
| [`ArpOp::from()`](../crates/firewall_core/src/net.rs#L262) | 262 | 0 | Never called |
| [`HardwareType::from_bytes()`](../crates/firewall_core/src/net.rs#L290) | 290 | 0 | Never called |
| [`HardwareType::try_from()`](../crates/firewall_core/src/net.rs#L300) | 300 | 0 | Never called |
| [`HardwareType::from()`](../crates/firewall_core/src/net.rs#L311) | 311 | 0 | Never called |
| [`Arp::parse()`](../crates/firewall_core/src/net.rs#L350) | 350 | 0 | Never called |
| [`Arp::allowed_ptype()`](../crates/firewall_core/src/net.rs#L380) | 380 | 0 | Never called |
| [`IpProtocol::try_from()`](../crates/firewall_core/src/net.rs#L414) | 414 | 0 | Never called |
| [`IpProtocol::from()`](../crates/firewall_core/src/net.rs#L434) | 434 | 0 | Never called |
| [`Ipv4Repr::parse()`](../crates/firewall_core/src/net.rs#L475) | 475 | 0 | Never called |
| [`TcpRepr::parse()`](../crates/firewall_core/src/net.rs#L509) | 509 | 0 | Never called |
| [`UdpRepr::parse()`](../crates/firewall_core/src/net.rs#L538) | 538 | 0 | Never called |

**net.rs exec coverage:** 35/165 lines hit = **21.21%**

#### `firewall_core` Coverage Summary

| File | Functions Hit | Total Functions | Lines Hit | Total Lines | Coverage |
|------|--------------|----------------|-----------|-------------|----------|
| [`lib.rs`](../crates/firewall_core/src/lib.rs) | 1/1 | 100% | 3/28 | **10.71%** |
| [`net.rs`](../crates/firewall_core/src/net.rs) | 6/21 | 29% | 35/165 | **21.21%** |
| **Total** | **7/22** | **32%** | **38/193** | **19.69%** |

Only the Ethernet header parsing pipeline is exercised: `Address::from_bytes` ->
`Address::is_empty` -> `u16_from_be_bytes` -> `EtherType::from_bytes` ->
`EtherType::try_from` -> `EthernetRepr::parse`. All deeper protocol parsers (ARP,
IPv4, TCP, UDP) receive zero hits because `EtherType::try_from` returns `Err(())`
for every random two-byte value (probability of hitting a valid EtherType is 3/65536).

### Overall Coverage Summary

| Component | Lines Hit | Total Lines | Coverage |
|-----------|-----------|-------------|----------|
| App entry points + exec helpers | 39 | 137 | **28.47%** |
| GUMBOX contract methods | 195 | 227 | **85.90%** |
| [`firewall_core::lib.rs`](../crates/firewall_core/src/lib.rs) (exec) | 3 | 28 | **10.71%** |
| [`firewall_core::net.rs`](../crates/firewall_core/src/net.rs) (exec) | 35 | 165 | **21.21%** |
| **Total** | **272** | **557** | **48.83%** |

### Recommendations

To achieve complete coverage of the `timeTriggered` routing logic, custom PropTest
strategies are needed that produce well-formed Ethernet frames. Specifically:

1. **ARP frames** -- set bytes 0--5 to non-zero values, bytes 12--13 to `[0x08, 0x06]`,
   and populate valid `HardwareType` and `ArpOp` fields at the appropriate offsets.

2. **MAVLink UDP frames** -- set EtherType to IPv4 (`[0x08, 0x00]`), IP protocol to
   UDP (17), and UDP src/dst ports to 14550/14562.

3. **Whitelisted UDP frames** -- set EtherType to IPv4, IP protocol to UDP, and UDP
   destination port to 68 (DHCP client), with a source port that is not 14550 (to
   avoid matching the MAVLink path).

4. **Disallowed frames** -- valid Ethernet frames with an EtherType/protocol combination
   that does not match ARP, whitelisted UDP, or MAVLink (e.g., IPv4 TCP on a
   non-whitelisted port, or IPv6).

---

## Claude Custom Strategies

### Motivation

The [Default PropTest Generators](#default-proptest-generators) analysis identified that
uniformly random 1600-byte arrays almost never produce a frame that passes
`firewall_core::EthFrame::parse`. As a result, the inner routing branches of
`timeTriggered` -- MAVLink forwarding, VMM forwarding, and disallowed-frame dropping --
were never exercised, leaving coverage at 45% (20/44 lines).

Claude designed custom PropTest strategies to address this gap. Each strategy starts from
a random 1600-byte array (via the HAMR-generated `SW_RawEthernetMessage_strategy_default`)
and uses `prop_map` to stamp the minimum required header bytes so that `EthFrame::parse`
succeeds and the frame is classified into a specific routing category. The remaining
bytes stay random, preserving PropTest's ability to explore the input space. The byte
offsets and values used in each strategy were derived from reading the `firewall_core`
parser source code (`EthernetRepr::parse`, `Arp::parse`, `Ipv4Repr::parse`,
`UdpRepr::parse`) and the RxFirewall application logic (`can_send_to_mavlink`,
`can_send_to_vmm`, `config::udp::ALLOWED_PORTS`, `config::tcp::ALLOWED_PORTS`).

### Test Configuration

| Parameter | Value |
|-----------|-------|
| Target test cases per test | 100 |
| Reject ratio | 5 (max 500 rejected inputs per test) |
| Input ports | `EthernetFramesRxIn0`..`EthernetFramesRxIn3` |
| Test file | [`src/test/tests_claude_cust_strategies.rs`](../crates/seL4_RxFirewall_RxFirewall/src/test/tests_claude_cust_strategies.rs) |
| Test name prefix | `prop_claude_cust_strategies_*` |

### Strategy Descriptions

#### `arp_frame_strategy` -- Valid ARP Frames (HLR-05)

Generates frames that parse as valid ARP requests, exercising the
`can_send_to_vmm` ARP path that forwards frames unchanged to `VmmOut`.

**Stamped bytes:**

| Offset | Value | Purpose |
|--------|-------|---------|
| 0--5 | `ff:ff:ff:ff:ff:ff` | Broadcast destination MAC (non-zero, satisfies `EthernetRepr::parse`) |
| 12--13 | `0x08 0x06` | EtherType = ARP |
| 14--15 | `0x00 0x01` | `HardwareType` = Ethernet (required by `Arp::parse`) |
| 16--17 | `0x08 0x00` | `ptype` = IPv4 (required by `Arp::parse`) |
| 20--21 | `0x00 0x01` | `ArpOp` = Request (required by `Arp::parse`) |

The broadcast MAC mirrors real ARP request behavior. All other bytes remain random.
When this frame reaches the routing logic, `get_frame_packet` returns
`Some(EthFrame::Arp(...))`, `can_send_to_mavlink` returns false, and
`can_send_to_vmm` returns true, so the frame is forwarded to `VmmOut`.

**GUMBO guarantee exercised:** `hlr_05_rx{0..3}_can_send_arp_to_vmm`

#### `mavlink_udp_frame_strategy` -- MAVLink UDP Frames (HLR-18)

Generates frames that parse as MAVLink UDP traffic, exercising the
`can_send_to_mavlink` path that extracts the UDP payload and forwards it
to `MavlinkOut`.

**Stamped bytes:**

| Offset | Value | Purpose |
|--------|-------|---------|
| 0--5 | `02:00:00:00:00:01` | Non-zero unicast destination MAC |
| 12--13 | `0x08 0x00` | EtherType = IPv4 |
| 14 | `0x45` | IPv4 version 4, IHL 5 (20-byte header, required by `Ipv4Repr::parse`) |
| 16--17 | `0x00 0x3c` | Total length = 60 (must be <= 9000 for `Ipv4Repr::parse`) |
| 23 | `0x11` | IP protocol = UDP (17) |
| 34--35 | `0x38 0xD6` | UDP source port = 14550 (`MAV_UDP_SRC_PORT`) |
| 36--37 | `0x38 0xE2` | UDP destination port = 14562 (`MAV_UDP_DST_PORT`) |

The source and destination ports match the MAVLink constants defined in the
application's `config` module. When this frame reaches the routing logic,
`get_frame_packet` returns `Some(EthFrame::Udp(...))`, and `can_send_to_mavlink`
returns true because both the source port (14550) and destination port (14562)
match. The frame is then processed by `udp_frame_from_raw_eth` and the result
is sent to `MavlinkOut`.

**GUMBO guarantee exercised:** `hlr_18_rx{0..3}_can_send_mavlink_udp`

#### `whitelisted_udp_frame_strategy` -- Whitelisted UDP Frames (HLR-13)

Generates frames that parse as non-MAVLink UDP traffic on an allowed port,
exercising the `can_send_to_vmm` UDP allowlist path that forwards frames
unchanged to `VmmOut`.

**Stamped bytes:**

| Offset | Value | Purpose |
|--------|-------|---------|
| 0--5 | `02:00:00:00:00:01` | Non-zero unicast destination MAC |
| 12--13 | `0x08 0x00` | EtherType = IPv4 |
| 14 | `0x45` | IPv4 version 4, IHL 5 |
| 16--17 | `0x00 0x3c` | Total length = 60 |
| 23 | `0x11` | IP protocol = UDP (17) |
| 34--35 | `0x00 0x43` | UDP source port = 67 (DHCP server) |
| 36--37 | `0x00 0x44` | UDP destination port = 68 (DHCP client) |

Port 68 is in the application's `config::udp::ALLOWED_PORTS` allowlist. The source
port (67) does not match `MAV_UDP_SRC_PORT` (14550), so `can_send_to_mavlink`
returns false. Then `can_send_to_vmm` returns true because the destination port is
whitelisted, and the frame is forwarded unchanged to `VmmOut`.

**GUMBO guarantee exercised:** `hlr_13_rx{0..3}_can_send_ipv4_udp`

#### `disallowed_frame_strategy` -- Disallowed Frames (HLR-15)

Generates frames that parse successfully but are rejected by both
`can_send_to_mavlink` and `can_send_to_vmm`, verifying that the component
correctly produces no output. Uses `prop_oneof!` to uniformly select from
three sub-categories:

1. **IPv6 frames** -- EtherType `0x86DD`. The firewall recognizes IPv6 at the
   Ethernet layer but has no IPv6 routing rules, so these are dropped.

2. **IPv4 TCP on a non-whitelisted port** -- IP protocol 6 (TCP), destination
   port 80. TCP port 80 is not in `config::tcp::ALLOWED_PORTS` (which contains
   only 5760), so `can_send_to_vmm` returns false.

3. **IPv4 UDP on a non-whitelisted, non-MAVLink port** -- IP protocol 17 (UDP),
   source port 1234, destination port 9999. Neither port matches the MAVLink
   constants, and destination port 9999 is not in `config::udp::ALLOWED_PORTS`
   (which contains only 68).

In all three cases, the routing logic falls through without calling any `put_`
method, so no output is produced on any port.

**GUMBO guarantee exercised:** `hlr_15_rx{0..3}_disallow`

#### `mixed_frame_strategy` -- Weighted Mix of All Categories

Combines all of the above into a single strategy using `prop_oneof!` with weights:

| Weight | Category | Description |
|--------|----------|-------------|
| 1 | `None` | No input on this port (exercises `hlr_17` no-input path) |
| 2 | ARP | Valid ARP request |
| 2 | MAVLink UDP | MAVLink UDP (14550 -> 14562) |
| 2 | Whitelisted UDP | DHCP client (dst port 68) |
| 2 | Disallowed | IPv6 / TCP port 80 / UDP port 9999 |
| 1 | Random bytes | Unstructured 1600-byte array (unparseable, exercises no-parse drop) |

The weights are chosen so that parseable frame categories collectively dominate
(8 out of 10), while `None` and random-bytes inputs each appear at 10% to maintain
coverage of the no-input and unparseable-input paths. Each port draws independently
from this distribution, so a single test case may combine different frame types
across the four Rx channels.

This test exercises all five GUMBO guarantee families (`hlr_05`, `hlr_13`, `hlr_15`,
`hlr_17`, `hlr_18`) in a single PropTest run.

### Tests Executed

| Test | Strategy | Description |
|------|----------|-------------|
| `prop_claude_cust_strategies_mixed` | `mixed_frame_strategy` | Weighted mix of all frame categories plus `None`; exercises every routing branch |
| `prop_claude_cust_strategies_arp` | `arp_frame_strategy` | All ports receive valid ARP frames; targets HLR-05 |
| `prop_claude_cust_strategies_mavlink_udp` | `mavlink_udp_frame_strategy` | All ports receive MAVLink UDP frames; targets HLR-18 |
| `prop_claude_cust_strategies_whitelisted_udp` | `whitelisted_udp_frame_strategy` | All ports receive DHCP UDP frames; targets HLR-13 |
| `prop_claude_cust_strategies_disallowed` | `disallowed_frame_strategy` | All ports receive parseable but rejected frames; targets HLR-15 |

### PropTest Generation Statistics

| Test | Valid (Passed) | Rejected (Precondition) | Failed (Postcondition) | Total Generated |
|------|---------------|------------------------|----------------------|-----------------|
| `mixed` | 100 | 0 | 0 | 100 |
| `arp` | 100 | 0 | 0 | 100 |
| `mavlink_udp` | 100 | 0 | 0 | 100 |
| `whitelisted_udp` | 100 | 0 | 0 | 100 |
| `disallowed` | 100 | 0 | 0 | 100 |

All 500 generated test vectors passed. No inputs were rejected (there is no
precondition) and no postcondition violations were detected.

### Test Results

| Test | Result | Cases |
|------|--------|-------|
| `prop_claude_cust_strategies_mixed` | **PASS** | 100/100 |
| `prop_claude_cust_strategies_arp` | **PASS** | 100/100 |
| `prop_claude_cust_strategies_mavlink_udp` | **PASS** | 100/100 |
| `prop_claude_cust_strategies_whitelisted_udp` | **PASS** | 100/100 |
| `prop_claude_cust_strategies_disallowed` | **PASS** | 100/100 |

### Coverage Analysis: Entry Points

| Entry Point | Lines Hit | Total Instrumented Lines | Coverage |
|-------------|-----------|-------------------------|----------|
| `initialize` (lines 25--30) | 5 | 5 | **100%** |
| `timeTriggered` (lines 150--203) | 44 | 44 | **100%** |

The custom strategies achieve **100% line coverage** of the `timeTriggered` entry
point, up from 45% with default generators. All 24 previously-uncovered lines are
now exercised.

**Note on `initialize` coverage:** The custom strategy tests do not test the
`initialize` entry point in isolation. The 100% coverage of `initialize` is a
side effect of the GUMBOX `testComputeCB` harness, which calls
`seL4_RxFirewall_RxFirewall_initialize()` before every compute test case (see
[`cb_apis.rs`](../crates/seL4_RxFirewall_RxFirewall/src/test/util/cb_apis.rs), line 67).

#### Per-Line Execution Counts (timeTriggered)

The following table shows execution counts for the inner routing branches of Rx0
(lines 157--166). The pattern is representative of all four channels.

| Line | Code | Hits |
|------|------|------|
| 157 | `if let Some(frame) = api.get_EthernetFramesRxIn0()` | 500 / 496 |
| 158 | `if let Some(eth) = get_frame_packet(&frame)` | 496 / 487 |
| 159 | `if can_send_to_mavlink(&eth.eth_type)` | 487 |
| 160 | `let output = udp_frame_from_raw_eth(frame);` | 120 |
| 161 | `api.put_MavlinkOut0(output);` | 120 |
| 162 | `} else if can_send_to_vmm(&eth.eth_type) {` | 367 |
| 163 | `api.put_VmmOut0(frame);` | 245 |
| 164 | `}` | 245 / 122 |

These counts confirm that all three routing outcomes are exercised: the MAVLink
forwarding path (lines 160--161), the VMM forwarding path (lines 163--164), and
the implicit drop path (when neither condition is true, control falls through
line 164 without sending).

### Coverage Analysis: Application Exec Helpers

Coverage of non-`verus!` exec helper functions in [`seL4_RxFirewall_RxFirewall_app.rs`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs).
Functions inside the `verus! { }` block (spec functions and config constants, lines
231--284) are excluded from this analysis.

| Function | Line | Entry Hits | Coverage | Notes |
|----------|------|-----------|----------|-------|
| [`new()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L14) | 14 | 500 | Full | Called once per test case |
| [`initialize()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L25) | 25 | 500 | Full | Logging only, no branches |
| [`timeTriggered()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L150) | 150 | 500 | **Full (100%)** | All routing branches exercised |
| [`get_frame_packet()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L304) | 304 | 1,965 | Full | Returns `Some` for ~98% of inputs |
| [`can_send_to_mavlink()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L320) | 320 | 1,923 | Full | All match arms exercised |
| [`udp_frame_from_raw_eth()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L338) | 338 | 474 | Full | Called for every MAVLink frame |
| [`udp_headers_from_raw_eth()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L351) | 351 | 474 | Full | Copies first 42 header bytes |
| [`udp_payload_from_raw_eth()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L375) | 375 | 474 | Full | Copies remaining payload bytes |
| [`can_send_to_vmm()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L403) | 403 | 1,449 | Full | ARP, IPv4 UDP, IPv6, and TCP arms exercised |
| [`udp_port_allowed()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L432) | 432 | 644 | Full | Called for non-MAVLink UDP |
| [`tcp_port_allowed()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L442) | 442 | 0 | **None** | TCP path goes through `can_send_to_vmm` `_ =>` arm |
| [`port_allowed()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L453) | 453 | 644 | Full | Scans allowlist, returns true or false |
| [`info_protocol()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L473) | 473 | 171 | Full | Logs dropped TCP packets |
| [`log_info()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L218) | 218 | 855 | Full | Called from `initialize` and `get_frame_packet` |

**Summary:** 13 of 14 exec helper functions are reached. The only unreached function
is `tcp_port_allowed()`, which is never called because the `can_send_to_vmm` match
for TCP packets hits the `_ =>` catch-all arm (which calls `info_protocol` and
returns false) rather than calling `tcp_port_allowed`.

**Observation -- possible TCP allowlist defect:** The `can_send_to_vmm` inner match
on `ip.protocol` has a specific arm only for `Udp`; all other IPv4 protocols
(including `Tcp`) fall through the `_ =>` catch-all and are unconditionally dropped.
This means `tcp_port_allowed()` and `config::tcp::ALLOWED_PORTS = [5760]` (MAVLink
TCP) are dead code -- no TCP packet can ever pass the firewall, regardless of its
destination port. Port 5760 is the standard MAVLink TCP port used by ground control
stations (e.g., QGroundControl) for TCP connections to ArduPilot. The existence of
the TCP allowlist with this port strongly suggests the *intent* was to permit
MAVLink-over-TCP traffic, but the implementation does not consult the allowlist.
If the ArduPilot deployment requires MAVLink TCP connectivity, `can_send_to_vmm`
would need a `Tcp(tcp)` match arm analogous to the `Udp(udp)` arm that calls
`tcp_port_allowed(tcp.dst_port)`. If TCP is intentionally blocked, the dead
`tcp_port_allowed` function and `config::tcp::ALLOWED_PORTS` should be removed to
avoid confusion.

**Overall file coverage:** 125/137 lines hit = **91.24%** (up from 28.47%)

### Coverage Analysis: GUMBOX Contract Methods

Coverage of the executable GUMBO contract functions in
[`seL4_RxFirewall_RxFirewall_GUMBOX.rs`](../crates/seL4_RxFirewall_RxFirewall/src/bridge/seL4_RxFirewall_RxFirewall_GUMBOX.rs). Each GUMBOX guarantee function uses the
`implies!` macro (`!$lhs || $rhs`), which short-circuits: the consequent (right-hand
side) is only evaluated when the antecedent (left-hand side) is true.

#### Per-Guarantee Consequent Coverage

| Guarantee | Rx0 | Rx1 | Rx2 | Rx3 | Consequent Reached? |
|-----------|-----|-----|-----|-----|---------------------|
| `hlr_05` (ARP -> VMM) | 125 | 115 | 125 | 120 | **Yes** -- `valid_arp()` true for ARP frames |
| `hlr_18` (MAVLink UDP) | 120 | 123 | 117 | 114 | **Yes** -- `valid_ipv4_udp_mavlink()` true for MAVLink frames |
| `hlr_13` (Whitelisted UDP) | 120 | 120 | 119 | 121 | **Yes** -- `valid_ipv4_udp_port()` true for DHCP frames |
| `hlr_15` (Disallow) | 131 | 128 | 133 | 134 | **Yes** -- `!rx_allow_outbound_frame()` true for disallowed frames |
| `hlr_17` (No input) | 500 | 500 | 500 | 500 | **Fully covered** -- non-short-circuit `\|` evaluates both sides |

All 22 GUMBOX functions are called 500 times each. All five guarantee families now
have their consequent lines exercised. The custom strategies produce inputs that
trigger the antecedent conditions for every guarantee, ensuring the postcondition
checks (the consequent expressions) are actually evaluated.

**Overall GUMBOX file coverage:** 227/227 lines hit = **100%**, 396/396 regions
covered = **100%**

#### Sub-Expression Region Coverage

The `implies!` macro expands to `!$lhs || $rhs`. With a compound antecedent of the
form `is_some() && validator(unwrap())`, the full expression becomes:

```
!(is_some() && validator(unwrap())) || consequent
```

LLVM instruments this as three regions:

| Region | Sub-Expression | Evaluates When |
|--------|---------------|----------------|
| R1 | `is_some()` | Always (function entry) |
| R2 | `validator(unwrap())` | Only when `is_some()` returns true (`&&` short-circuits on false) |
| R3 | consequent | Only when antecedent is true (`!antecedent` is false, so `\|\|` evaluates RHS) |

##### hlr_05 / hlr_18 / hlr_13 (ARP, MAVLink, Whitelisted UDP)

These three guarantee families share the same structure. The antecedent is
`is_some() && validator(unwrap())` where `validator` is `valid_arp`,
`valid_ipv4_udp_mavlink`, or `valid_ipv4_udp_port` respectively.

**hlr_05 (ARP -> VMM):**

| Region | Sub-Expression | Rx0 | Rx1 | Rx2 | Rx3 |
|--------|---------------|-----|-----|-----|-----|
| R1 | `is_some()` | 500 | 500 | 500 | 500 |
| R2 | `valid_arp(unwrap())` | 496 | 486 | 494 | 489 |
| R3 | consequent (output checks) | **125** | **115** | **125** | **120** |

**hlr_18 (MAVLink UDP):**

| Region | Sub-Expression | Rx0 | Rx1 | Rx2 | Rx3 |
|--------|---------------|-----|-----|-----|-----|
| R1 | `is_some()` | 500 | 500 | 500 | 500 |
| R2 | `valid_ipv4_udp_mavlink(unwrap())` | 496 | 486 | 494 | 489 |
| R3 | consequent (output checks) | **120** | **123** | **117** | **114** |

**hlr_13 (Whitelisted UDP):**

| Region | Sub-Expression | Rx0 | Rx1 | Rx2 | Rx3 |
|--------|---------------|-----|-----|-----|-----|
| R1 | `is_some()` | 500 | 500 | 500 | 500 |
| R2 | `valid_ipv4_udp_port(unwrap())` | 496 | 486 | 494 | 489 |
| R3 | consequent (output checks) | **120** | **120** | **119** | **121** |

All three regions are covered for all four channels. R2 counts match the number of
`Some` inputs per channel (mixed strategy generates ~98% `Some`). R3 counts match
the number of inputs where the specific validator returns true.

##### hlr_15 (Disallow)

The antecedent is `is_some() && !(rx_allow_outbound_frame(unwrap()))`.

| Region | Sub-Expression | Rx0 | Rx1 | Rx2 | Rx3 |
|--------|---------------|-----|-----|-----|-----|
| R1 | `is_some()` | 500 | 500 | 500 | 500 |
| R2 | `!(rx_allow_outbound_frame(unwrap()))` | 496 | 486 | 494 | 489 |
| R3 | `VmmOut.is_none() & MavlinkOut.is_none()` | **131** | **128** | **133** | **134** |

R3 is reached for all disallowed inputs: IPv6 frames, TCP on non-whitelisted ports,
UDP on non-whitelisted/non-MAVLink ports, and random unparseable bytes. Unlike the
default generators (which only tested unparseable bytes), the custom strategies
exercise `rx_allow_outbound_frame` returning false for *multiple distinct reasons*.

##### hlr_17 (No Input)

This guarantee uses non-short-circuit `|` (bitwise OR) directly:

```rust
api_EthernetFramesRxIn0.is_some() |
  api_VmmOut0.is_none() & api_MavlinkOut0.is_none()
```

| Region | Sub-Expression | Rx0 | Rx1 | Rx2 | Rx3 |
|--------|---------------|-----|-----|-----|-----|
| R1 | `is_some()` | 500 | 500 | 500 | 500 |
| R2 | `VmmOut.is_none() & MavlinkOut.is_none()` | 500 | 500 | 500 | 500 |

Both sides are always evaluated because `|` (bitwise OR) does not short-circuit.
All regions are fully covered.

##### Region Coverage Summary

| Guarantee | Total Regions | Regions Covered | Coverage |
|-----------|--------------|-----------------|----------|
| `hlr_05` (×4 channels) | 48 | 48 | **100%** |
| `hlr_18` (×4 channels) | 56 | 56 | **100%** |
| `hlr_13` (×4 channels) | 48 | 48 | **100%** |
| `hlr_15` (×4 channels) | 32 | 32 | **100%** |
| `hlr_17` (×4 channels) | 24 | 24 | **100%** |
| `compute_CEP_T_Guar` | 22 | 22 | **100%** |
| `compute_CEP_Post` | 2 | 2 | **100%** |
| **Total** | **232** | **232** | **100%** |

Note: The per-guarantee region totals are from the LLVM coverage report. Region
counts differ from the 3-region (R1/R2/R3) simplification above because LLVM
also creates regions for function entry, parameter evaluation, and `&` (bitwise
AND) sub-expressions within the consequent. Total file regions are 396/396 = 100%.

### Coverage Analysis: `firewall_core` Crate

Coverage of exec (non-`verus!`) functions in the `firewall_core` crate. Spec functions
inside `verus! { }` blocks (starting at `lib.rs` line 117 and `net.rs` line 549) are
excluded.

#### [`firewall_core::lib.rs`](../crates/firewall_core/src/lib.rs) -- Frame Parser

| Function | Line | Entry Hits | Notes |
|----------|------|-----------|-------|
| [`EthFrame::parse()`](../crates/firewall_core/src/lib.rs#L78) | 78 | 1,965 | Parses ARP (485), IPv4 (1,280), IPv6 (149), and fails for 42 invalid |

`EthFrame::parse` successfully parses the vast majority of inputs. The `match
header.ethertype` dispatch (line 80) exercises all three branches: `Arp` (485 hits),
`Ipv4` (1,280 hits), and `Ipv6` (149 hits). The early-return `?` at line 79 is hit
42 times (invalid EtherType from random-bytes inputs in the mixed strategy).

**lib.rs exec coverage:** 20/28 lines hit = **71.43%**

The 8 uncovered lines are the `Ipv4ProtoPacket` variants that the custom strategies
do not generate: `HopByHop`, `Icmp`, `Igmp`, `Ipv6Route`, `Ipv6Frag`, `Icmpv6`,
`Ipv6NoNxt`, `Ipv6Opts` (lines 95--102). The custom strategies only stamp TCP
(protocol 6) and UDP (protocol 17) because those are the only protocols the
RxFirewall's routing logic distinguishes. All other IP protocols follow the same
code path as TCP through `can_send_to_vmm`'s `_ =>` catch-all arm, which
unconditionally drops them. Generating these additional protocol numbers would
exercise more match arms in the `firewall_core` parser but would not test any
additional RxFirewall component behavior -- the firewall routes them identically.
A dedicated `firewall_core` unit test suite would be the appropriate place to
cover these parser branches.

#### [`firewall_core::net.rs`](../crates/firewall_core/src/net.rs) -- Protocol Parsers

| Function | Line | Entry Hits | Notes |
|----------|------|-----------|-------|
| [`Ipv4Address::from_bytes()`](../crates/firewall_core/src/net.rs#L23) | 23 | 970 | Called 2× per ARP parse (src + dst protocol addr) |
| [`Address::from_bytes()`](../crates/firewall_core/src/net.rs#L62) | 62 | 4,900 | Called 2× per `EthernetRepr::parse` + 2× per `Arp::parse` |
| [`Address::is_empty()`](../crates/firewall_core/src/net.rs#L86) | 86 | 1,965 | Dst MAC zero check; always returns false (stamped non-zero) |
| [`u16_from_be_bytes()`](../crates/firewall_core/src/net.rs#L116) | 116 | 7,110 | Called by EtherType, ArpOp, HardwareType, Ipv4Repr, TcpRepr, UdpRepr |
| [`EtherType::from_bytes()`](../crates/firewall_core/src/net.rs#L144) | 144 | 2,450 | Called in EthernetRepr::parse and Arp::parse |
| [`EtherType::try_from()`](../crates/firewall_core/src/net.rs#L154) | 154 | 2,450 | IPv4: 1,770; ARP: 485; IPv6: 149; Unknown: 42 |
| `EtherType::from()` (into) | 167 | 0 | **Never called** (reverse conversion, unused) |
| [`EthernetRepr::parse()`](../crates/firewall_core/src/net.rs#L202) | 202 | 1,965 | Returns `Some` for 1,923 inputs, `None` for 42 |
| [`ArpOp::from_bytes()`](../crates/firewall_core/src/net.rs#L240) | 240 | 485 | All 485 return `Request` |
| [`ArpOp::try_from()`](../crates/firewall_core/src/net.rs#L250) | 250 | 485 | All 485 hit `1 => Request` branch |
| [`ArpOp::from()`](../crates/firewall_core/src/net.rs#L262) | 262 | 0 | **Never called** (reverse conversion, unused) |
| [`HardwareType::from_bytes()`](../crates/firewall_core/src/net.rs#L290) | 290 | 485 | All 485 return `Ethernet` |
| [`HardwareType::try_from()`](../crates/firewall_core/src/net.rs#L300) | 300 | 485 | All 485 hit `1 => Ethernet` branch |
| [`HardwareType::from()`](../crates/firewall_core/src/net.rs#L311) | 311 | 0 | **Never called** (reverse conversion, unused) |
| [`Arp::parse()`](../crates/firewall_core/src/net.rs#L350) | 350 | 485 | All 485 return `Some` (all fields valid) |
| [`Arp::allowed_ptype()`](../crates/firewall_core/src/net.rs#L380) | 380 | 485 | All 485 return `true` (ptype is IPv4, not Arp) |
| [`IpProtocol::try_from()`](../crates/firewall_core/src/net.rs#L414) | 414 | 1,280 | TCP: 171; UDP: 1,109; others: 0 |
| [`IpProtocol::from()`](../crates/firewall_core/src/net.rs#L434) | 434 | 0 | **Never called** (reverse conversion, unused) |
| [`Ipv4Repr::parse()`](../crates/firewall_core/src/net.rs#L475) | 475 | 1,280 | All 1,280 return `Some` (valid IHL and length) |
| [`TcpRepr::parse()`](../crates/firewall_core/src/net.rs#L509) | 509 | 171 | Called for all TCP-protocol IPv4 frames |
| [`UdpRepr::parse()`](../crates/firewall_core/src/net.rs#L538) | 538 | 1,109 | Called for all UDP-protocol IPv4 frames |

**net.rs exec coverage:** 117/165 lines hit = **70.91%**

The 48 uncovered lines fall into two categories:
1. **Reverse conversion functions** (`EtherType::from`, `ArpOp::from`,
   `HardwareType::from`, `IpProtocol::from`) -- 4 functions, 20 lines. These
   convert enum variants back to numeric representations and are never called
   by the parse path.
2. **Uncommon IP protocol branches** in `IpProtocol::try_from` -- `HopByHop`,
   `Icmp`, `Igmp`, `Ipv6Route`, `Ipv6Frag`, `Icmpv6`, `Ipv6NoNxt`, `Ipv6Opts`,
   and `_ => Err(())`. The custom strategies only generate TCP (0x06) and UDP
   (0x11) protocols.
3. **`ArpOp::Reply`** (line 254) and the error branch (line 255) -- the ARP
   strategy always stamps `ArpOp::Request`.
4. **Unreachable `Address::is_empty` true path** (lines 98--101) -- all
   destination MACs are stamped non-zero.

#### `firewall_core` Coverage Summary

| File | Functions Hit | Total Functions | Lines Hit | Total Lines | Coverage |
|------|--------------|----------------|-----------|-------------|----------|
| [`lib.rs`](../crates/firewall_core/src/lib.rs) | 1/1 | 100% | 20/28 | **71.43%** |
| [`net.rs`](../crates/firewall_core/src/net.rs) | 17/21 | 81% | 117/165 | **70.91%** |
| **Total** | **18/22** | **82%** | **137/193** | **70.98%** |

The custom strategies exercise the full Ethernet -> IPv4/ARP -> TCP/UDP parsing
pipeline, including ARP field validation (`HardwareType`, `ArpOp`, `allowed_ptype`),
IPv4 header parsing (`Ipv4Repr::parse`), and both TCP and UDP payload parsing. The
remaining gaps are reverse conversion functions (unused by the parse path) and
uncommon IP protocol variants.

### Overall Coverage Summary

| Component | Lines Hit | Total Lines | Coverage |
|-----------|-----------|-------------|----------|
| App entry points + exec helpers | 125 | 137 | **91.24%** |
| GUMBOX contract methods | 227 | 227 | **100%** |
| [`firewall_core::lib.rs`](../crates/firewall_core/src/lib.rs) (exec) | 20 | 28 | **71.43%** |
| [`firewall_core::net.rs`](../crates/firewall_core/src/net.rs) (exec) | 117 | 165 | **70.91%** |
| **Total** | **489** | **557** | **87.79%** |

### Comparison with Default Generators

| Metric | Default Generators | Custom Strategies |
|--------|-------------------|-------------------|
| Total test cases | 100 (1 test) | 500 (5 tests) |
| `timeTriggered` coverage | 45% (20/44 lines) | **100%** (44/44 lines) |
| Routing branches exercised | None | All (MAVLink, VMM-ARP, VMM-UDP, drop) |
| GUMBO guarantee families tested | 1 of 5 (`hlr_17`) | **5 of 5** |
| GUMBOX region coverage | 75.86% (176/232) | **100%** (232/232) |
| App exec helpers coverage | 28.47% (39/137 lines) | **91.24%** (125/137 lines) |
| `firewall_core` coverage | 19.69% (38/193 lines) | **70.98%** (137/193 lines) |
| Overall coverage | 48.83% (272/557 lines) | **87.79%** (489/557 lines) |
| Postcondition failures | 0 | 0 |

---

## Robbie VanVossen Custom Strategies

### Motivation

Robbie VanVossen independently developed a domain-aware PropTest generator for the
RxFirewall as part of the INSPECTA project. His approach replaces the HAMR-generated
uniform random generator with a hierarchical strategy that mirrors the layered
structure of real Ethernet frames. Rather than stamping fixed header values onto
random arrays (as the Claude strategies do), Robbie's generator composes weighted
sub-strategies at each protocol layer, using `prop_flat_map` to select the
appropriate sub-parser strategy based on the value chosen at the layer above.

The original source is at:
[loonwerks/INSPECTA-models `tests.rs`](https://github.com/loonwerks/INSPECTA-models/blob/ffe80c4f3fae4531cfa3bfe68f5dd00d02d957ac/open-platform-models/open-platform/microkit/crates/seL4_RxFirewall_RxFirewall/src/tests.rs).
Only the compute entry point test (`testComputeCB_macro`) is included here; the
manual unit tests, `testInitializeCB_macro`, and `testComputeCBwLV_macro` were
excluded because we are comparing compute-entry-point strategy approaches only.

### Test Configuration

| Parameter | Value |
|-----------|-------|
| Target test cases | 100 |
| Reject ratio | 5 (max 500 rejected inputs) |
| Input ports | `EthernetFramesRxIn0`..`EthernetFramesRxIn3` |
| Option distribution | 50/50 `Some`/`None` (via `option_strategy_default`) |
| Test file | [`src/test/tests_robbiev_cust_stategies.rs`](../crates/seL4_RxFirewall_RxFirewall/src/test/tests_robbiev_cust_stategies.rs) |
| Test name prefix | `prop_robbiev_cust_stategies_*` |

### Strategy Architecture

Robbie's approach builds frames from the top down using `prop_flat_map` to chain
layer-dependent decisions:

```
ethertype_strategy()                    -- choose EtherType
  |
  +-- 0x0800 (IPv4, weight 20) ------> ipv4_strategy()
  |                                       |
  |                                       +-- ipv4_protocol_strategy()  -- choose IP protocol
  |                                       |     |
  |                                       |     +-- 0x06 (TCP, weight 10) --> tcp_strategy()
  |                                       |     +-- 0x11 (UDP, weight 10) --> udp_strategy()
  |                                       |     +-- other (weight ~33)    --> default_packet_strategy()
  |                                       |
  |                                       +-- ipv4_length_strategy()    -- total length
  |
  +-- 0x0806 (ARP, weight 10) -------> arp_strategy()
  |                                       |
  |                                       +-- arp_hwtype_strategy()     -- hardware type
  |                                       +-- arp_ethertype_strategy()  -- ptype field
  |                                       +-- arp_op_strategy()         -- operation code
  |
  +-- 0x86DD (IPv6, weight 2) -------> default_packet_strategy()
  |
  +-- random u16 (weight 1) ---------> default_packet_strategy()
```

The final assembly (`SW_RawEthernetMessage_stategy_cust`) combines:
1. A destination MAC from `dst_mac_strategy` (98% random non-zero, 2% all-zeros)
2. The chosen EtherType at bytes 12--13
3. The layer-specific payload spliced in at byte 14+
4. A random 1600-byte base array for all remaining bytes

### Layer-Specific Weight Distributions

#### EtherType (frame bytes 12--13)

| Value | Protocol | Weight | Approximate % |
|-------|----------|--------|---------------|
| `0x0800` | IPv4 | 20 | 61% |
| `0x0806` | ARP | 10 | 30% |
| `0x86DD` | IPv6 | 2 | 6% |
| random `u16` | unparseable | 1 | 3% |

#### IPv4 Protocol (IPv4 header byte 9)

| Value | Protocol | Weight | Approximate % |
|-------|----------|--------|---------------|
| `0x06` | TCP | 10 | 19% |
| `0x11` | UDP | 10 | 19% |
| `0x00` | HopByHop | 4 | 7% |
| `0x01` | ICMP | 4 | 7% |
| `0x02` | IGMP | 4 | 7% |
| `0x2b` | IPv6Route | 4 | 7% |
| `0x2c` | IPv6Frag | 4 | 7% |
| `0x3a` | ICMPv6 | 4 | 7% |
| `0x3b` | IPv6NoNxt | 4 | 7% |
| `0x3c` | IPv6Opts | 4 | 7% |
| random `u8` | other | 1 | 2% |

#### UDP Destination Port (UDP header bytes 2--3)

| Value | Meaning | Weight | Approximate % |
|-------|---------|--------|---------------|
| 68 | DHCP client (whitelisted) | 1 | 20% |
| random `u16` | any port | 4 | 80% |

The UDP source port is left entirely random. This means the probability of
generating both `src=14550` and `dst=14562` (the MAVLink port pair) is
approximately (1/65536) x (1/5) = ~0.0003%, which is effectively zero over
100 test cases.

#### TCP Destination Port (TCP header bytes 2--3)

| Value | Meaning | Weight | Approximate % |
|-------|---------|--------|---------------|
| 5760 | whitelisted TCP port | 1 | 20% |
| random `u16` | any port | 4 | 80% |

#### ARP Fields

| Field | Valid Values | Valid Weight | Random Weight |
|-------|-------------|-------------|---------------|
| Hardware type | `0x0001` (Ethernet) | 40 | 1 |
| ptype | `0x0800` (20), `0x0806` (2), `0x86DD` (20) | 42 | 1 |
| Operation | `0x0001` Request (20), `0x0002` Reply (20) | 40 | 1 |

#### Destination MAC (frame bytes 0--5)

| Value | Weight | Approximate % |
|-------|--------|---------------|
| random 6 bytes | 50 | 98% |
| `00:00:00:00:00:00` | 1 | 2% |

### Tests Executed

| Test | Description |
|------|-------------|
| `prop_robbiev_cust_stategies_testComputeCB_macro` | Domain-aware hierarchical generator for all four Rx ports with 50/50 Some/None; exercises ARP, IPv4 (TCP/UDP), IPv6, and unparseable frame paths |

### PropTest Generation Statistics

| Test | Valid (Passed) | Rejected (Precondition) | Failed (Postcondition) | Total Generated |
|------|---------------|------------------------|----------------------|-----------------|
| `testComputeCB_macro` | 100 | 0 | 0 | 100 |

All 100 generated test vectors passed. No inputs were rejected and no
postcondition violations were detected.

### Test Results

| Test | Result | Cases |
|------|--------|-------|
| `prop_robbiev_cust_stategies_testComputeCB_macro` | **PASS** | 100/100 |

### Coverage Analysis: Entry Points

| Entry Point | Lines Hit | Total Instrumented Lines | Coverage |
|-------------|-----------|-------------------------|----------|
| `initialize` (lines 25--30) | 5 | 5 | **100%** |
| `timeTriggered` (lines 150--203) | 36 | 44 | **82%** |

**Note on `initialize` coverage:** As with the Claude custom strategies, the
100% coverage of `initialize` is a side effect of the `testComputeCB` harness
calling `seL4_RxFirewall_RxFirewall_initialize()` before every compute test
case, not a dedicated initialize test.

#### Per-Line Execution Counts (timeTriggered)

The following table shows execution counts for the inner routing branches of Rx0
(lines 157--166). The pattern is representative of all four channels.

| Line | Code | Hits |
|------|------|------|
| 157 | `if let Some(frame) = api.get_EthernetFramesRxIn0()` | 100 / 45 |
| 158 | `if let Some(eth) = get_frame_packet(&frame)` | 45 / 14 |
| 159 | `if can_send_to_mavlink(&eth.eth_type)` | 14 |
| 160 | `let output = udp_frame_from_raw_eth(frame);` | **0** |
| 161 | `api.put_MavlinkOut0(output);` | **0** |
| 162 | `} else if can_send_to_vmm(&eth.eth_type) {` | 14 |
| 163 | `api.put_VmmOut0(frame);` | 11 |
| 164 | `}` | 11 / 3 |
| 165 | `}` | 31 |
| 166 | `}` | 55 |

Lines 160--161 (the MAVLink forwarding path) have zero hits. The same pattern
repeats for Rx1 (lines 172--173), Rx2 (lines 184--185), and Rx3 (lines 196--197),
for a total of **8 uncovered lines**.

#### Uncovered Lines

```rust
160:          let output = udp_frame_from_raw_eth(frame);
161:          api.put_MavlinkOut0(output);
172:          let output = udp_frame_from_raw_eth(frame);
173:          api.put_MavlinkOut1(output);
184:          let output = udp_frame_from_raw_eth(frame);
185:          api.put_MavlinkOut2(output);
196:          let output = udp_frame_from_raw_eth(frame);
197:          api.put_MavlinkOut3(output);
```

All 8 uncovered lines are the MAVLink forwarding path: `udp_frame_from_raw_eth`
followed by `put_MavlinkOut*`.

#### Root Cause Analysis

The MAVLink path requires `can_send_to_mavlink` to return true, which checks that
the frame is a UDP packet with source port 14550 (`MAV_UDP_SRC_PORT`) **and**
destination port 14562 (`MAV_UDP_DST_PORT`).

However, `can_send_to_mavlink` receives **zero** IPv4 frames. The coverage data
shows 0 hits on the `Ipv4(ip)` match arm at line 321 across all four channels.
This is because all IPv4 frames fail parsing in `Ipv4Repr::parse` at the IHL
byte check (`packet[0] != 0x45`, line 478 of `net.rs`). Robbie's `ipv4_strategy`
stamps the protocol byte and total length but does **not** stamp byte 14 of the
full frame (byte 0 of the IPv4 header) to `0x45` (version 4, IHL 5). Since the
base array is random, only ~1/256 IPv4 frames would have the correct IHL byte
by chance, and with the small sample size (roughly 120 IPv4-designated frames
across all channels), the probability of any passing is low. The fresh coverage
data confirms that **zero** IPv4 frames completed parsing.

This has two consequences:

1. **MAVLink path (hlr_18) -- not exercised.** Even if the MAVLink port pair
   were generated, the IPv4 frame would fail parsing before the port check.

2. **Whitelisted UDP path (hlr_13) -- not exercised.** The `can_send_to_vmm`
   inner `Udp(udp)` match arm shows 0 hits (line 407), confirming no IPv4 UDP
   frames reach the allowlist check.

#### Covered Lines -- What Is Exercised

Despite missing the entire IPv4 path, Robbie's generator achieves coverage of
the ARP and drop branches:

- **Frame parsing** (line 158): 45 hits on Rx0, indicating ~45% of `Some` inputs
  produce a parseable frame. Of these, all are either ARP or IPv6 (not IPv4).

- **ARP -> VMM forwarding** (lines 163--164): 11 hits on Rx0, confirming valid ARP
  frames reach `put_VmmOut0`. This exercises `hlr_05`.

- **Disallowed frame drop** (implicit fall-through at line 164): 3 hits on Rx0
  where frames parse but match neither MAVLink nor VMM conditions (IPv6 frames).
  This exercises `hlr_15`.

- **No input / unparseable** (lines 165--166): `None` inputs and unparseable
  frames produce no output. This exercises `hlr_17`.

### Coverage Analysis: Application Exec Helpers

Coverage of non-`verus!` exec helper functions in [`seL4_RxFirewall_RxFirewall_app.rs`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs).
Functions inside the `verus! { }` block (spec functions and config constants, lines
231--284) are excluded from this analysis.

| Function | Line | Entry Hits | Coverage | Notes |
|----------|------|-----------|----------|-------|
| [`new()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L14) | 14 | 100 | Full | Called once per test case |
| [`initialize()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L25) | 25 | 100 | Full | Logging only, no branches |
| [`timeTriggered()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L150) | 150 | 100 | **82%** (36/44) | MAVLink path uncovered |
| [`get_frame_packet()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L304) | 304 | 195 | Full | Returns `Some` for 61, `None` for 134 |
| [`can_send_to_mavlink()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L320) | 320 | 61 | **Partial** | IPv4 match arm: 0 hits; only `_ => false` exercised |
| [`udp_frame_from_raw_eth()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L338) | 338 | 0 | **None** | Never called (no MAVLink frames) |
| [`udp_headers_from_raw_eth()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L351) | 351 | 0 | **None** | Never called |
| [`udp_payload_from_raw_eth()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L375) | 375 | 0 | **None** | Never called |
| [`can_send_to_vmm()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L403) | 403 | 61 | **Partial** | ARP: 46; IPv4: 0; IPv6: 15 |
| [`udp_port_allowed()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L432) | 432 | 0 | **None** | Never called (no IPv4 UDP reaches allowlist check) |
| [`tcp_port_allowed()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L442) | 442 | 0 | **None** | TCP path goes through `_ =>` catch-all |
| [`port_allowed()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L453) | 453 | 0 | **None** | Never called |
| [`info_protocol()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L473) | 473 | 0 | **None** | Never called (no IPv4 non-UDP reaches `_ =>` arm) |
| [`log_info()`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L218) | 218 | 249 | Full | Called from `initialize` and `get_frame_packet` |

**Summary:** 7 of 14 exec helper functions are reached. The 7 unreached functions
are all in the IPv4 processing pipeline: `udp_frame_from_raw_eth`,
`udp_headers_from_raw_eth`, `udp_payload_from_raw_eth`, `udp_port_allowed`,
`tcp_port_allowed`, `port_allowed`, and `info_protocol`. All of these require a
successfully parsed IPv4 frame, which Robbie's strategy does not produce due to
the missing IHL byte stamp.

**Overall file coverage:** 67/137 lines hit = **48.91%** (192 regions, 87 covered = 45.31%)

### Coverage Analysis: GUMBOX Contract Methods

Coverage of the executable GUMBO contract functions in
[`seL4_RxFirewall_RxFirewall_GUMBOX.rs`](../crates/seL4_RxFirewall_RxFirewall/src/bridge/seL4_RxFirewall_RxFirewall_GUMBOX.rs). Each GUMBOX guarantee function uses the
`implies!` macro (`!$lhs || $rhs`), which short-circuits: the consequent (right-hand
side) is only evaluated when the antecedent (left-hand side) is true.

#### Per-Guarantee Consequent Coverage

| Guarantee | Rx0 | Rx1 | Rx2 | Rx3 | Consequent Reached? |
|-----------|-----|-----|-----|-----|---------------------|
| `hlr_05` (ARP -> VMM) | 11 | 9 | 12 | 14 | **Yes** -- `valid_arp()` true for ARP frames |
| `hlr_18` (MAVLink UDP) | 0 | 0 | 0 | 0 | **No** -- `valid_ipv4_udp_mavlink()` never true (no IPv4 frames parse) |
| `hlr_13` (Whitelisted UDP) | 0 | 0 | 0 | 0 | **No** -- `valid_ipv4_udp_port()` never true (no IPv4 frames parse) |
| `hlr_15` (Disallow) | 34 | 40 | 38 | 37 | **Yes** -- `!rx_allow_outbound_frame()` true for non-ARP parseable + unparseable frames |
| `hlr_17` (No input) | 100 | 100 | 100 | 100 | **Fully covered** -- non-short-circuit `|` evaluates both sides |

All 22 GUMBOX functions are called 100 times each. Only three of five guarantee
families have their consequent lines exercised: `hlr_05` (ARP), `hlr_15` (disallow),
and `hlr_17` (no input). The `hlr_18` (MAVLink UDP) and `hlr_13` (whitelisted UDP)
consequents are never reached because no IPv4 frames successfully parse.

**Overall GUMBOX file coverage:** 207/227 lines hit = **91.19%**, 344/396 regions
covered = **86.87%**

#### Sub-Expression Region Coverage

The `implies!` macro expands to `!$lhs || $rhs`. With a compound antecedent of the
form `is_some() && validator(unwrap())`, the full expression becomes:

```
!(is_some() && validator(unwrap())) || consequent
```

LLVM instruments this as three regions:

| Region | Sub-Expression | Evaluates When |
|--------|---------------|----------------|
| R1 | `is_some()` | Always (function entry) |
| R2 | `validator(unwrap())` | Only when `is_some()` returns true (`&&` short-circuits on false) |
| R3 | consequent | Only when antecedent is true (`!antecedent` is false, so `\|\|` evaluates RHS) |

##### hlr_05 (ARP -> VMM)

| Region | Sub-Expression | Rx0 | Rx1 | Rx2 | Rx3 |
|--------|---------------|-----|-----|-----|-----|
| R1 | `is_some()` | 100 | 100 | 100 | 100 |
| R2 | `valid_arp(unwrap())` | 45 | 49 | 50 | 51 |
| R3 | consequent (output checks) | **11** | **9** | **12** | **14** |

All three regions are covered. R2 counts match the number of `Some` inputs per
channel. R3 counts match the number of ARP frames that pass all ARP validation
checks in `firewall_core` (see Root Cause Analysis below for why this is fewer
than the raw ARP EtherType count).

##### hlr_18 (MAVLink UDP) and hlr_13 (Whitelisted UDP)

These two guarantee families share the same pattern: the antecedent requires a
valid IPv4 UDP frame, but no IPv4 frames pass `Ipv4Repr::parse`.

**hlr_18 (MAVLink UDP):**

| Region | Sub-Expression | Rx0 | Rx1 | Rx2 | Rx3 |
|--------|---------------|-----|-----|-----|-----|
| R1 | `is_some()` | 100 | 100 | 100 | 100 |
| R2 | `valid_ipv4_udp_mavlink(unwrap())` | 45 | 49 | 50 | 51 |
| R3 | consequent (output checks) | **0** | **0** | **0** | **0** |

**hlr_13 (Whitelisted UDP):**

| Region | Sub-Expression | Rx0 | Rx1 | Rx2 | Rx3 |
|--------|---------------|-----|-----|-----|-----|
| R1 | `is_some()` | 100 | 100 | 100 | 100 |
| R2 | `valid_ipv4_udp_port(unwrap())` | 45 | 49 | 50 | 51 |
| R3 | consequent (output checks) | **0** | **0** | **0** | **0** |

R2 is reached (the validator functions are called for all `Some` inputs), but the
validators always return false because no IPv4 frame has a valid IHL byte. This
means the antecedent is always false, so `!antecedent` is true, and the `||`
short-circuits -- the consequent is never evaluated.

##### hlr_15 (Disallow)

The antecedent is `is_some() && !(rx_allow_outbound_frame(unwrap()))`.

| Region | Sub-Expression | Rx0 | Rx1 | Rx2 | Rx3 |
|--------|---------------|-----|-----|-----|-----|
| R1 | `is_some()` | 100 | 100 | 100 | 100 |
| R2 | `!(rx_allow_outbound_frame(unwrap()))` | 45 | 49 | 50 | 51 |
| R3 | `VmmOut.is_none() & MavlinkOut.is_none()` | **34** | **40** | **38** | **37** |

R3 is reached for IPv6 frames (which parse but are not allowed) and for IPv4
frames that fail parsing (treated as disallowed by the GUMBOX validator since the
raw bytes do not match any "allowed" spec pattern). The consequent count is higher
than the ARP consequent count because disallowed frames outnumber valid ARP frames
in the generated distribution.

##### hlr_17 (No Input)

This guarantee uses non-short-circuit `|` (bitwise OR) directly:

```rust
api_EthernetFramesRxIn0.is_some() |
  api_VmmOut0.is_none() & api_MavlinkOut0.is_none()
```

| Region | Sub-Expression | Rx0 | Rx1 | Rx2 | Rx3 |
|--------|---------------|-----|-----|-----|-----|
| R1 | `is_some()` | 100 | 100 | 100 | 100 |
| R2 | `VmmOut.is_none() & MavlinkOut.is_none()` | 100 | 100 | 100 | 100 |

Both sides are always evaluated because `|` (bitwise OR) does not short-circuit.
All regions are fully covered.

##### Region Coverage Summary

| Guarantee | Total Regions | Regions Covered | Coverage |
|-----------|--------------|-----------------|----------|
| `hlr_05` (×4 channels) | 48 | 48 | **100%** |
| `hlr_18` (×4 channels) | 56 | 40 | **71%** |
| `hlr_13` (×4 channels) | 48 | 32 | **67%** |
| `hlr_15` (×4 channels) | 32 | 32 | **100%** |
| `hlr_17` (×4 channels) | 24 | 24 | **100%** |
| `compute_CEP_T_Guar` | 22 | 22 | **100%** |
| `compute_CEP_Post` | 2 | 2 | **100%** |
| **Total** | **232** | **200** | **86%** |

The 32 uncovered regions are all R3 (consequent) regions from `hlr_18` (16 regions
across 4 channels) and `hlr_13` (16 regions across 4 channels). These consequent
expressions are never evaluated because no IPv4 frames pass parsing.

Note: Total file regions per LLVM report are 396, of which 344 are covered (86.87%).
The per-guarantee totals above use the simplified 3-region model; the LLVM total
includes additional regions for function entry, parameter evaluation, and `&`
sub-expressions within consequents.

### Coverage Analysis: `firewall_core` Crate

Coverage of exec (non-`verus!`) functions in the `firewall_core` crate. Spec functions
inside `verus! { }` blocks (starting at `lib.rs` line 117 and `net.rs` line 549) are
excluded.

#### [`firewall_core::lib.rs`](../crates/firewall_core/src/lib.rs) -- Frame Parser

| Function | Line | Entry Hits | Notes |
|----------|------|-----------|-------|
| [`EthFrame::parse()`](../crates/firewall_core/src/lib.rs#L78) | 78 | 195 | ARP: 54 (46 valid); IPv4: 118 (**all fail IHL**); IPv6: 15; early-return: 8 |

`EthFrame::parse` successfully parses 61 of 195 inputs. The `match
header.ethertype` dispatch (line 80) exercises all three branches: `Arp` (54 hits,
46 return `Some`), `Ipv4` (118 hits, **all return `None`** at line 86→0), and
`Ipv6` (15 hits). The early-return `?` at line 79 is hit 8 times (invalid
EtherType or empty destination MAC).

The critical finding is at line 86: `Ipv4Repr::parse` returns `None` for all 118
IPv4 frames. The coverage annotation `^0` on the inner line confirms that zero
IPv4 frames proceed past `Ipv4Repr::parse`. Lines 88--108 (the `match
ip.protocol` dispatch and `PacketType::Ipv4` construction) all show 0 hits.

**lib.rs exec coverage:** 9/28 lines hit = **32.14%**

The 19 uncovered lines are the entire IPv4 protocol dispatch block (lines 88--108):
`HopByHop`, `Icmp`, `Igmp`, `Tcp`, `Udp`, `Ipv6Route`, `Ipv6Frag`, `Icmpv6`,
`Ipv6NoNxt`, `Ipv6Opts`, and the `PacketType::Ipv4(Ipv4Packet { ... })` construction.
All of these require `Ipv4Repr::parse` to return `Some`, which never happens.

#### [`firewall_core::net.rs`](../crates/firewall_core/src/net.rs) -- Protocol Parsers

| Function | Line | Entry Hits | Notes |
|----------|------|-----------|-------|
| [`Ipv4Address::from_bytes()`](../crates/firewall_core/src/net.rs#L23) | 23 | 92 | Called 2× per valid ARP parse (src + dst protocol addr) |
| [`Address::from_bytes()`](../crates/firewall_core/src/net.rs#L62) | 62 | 479 | Called 2× per `EthernetRepr::parse` + 2× per `Arp::parse` |
| [`Address::is_empty()`](../crates/firewall_core/src/net.rs#L86) | 86 | 195 | 192 return false, 3 return true (2% all-zeros MAC) |
| [`u16_from_be_bytes()`](../crates/firewall_core/src/net.rs#L116) | 116 | 463 | EtherType, ArpOp, HardwareType, Ipv4Repr fields |
| [`EtherType::from_bytes()`](../crates/firewall_core/src/net.rs#L144) | 144 | 245 | Called in EthernetRepr::parse and Arp::parse |
| [`EtherType::try_from()`](../crates/firewall_core/src/net.rs#L154) | 154 | 245 | IPv4: 144; ARP: 58; IPv6: 37; Unknown: 6 |
| `EtherType::from()` (into) | 167 | 0 | **Never called** (reverse conversion, unused) |
| [`EthernetRepr::parse()`](../crates/firewall_core/src/net.rs#L202) | 202 | 195 | 187 return `Some`, 8 fail (3 empty MAC, 5 bad EtherType) |
| [`ArpOp::from_bytes()`](../crates/firewall_core/src/net.rs#L240) | 240 | 48 | Called for ARP frames that pass HardwareType and ptype checks |
| [`ArpOp::try_from()`](../crates/firewall_core/src/net.rs#L250) | 250 | 48 | Request: 21; Reply: 25; error: 2 |
| [`ArpOp::from()`](../crates/firewall_core/src/net.rs#L262) | 262 | 0 | **Never called** (reverse conversion, unused) |
| [`HardwareType::from_bytes()`](../crates/firewall_core/src/net.rs#L290) | 290 | 54 | Called for all ARP frames |
| [`HardwareType::try_from()`](../crates/firewall_core/src/net.rs#L300) | 300 | 54 | Ethernet: 53; error: 1 |
| [`HardwareType::from()`](../crates/firewall_core/src/net.rs#L311) | 311 | 0 | **Never called** (reverse conversion, unused) |
| [`Arp::parse()`](../crates/firewall_core/src/net.rs#L350) | 350 | 54 | 46 return `Some`, 8 fail (1 bad htype, 1 bad ptype EtherType, 4 ptype=Arp, 2 bad op) |
| [`Arp::allowed_ptype()`](../crates/firewall_core/src/net.rs#L380) | 380 | 52 | 48 return true, 4 return false (ptype is Arp) |
| [`IpProtocol::try_from()`](../crates/firewall_core/src/net.rs#L414) | 414 | 118 | **All 10 variants + error branch hit** |
| [`IpProtocol::from()`](../crates/firewall_core/src/net.rs#L434) | 434 | 0 | **Never called** (reverse conversion, unused) |
| [`Ipv4Repr::parse()`](../crates/firewall_core/src/net.rs#L475) | 475 | 118 | Protocol parsed for 116; **all 116 fail at IHL byte check** |
| [`TcpRepr::parse()`](../crates/firewall_core/src/net.rs#L509) | 509 | 0 | **Never reached** (no valid IPv4) |
| [`UdpRepr::parse()`](../crates/firewall_core/src/net.rs#L538) | 538 | 0 | **Never reached** (no valid IPv4) |

**net.rs exec coverage:** 123/165 lines hit = **74.55%**

A notable strength of Robbie's strategy: `IpProtocol::try_from` exercises **all 10
named protocol variants** (HopByHop: 9, Icmp: 8, Igmp: 7, Tcp: 22, Udp: 23,
Ipv6Route: 7, Ipv6Frag: 8, Icmpv6: 12, Ipv6NoNxt: 15, Ipv6Opts: 5) plus the
error branch (2), thanks to `ipv4_protocol_strategy` explicitly weighting all
protocol values. However, all this parsing work is discarded because `Ipv4Repr::parse`
fails at the IHL byte check immediately afterward.

The 42 uncovered lines fall into three categories:
1. **Reverse conversion functions** (`EtherType::from`, `ArpOp::from`,
   `HardwareType::from`, `IpProtocol::from`) -- 4 functions, ~27 lines. These
   convert enum variants back to numeric representations and are never called
   by the parse path.
2. **`Ipv4Repr::parse` post-IHL lines** (lines 480--484) -- the `Some(...)` return
   that constructs the Ipv4Repr struct. Never reached because the IHL check always
   fails.
3. **`TcpRepr::parse`** and **`UdpRepr::parse`** -- 9 lines. Never reached because
   no IPv4 frames complete parsing.

#### `firewall_core` Coverage Summary

| File | Functions Hit | Total Functions | Lines Hit | Total Lines | Coverage |
|------|--------------|----------------|-----------|-------------|----------|
| [`lib.rs`](../crates/firewall_core/src/lib.rs) | 1/1 | 100% | 9/28 | **32.14%** |
| [`net.rs`](../crates/firewall_core/src/net.rs) | 15/21 | 71% | 123/165 | **74.55%** |
| **Total** | **16/22** | **73%** | **132/193** | **68.39%** |

Robbie's hierarchical strategy exercises the Ethernet parsing pipeline and the full
ARP validation chain (HardwareType, ptype, ArpOp), plus all IPv4 protocol variant
parsing. The critical gap is the missing IHL byte stamp (`v[0] = 0x45` at frame
offset 14), which causes every IPv4 frame to fail `Ipv4Repr::parse`, preventing
any TCP or UDP payload parsing.

### Overall Coverage Summary

| Component | Lines Hit | Total Lines | Coverage |
|-----------|-----------|-------------|----------|
| App entry points + exec helpers | 67 | 137 | **48.91%** |
| GUMBOX contract methods | 207 | 227 | **91.19%** |
| [`firewall_core::lib.rs`](../crates/firewall_core/src/lib.rs) (exec) | 9 | 28 | **32.14%** |
| [`firewall_core::net.rs`](../crates/firewall_core/src/net.rs) (exec) | 123 | 165 | **74.55%** |
| **Total** | **406** | **557** | **72.89%** |

### GUMBO Guarantee Coverage

| Guarantee | Requirement | Exercised? |
|-----------|-------------|------------|
| `hlr_05_rx{0..3}_can_send_arp_to_vmm` | HLR-05 | Yes |
| `hlr_18_rx{0..3}_can_send_mavlink_udp` | HLR-18 | **No** |
| `hlr_13_rx{0..3}_can_send_ipv4_udp` | HLR-13 | **No** |
| `hlr_15_rx{0..3}_disallow` | HLR-15 | Yes |
| `hlr_17_rx{0..3}_no_input` | HLR-17 | Yes |

3 of 5 GUMBO guarantee families are exercised. `hlr_18` (MAVLink UDP forwarding)
and `hlr_13` (whitelisted UDP forwarding) are both not covered because no IPv4
frames pass `Ipv4Repr::parse`.

### Comparison with Default Generators

| Metric | Default Generators | Robbie V Custom Strategies |
|--------|-------------------|---------------------------|
| Total test cases | 100 (1 test) | 100 (1 test) |
| `timeTriggered` coverage | 45% (20/44 lines) | **82%** (36/44 lines) |
| Routing branches exercised | None | ARP, disallowed, drop |
| GUMBO guarantee families tested | 1 of 5 (`hlr_17`) | **3 of 5** |
| GUMBOX region coverage | 75.86% (176/232) | **86%** (200/232) |
| App exec helpers coverage | 28.47% (39/137 lines) | **48.91%** (67/137 lines) |
| `firewall_core` coverage | 19.69% (38/193 lines) | **68.39%** (132/193 lines) |
| Overall coverage | 48.83% (272/557 lines) | **72.89%** (406/557 lines) |
| Postcondition failures | 0 | 0 |

---

## Comparison: Claude vs Robbie Custom Strategies

### Design Philosophy

| Aspect | Claude | Robbie |
|--------|--------|--------|
| Approach | **Fixed-stamp overlay**: starts with a random 1600-byte array and stamps the minimum bytes needed for each routing category | **Hierarchical composition**: builds frames layer by layer using `prop_flat_map` chains that select sub-strategies based on upper-layer choices |
| Frame construction | `prop_map` on the default HAMR generator — stamps specific byte offsets, leaves everything else random | `prop_flat_map` tree: `ethertype_strategy` → `ipv4_strategy` / `arp_strategy` → `tcp_strategy` / `udp_strategy`, assembled onto a random base array |
| Test organization | 5 separate tests, each targeting one routing category (ARP, MAVLink, whitelisted UDP, disallowed, mixed) | 1 combined test using a single weighted generator that covers all categories simultaneously |
| Strategy derivation | Byte offsets derived by reading `firewall_core` parser source code and application routing logic | Protocol layer structure mirrors real Ethernet frame composition; weights based on relative frequency assumptions |

### Coverage Summary

| Metric | Claude | Robbie |
|--------|--------|--------|
| Total test cases | 500 (5 tests × 100) | 100 (1 test) |
| Total lines covered | **489/557 (87.79%)** | 406/557 (72.89%) |
| App entry points + exec helpers | **125/137 (91.24%)** | 67/137 (48.91%) |
| GUMBOX line coverage | **227/227 (100%)** | 207/227 (91.19%) |
| GUMBOX region coverage | **396/396 (100%)** | 344/396 (86.87%) |
| [`firewall_core::lib.rs`](../crates/firewall_core/src/lib.rs) | 20/28 (71.43%) | 9/28 (32.14%) |
| [`firewall_core::net.rs`](../crates/firewall_core/src/net.rs) | **117/165 (70.91%)** | 123/165 (74.55%) |
| `firewall_core` total | **137/193 (70.98%)** | 132/193 (68.39%) |

### GUMBO Guarantee Exercised

| Guarantee Family | Claude | Robbie |
|-----------------|--------|--------|
| hlr_05 (ARP → VMM) | Yes | Yes |
| hlr_18 (MAVLink UDP → MavlinkOut) | **Yes** | No |
| hlr_13 (Whitelisted UDP → VMM) | **Yes** | No |
| hlr_15 (Disallowed → drop) | Yes | Yes |
| hlr_17 (No input → no output) | Yes | Yes |
| **Families exercised** | **5 of 5** | 3 of 5 |

### Routing Branch Coverage (timeTriggered)

| Branch | Claude | Robbie |
|--------|--------|--------|
| MAVLink forwarding (`put_MavlinkOut*`) | **Covered** (lines 160--161 hit) | Not covered (0 hits) |
| VMM ARP forwarding (`put_VmmOut*`) | **Covered** | **Covered** |
| VMM whitelisted UDP forwarding | **Covered** | Not covered |
| Disallowed frame drop | **Covered** | **Covered** |
| No-input path | **Covered** | **Covered** |
| `timeTriggered` line coverage | **44/44 (100%)** | 36/44 (82%) |

### Application Exec Helper Coverage

| Function | Claude | Robbie |
|----------|--------|--------|
| [`get_frame_packet`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L304) | 1,965 hits | 195 hits |
| [`can_send_to_mavlink`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L320) | 1,877 hits (IPv4 branch: **1,280**) | 61 hits (IPv4 branch: **0**) |
| [`can_send_to_vmm`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L403) (UDP inner arm) | **1,109 hits** | **0 hits** |
| [`udp_frame_from_raw_eth`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L338) | **474 hits** | 0 hits |
| [`udp_headers_from_raw_eth`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L351) | **474 hits** | 0 hits |
| [`udp_payload_from_raw_eth`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L375) | **474 hits** | 0 hits |
| [`udp_port_allowed`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L432) | **1,109 hits** | 0 hits |
| [`tcp_port_allowed`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L442) | **0 hits** | 0 hits |
| [`port_allowed`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L453) | **1,109 hits** | 0 hits |
| [`info_protocol`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L473) | **171 hits** | 0 hits |

Both strategies leave `tcp_port_allowed` at 0 hits. In the application code, `can_send_to_vmm`
only has a `Udp(udp)` arm — TCP packets hit the `_ =>` catch-all and are dropped before
reaching the TCP allowlist, making `tcp_port_allowed` dead code in the current implementation.

### firewall_core Parser Coverage

| Parser Function | Claude | Robbie | Advantage |
|----------------|--------|--------|-----------|
| [`IpProtocol::try_from`](../crates/firewall_core/src/net.rs#L414) variants | TCP + UDP only (2/10) | **All 10 + error** | Robbie |
| [`ArpOp::try_from`](../crates/firewall_core/src/net.rs#L250) variants | Request only (1/2) | **Request + Reply + error** | Robbie |
| [`HardwareType::try_from`](../crates/firewall_core/src/net.rs#L300) variants | Ethernet only | Ethernet + error | Robbie |
| [`Arp::parse`](../crates/firewall_core/src/net.rs#L350) failure paths | All pass (0 failures) | 8 failures (htype, ptype, op) | Robbie |
| [`Address::is_empty`](../crates/firewall_core/src/net.rs#L86) true path | Never (MAC always non-zero) | **3 hits** (2% zero-MAC) | Robbie |
| [`Ipv4Repr::parse`](../crates/firewall_core/src/net.rs#L475) | **1,280 succeed** | 118 enter, **0 succeed** | Claude |
| [`TcpRepr::parse`](../crates/firewall_core/src/net.rs#L509) | **171 hits** | 0 hits | Claude |
| [`UdpRepr::parse`](../crates/firewall_core/src/net.rs#L538) | **1,109 hits** | 0 hits | Claude |
| [`firewall_core::lib.rs`](../crates/firewall_core/src/lib.rs) IPv4 dispatch | **All branches covered** | 0 hits (all fail IHL) | Claude |

### Strengths and Weaknesses

#### Claude Strategies

**Strengths:**
- Achieves 100% `timeTriggered` line coverage and 100% GUMBOX region coverage
- Exercises all 5 GUMBO guarantee families, including the IPv4 UDP paths (hlr_18, hlr_13)
- Dedicated per-category tests make it easy to isolate which category causes a failure
- Full MAVLink pipeline coverage (`udp_frame_from_raw_eth`, `udp_headers_from_raw_eth`, `udp_payload_from_raw_eth`)

**Weaknesses:**
- Only generates ARP Request (never Reply) — misses `ArpOp::Reply` branch in `net.rs`
- Only generates TCP and UDP protocols — `IpProtocol::try_from` misses 8 of 10 named variants
- All ARP frames pass validation — never exercises `Arp::parse` failure paths
- All destination MACs are stamped non-zero — never exercises `Address::is_empty` true path
- MAVLink strategy always stamps both ports correctly — cannot detect `&&` → `||` in `can_send_to_mavlink`
- All IPv4 frames use `length=60` — never exercises the MTU boundary check (`length` near 9000) in `Ipv4Repr::parse`
- Whitelisted UDP strategy always stamps `dst=68` — cannot distinguish "port 68 is allowed" from "all ports are allowed"
- 5 separate tests (500 total cases) vs Robbie's single 100-case test — higher test budget

Mutation testing confirmed the first four weaknesses above and identified the last three;
see [Generator Design: Near-Miss Fuzzing](testing-mutants-rxfirewall.md#generator-design-near-miss-fuzzing)
in the mutation testing report for analysis and a proposed fix (boundary-adjacent inputs).

#### Robbie Strategies

**Strengths:**
- Hierarchical `prop_flat_map` design mirrors real protocol layering, producing more realistic frames
- `ipv4_protocol_strategy` exercises all 10 `IpProtocol` variants plus the error branch
- `arp_op_strategy` generates both Request and Reply plus the error path
- `dst_mac_strategy` includes 2% all-zeros MAC, exercising the `Address::is_empty` true path
- ARP field weighting allows some invalid combinations, exercising parser failure paths
- Single 100-case test is more efficient (5× fewer test cases than Claude)

**Weaknesses:**
- Missing `v[0] = 0x45` (IPv4 version/IHL byte) causes **all** IPv4 frames to fail `Ipv4Repr::parse`, blocking the entire IPv4 path
- No MAVLink port pair generation — even with the IHL fix, P(MAVLink match) ≈ 3×10⁻⁹
- `hlr_18` and `hlr_13` GUMBO guarantees are never exercised
- `udp_frame_from_raw_eth` and related MAVLink output functions have 0 coverage
- 50/50 `Some`/`None` distribution means ~50% of inputs exercise only the no-input path

### Impact of the IHL Fix

Adding `v[0] = 0x45` to Robbie's `ipv4_strategy` would unblock the IPv4 parsing pipeline.
Experimental testing with this one-line fix showed:

| Metric | Robbie (original) | Robbie (with IHL fix) | Claude |
|--------|------------------|----------------------|--------|
| Total coverage | 406/557 (72.89%) | **471/557 (84.56%)** | 489/557 (87.79%) |
| App coverage | 67/137 (48.91%) | **94/137 (68.61%)** | 125/137 (91.24%) |
| GUMBOX line coverage | 207/227 (91.19%) | **213/227 (93.83%)** | 227/227 (100%) |
| [`firewall_core::lib.rs`](../crates/firewall_core/src/lib.rs) | 9/28 (32.14%) | **28/28 (100%)** | 20/28 (71.43%) |
| [`firewall_core::net.rs`](../crates/firewall_core/src/net.rs) | 123/165 (74.55%) | **136/165 (82.42%)** | 117/165 (70.91%) |
| `can_send_to_mavlink` IPv4 hits | 0 | **109** | 1,280 |
| `udp_port_allowed` hits | 0 | **20** | 1,109 |

With the IHL fix, Robbie's strategies would reach 84.56% overall — closing most of the gap
with Claude's 87.79%. The remaining difference would be the MAVLink path: Robbie's
`udp_strategy` does not generate the specific port pair (14550→14562) needed to exercise
`udp_frame_from_raw_eth` and the hlr_18 guarantee. Robbie's fixed strategies would also
**surpass** Claude on `firewall_core` coverage (164/193 = 84.97% vs 137/193 = 70.98%)
thanks to the broader protocol variant coverage.
