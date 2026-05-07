# RxFirewall Testing Report

## Table of Contents

- [Default PropTest Generators](#default-proptest-generators)
- [Claude Custom Strategies](#claude-custom-strategies)
- [Robbie VanVossen Custom Strategies](#robbie-vanvossen-custom-strategies)

---

## Default PropTest Generators

### Test Configuration

The RxFirewall was tested using the HAMR-generated GUMBOX PropTest infrastructure with
default random generators. The test configuration is:

| Parameter | Value |
|-----------|-------|
| Target test cases per entry point | 100 |
| Reject ratio | 5 (max 500 rejected inputs per entry point) |
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
| `prop_testInitializeCB_macro` | `initialize` | Invokes the initialize entry point and checks GUMBO postconditions |
| `prop_testComputeCB_macro` | `timeTriggered` | Generates random inputs for all four Rx ports, checks GUMBO postconditions |

### PropTest Generation Statistics

| Entry Point | Valid (Passed) | Rejected (Precondition) | Failed (Postcondition) | Total Generated |
|-------------|---------------|------------------------|----------------------|-----------------|
| `initialize` | 100 | 0 | 0 | 100 |
| `timeTriggered` | 100 | 0 | 0 | 100 |

All 200 generated test vectors passed. No inputs were rejected (there is no
precondition after the integration assumes were removed) and no postcondition
violations were detected.

### Test Results

| Entry Point | Result | Cases |
|-------------|--------|-------|
| `initialize` | **PASS** | 100/100 |
| `timeTriggered` | **PASS** | 100/100 |

### Coverage Analysis

| Entry Point | Lines Hit | Total Instrumented Lines | Coverage |
|-------------|-----------|-------------------------|----------|
| `initialize` (lines 25--30) | 5 | 5 | **100%** |
| `timeTriggered` (lines 150--203) | 20 | 44 | **45%** |

#### `initialize` Entry Point -- Full Coverage

The `initialize` entry point contains only a logging call and has no conditional
branches. All 5 instrumented lines are covered.

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
   values is 3/65536 (approximately 0.005%). Over 100 test cases, the expected
   number of frames with a valid EtherType is approximately 0.005 -- effectively
   zero.

3. **Protocol-specific sub-parsing** -- even if the EtherType is valid, ARP
   frames require valid `HardwareType` and `ArpOp` fields, and IPv4 frames
   require a recognized IP protocol number. These add further filtering.

The net effect is that random 1600-byte arrays almost never produce a parseable
Ethernet frame. When `get_frame_packet` returns `None`, the component correctly
does nothing (no output on any port), which satisfies the GUMBO postconditions.
But the routing logic inside the `Some(eth)` branch -- MAVLink detection,
VMM forwarding, and frame splitting -- is never exercised.

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

#### Recommendations

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
| Test file | `src/test/tests_claude_cust_strategies.rs` |
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

### Coverage Analysis

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
`cb_apis.rs`, line 67). This function (defined in `lib.rs`, line 36) creates a
fresh `seL4_RxFirewall_RxFirewall` instance, calls the application's `initialize`
entry point, and -- in test mode -- calls `initialize_test_globals()` to reset
the `Mutex`-guarded static port variables to `None`, ensuring each test case
starts from a clean state. Coverage data was collected from a clean run (prior
`.profraw` files deleted before instrumented execution), so no stale data from
previous test runs is included.

#### Per-Line Execution Counts (timeTriggered)

The following table shows execution counts for the inner routing branches of Rx0
(lines 159--164). The pattern is representative of all four channels.

| Line | Code | Hits |
|------|------|------|
| 159 | `if can_send_to_mavlink(&eth.eth_type) {` | 1437 |
| 160 | `let output = udp_frame_from_raw_eth(frame);` | 351 |
| 161 | `api.put_MavlinkOut0(output);` | 351 |
| 162 | `} else if can_send_to_vmm(&eth.eth_type) {` | 1086 |
| 163 | `api.put_VmmOut0(frame);` | 738 |
| 164 | `}` | 738 |

These counts confirm that all three routing outcomes are exercised: the MAVLink
forwarding path (lines 160--161), the VMM forwarding path (lines 163--164), and
the implicit drop path (when neither condition is true, control falls through
line 164 without sending).

#### GUMBO Guarantee Coverage

All five families of GUMBO compute guarantees are now exercised:

| Guarantee | Requirement | Exercised By |
|-----------|-------------|--------------|
| `hlr_05_rx{0..3}_can_send_arp_to_vmm` | HLR-05 | `arp`, `mixed` |
| `hlr_18_rx{0..3}_can_send_mavlink_udp` | HLR-18 | `mavlink_udp`, `mixed` |
| `hlr_13_rx{0..3}_can_send_ipv4_udp` | HLR-13 | `whitelisted_udp`, `mixed` |
| `hlr_15_rx{0..3}_disallow` | HLR-15 | `disallowed`, `mixed` |
| `hlr_17_rx{0..3}_no_input` | HLR-17 | `mixed` (via `None` and random-bytes inputs) |

### Comparison with Default Generators

| Metric | Default Generators | Custom Strategies |
|--------|-------------------|-------------------|
| Total test cases | 200 (2 tests) | 500 (5 tests) |
| `timeTriggered` coverage | 45% (20/44 lines) | **100%** (44/44 lines) |
| Routing branches exercised | None | All (MAVLink, VMM-ARP, VMM-UDP, drop) |
| GUMBO guarantee families tested | 1 of 5 (`hlr_17`) | **5 of 5** |
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
| Test file | `src/test/tests_robbiev_cust_stategies.rs` |
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

### Coverage Analysis

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
| 157 | `if let Some(frame) = api.get_EthernetFramesRxIn0() {` | 300 |
| 158 | `if let Some(eth) = get_frame_packet(&frame) {` | 168 |
| 159 | `if can_send_to_mavlink(&eth.eth_type) {` | 51 |
| 160 | `let output = udp_frame_from_raw_eth(frame);` | **0** |
| 161 | `api.put_MavlinkOut0(output);` | **0** |
| 162 | `} else if can_send_to_vmm(&eth.eth_type) {` | 51 |
| 163 | `api.put_VmmOut0(frame);` | 45 |
| 164 | `}` | 45 |
| 165 | `}` | 117 |
| 166 | `}` | 132 |

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

Robbie's `udp_strategy` biases the *destination* port toward 68 (the DHCP
whitelisted port) but leaves the *source* port entirely random. There is no
strategy that targets the MAVLink port pair specifically. The probability of
randomly generating both `src=14550` and `dst=14562` is:

- P(dst=14562) = 4/5 x 1/65536 = ~0.001% (from the random branch of
  `udp_port_strategy`)
- P(src=14550) = 1/65536 = ~0.002% (source port is fully random)
- P(both) ~ 3 x 10^-9

Over 100 test cases, the expected number of MAVLink-matching frames is
effectively zero.

#### Covered Lines -- What Is Exercised

Despite missing the MAVLink path, Robbie's generator achieves strong coverage of
the other routing branches:

- **Frame parsing** (line 158): 168 hits on Rx0, indicating ~56% of `Some` inputs
  produce a parseable frame -- a dramatic improvement over the default generator's
  ~0%.

- **ARP -> VMM forwarding** (lines 163--164): 45 hits, confirming valid ARP frames
  reach `put_VmmOut0`. This exercises `hlr_05`.

- **Whitelisted UDP -> VMM forwarding** (lines 163--164): the `udp_port_strategy`
  biases toward port 68, so some UDP frames hit the `can_send_to_vmm` path. This
  exercises `hlr_13`.

- **Disallowed frame drop** (implicit fall-through): frames that parse but match
  neither MAVLink nor VMM conditions produce no output. This exercises `hlr_15`.

- **No input / unparseable** (lines 165--166): `None` inputs and unparseable
  frames produce no output. This exercises `hlr_17`.

#### GUMBO Guarantee Coverage

| Guarantee | Requirement | Exercised? |
|-----------|-------------|------------|
| `hlr_05_rx{0..3}_can_send_arp_to_vmm` | HLR-05 | Yes |
| `hlr_18_rx{0..3}_can_send_mavlink_udp` | HLR-18 | **No** |
| `hlr_13_rx{0..3}_can_send_ipv4_udp` | HLR-13 | Yes |
| `hlr_15_rx{0..3}_disallow` | HLR-15 | Yes |
| `hlr_17_rx{0..3}_no_input` | HLR-17 | Yes |

4 of 5 GUMBO guarantee families are exercised. Only `hlr_18` (MAVLink UDP
forwarding) is not covered.

### Comparison with Default Generators

| Metric | Default Generators | Robbie V Custom Strategies |
|--------|-------------------|---------------------------|
| Total test cases | 200 (2 tests) | 100 (1 test) |
| `timeTriggered` coverage | 45% (20/44 lines) | **82%** (36/44 lines) |
| Routing branches exercised | None | ARP, whitelisted UDP, disallowed, drop |
| GUMBO guarantee families tested | 1 of 5 (`hlr_17`) | **4 of 5** |
| Postcondition failures | 0 | 0 |
