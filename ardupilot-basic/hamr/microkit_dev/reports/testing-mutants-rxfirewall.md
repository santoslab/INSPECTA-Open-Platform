# RxFirewall Mutation Testing Report

Mutation testing using [cargo-mutants](https://mutants.rs/) v27.0.0 on the
RxFirewall component. Unlike code coverage (which measures whether code is
reached), mutation testing measures whether tests actually *detect* behavioral
changes — injected bugs that survive all tests indicate gaps in test
effectiveness.

## Table of Contents

- [Setup](#setup)
- [Generator Design: Near-Miss Fuzzing](#generator-design-near-miss-fuzzing)
- [Mutation Results](#mutation-results)
  - [App Entry Point Methods](#app-entry-point-methods)
  - [App Exec Helper Functions](#app-exec-helper-functions)
  - [firewall_core Crate](#firewall_core-crate)

---

## Setup

### Tool

[cargo-mutants](https://mutants.rs/) v27.0.0. cargo-mutants injects mutations
(replacing function bodies with default values, flipping operators, deleting
match arms, etc.) into source code and re-runs the test suite for each mutation.
A mutation that causes a test failure is **caught**; one where all tests still
pass is **missed**, indicating a potential gap in test effectiveness.

### Workspace Configuration

HAMR generates each component as a standalone Rust crate with its own
`Cargo.toml` — there is no workspace by default. The RxFirewall crate
(`seL4_RxFirewall_RxFirewall`) depends on `firewall_core` via a path dependency:

```
crates/
  data/                              # shared data types
  GumboLib/                          # GUMBO contract library
  firewall_core/                     # Ethernet frame parser
  seL4_RxFirewall_RxFirewall/        # RxFirewall component (has tests)
```

cargo-mutants can mutate and test a single crate out of the box
(`cargo mutants` from within the crate directory). However, mutating
`firewall_core` is also valuable — the frame parser contains the routing
classification logic that the RxFirewall depends on — and `firewall_core` has
no tests of its own. To mutate `firewall_core` while running the RxFirewall
test suite as the oracle, cargo-mutants needs `--package` (what to mutate) and
`--test-package` (where the tests live), which requires both crates to be
members of the same Cargo workspace.

A workspace `Cargo.toml` was created at `hamr/microkit_dev/crates/Cargo.toml`:

```toml
[workspace]
members = [
    "data",
    "GumboLib",
    "firewall_core",
    "seL4_RxFirewall_RxFirewall",
]
resolver = "2"
```

A `rust-toolchain.toml` was also copied from the RxFirewall crate to the
workspace root — all crates require `nightly-2026-01-25` for Verus
compatibility, but Cargo resolves the toolchain file from the workspace root,
not from individual member crates.

### Mutation Scope

Mutations are restricted to **exec code** — application logic and runtime
parser functions. The following are excluded:

- **`data` crate** — auto-generated shared type definitions, not application logic
- **`GumboLib` crate** — GUMBO contract library, not application logic
- **GUMBOX contract methods** (`seL4_RxFirewall_RxFirewall_GUMBOX.rs`) — auto-generated test oracles; mutating these tests whether the tests catch oracle corruption, which is a separate concern
- **Bridge infrastructure** (`extern_c_api.rs`, `lib.rs`) — auto-generated HAMR plumbing
- **Verus spec code** inside `verus! { }` blocks — proof-only, not compiled into exec binaries

### Test Suite

Only the **Claude custom strategy tests** are used as the test oracle for all
mutation runs. These 5 tests (500 total cases) achieve 87.79% line coverage and
exercise all 5 GUMBO guarantee families. The test filter `claude` is passed to
`cargo test` via cargo-mutants' trailing arguments.

### Running

From the workspace root (`hamr/microkit_dev/crates/`):

```bash
# Mutate the RxFirewall app file, test with Claude tests
cargo mutants \
  -f seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs \
  --test-package seL4_RxFirewall_RxFirewall \
  -- claude

# Mutate firewall_core, test with Claude tests
cargo mutants \
  --package firewall-core \
  --test-package seL4_RxFirewall_RxFirewall \
  -- claude
```

---

## Generator Design: Near-Miss Fuzzing

A recurring pattern across both the app-level and firewall_core mutation results is
that "perfect" generators — strategies that always produce inputs squarely within a
target category — fail to catch mutations that widen the acceptance criteria. This
section describes the pattern and its fix. Several of these gaps were independently
identified as Claude strategy weaknesses in the
[Strengths and Weaknesses](testing-proptest-rxfirewall.md#strengths-and-weaknesses) analysis of
the test coverage report.

### The Problem

The Claude custom strategies stamp fixed, correct values for the fields that determine
classification. For example, the MAVLink strategy always sets `src_port=14550` and
`dst_port=14562`. This exercises the *forwarding* path (frames that should be accepted
*are* accepted), but it cannot distinguish between the correct classifier and an
overly-permissive one. The mutation `replace && with ||` in `can_send_to_mavlink` makes
the function accept frames where *either* port matches instead of *both* — but since
every generated test vector has both ports correct, `&&` and `||` produce identical
results for 100% of inputs.

**Increasing `NUM_CASES` does not help.** The port values are deterministic constants
in the strategy, not sampled from a distribution. Whether 100 or 10,000 test cases are
generated, every one has `src=14550, dst=14562`. The distinguishing inputs (e.g.,
`src=14550, dst=9999`) are simply not in the sampled space. The *mixed* strategy
includes fully random frames (1-in-10 weight), but a random 1518-byte frame that
happens to have one correct MAVLink port with valid IPv4+UDP headers is astronomically
unlikely.

### The Fix: Near-Miss Inputs

For every classification boundary in the routing logic, generators should produce
inputs on **both sides** of it: inputs that satisfy the criterion (happy path) and
inputs that are *just outside* it (near miss). A generator that is ~90% perfect and
~10% near-miss exercises both the acceptance and rejection boundaries.

For `can_send_to_mavlink`, this means the strategy should mostly stamp both ports
correctly but occasionally fuzz one port while keeping the other correct (~5% each
for src-right/dst-wrong and src-wrong/dst-right). The near-miss frames would be
classified as non-MAVLink by the real `&&` logic but accepted by the `||` mutation,
causing the GUMBOX postcondition to catch the misrouting.

### Affected Mutations

This pattern accounts for missed mutations across multiple functions and both crates.
Mutations marked *(near-miss)* in the per-function results throughout this report
would be caught by generators that include boundary-adjacent inputs:

**Exec helper functions ([`seL4_RxFirewall_RxFirewall_app.rs`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs)):**

| Mutation | Near-miss input needed |
|----------|-----------------------|
| `replace && with \|\|` in `can_send_to_mavlink` | Frame with only one MAVLink port correct (e.g., `src=14550, dst=9999`) |

**firewall_core ([`net.rs`](../crates/firewall_core/src/net.rs)):**

| Mutation | Near-miss input needed |
|----------|-----------------------|
| `Address::is_empty` → `false` | Frame with all-zero destination MAC |
| `Address::is_empty`: `<` → `<=` | Same — all-zero MAC exercises `is_empty`'s `true` return |
| `Address::is_empty`: `+=` → `-=` | Same — all-zero MAC where loop must check all 6 bytes |
| Delete `ArpOp::Reply` match arm | ARP Reply frame (`ArpOp=2`) alongside current Request-only strategy |
| `Arp::allowed_ptype` → `true` | Malformed ARP with `ptype=Arp` (currently always `ptype=IPv4`) |
| `Ipv4Repr::parse` MTU: `>` → `==` | Frame with `length` near 9000 (currently always 60) |
| `Ipv4Repr::parse` MTU: `>` → `>=` | Frame with `length=9000` exactly |

The whitelisted UDP strategy has the same limitation — it always stamps `dst=68`, so
it cannot distinguish "port 68 is allowed" from "all ports are allowed." Occasional
near-miss ports (e.g., `dst=69`) would strengthen that boundary test as well.

---

## Mutation Results

Results are organized by source location: the RxFirewall app file (entry points and
helper functions) and the firewall_core parser crate.

### App Entry Point Methods

#### Scope

Mutations targeting the two component entry point methods in
[`seL4_RxFirewall_RxFirewall_app.rs`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs):
- [`initialize`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L29) (line 29) — called once during system initialization
- [`timeTriggered`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L157) (lines 157–202) — the periodic compute entry point

#### Test Suite

Only the Claude custom strategy tests were used as the test oracle:

| Test | Strategy |
|------|----------|
| `prop_claude_cust_strategies_mixed` | Weighted mix of all frame categories |
| `prop_claude_cust_strategies_arp` | Valid ARP frames (HLR-05) |
| `prop_claude_cust_strategies_mavlink_udp` | MAVLink UDP frames (HLR-18) |
| `prop_claude_cust_strategies_whitelisted_udp` | Whitelisted UDP frames (HLR-13) |
| `prop_claude_cust_strategies_disallowed` | Disallowed frames (HLR-15) |

#### Mutation Operators

cargo-mutants generates mutations based on the function's return type and body
structure. Both `initialize` and `timeTriggered` return `()` (no return value),
so the only mutation available is **replace entire body with `()`** — effectively
deleting all statements in the function. cargo-mutants does not generate
statement-level deletions (e.g., removing a single `api.put_MavlinkOut0(output)`
call) within void functions.

#### Results

| Mutation | Line | Result |
|----------|------|--------|
| `replace initialize with ()` | 29 | **MISSED** |
| `replace timeTriggered with ()` | 157 | **CAUGHT** |

**Mutation score: 1/2 (50%)**

#### Analysis

##### `replace timeTriggered with ()` — CAUGHT

Replacing the entire `timeTriggered` body with `()` removes all routing logic:
no input ports are read, no frames are parsed, and no output ports are written.
Four of the five Claude tests detected this:

| Test | Result | Why |
|------|--------|-----|
| `arp` | **FAILED** | ARP inputs produced no `VmmOut` output — hlr_05 postcondition violated |
| `mavlink_udp` | **FAILED** | MAVLink inputs produced no `MavlinkOut` output — hlr_18 postcondition violated |
| `whitelisted_udp` | **FAILED** | Whitelisted UDP inputs produced no `VmmOut` output — hlr_13 postcondition violated |
| `mixed` | **FAILED** | Mix of frame types; same violations as above |
| `disallowed` | PASSED | Disallowed inputs expect *no* output — an empty body produces no output, which satisfies hlr_15 |

The `disallowed` test passing is correct behavior, not a gap: hlr_15 requires that
disallowed frames produce no output on any port, and an empty `timeTriggered` body
trivially satisfies this. A more targeted mutation (e.g., removing only the
`can_send_to_mavlink` check so disallowed frames *are* forwarded) would test hlr_15
more precisely, but cargo-mutants does not generate such statement-level mutations for
void functions.

##### `replace initialize with ()` — MISSED

The unmutated `initialize` body is:

```rust
pub fn initialize<API: seL4_RxFirewall_RxFirewall_Put_Api>(
    &mut self,
    api: &mut seL4_RxFirewall_RxFirewall_Application_Api<API>)
{
    log_info("initialize entrypoint invoked");
}
```

The only statement is a log call. Replacing it with `()` has no observable effect
on component state or port values — there are no DataPorts to initialize (all ports
are EventDataPorts, which do not require initialization per the HAMR execution
semantics), and there are no GUMBO state variables.

This is an **expected miss**, not a test gap. The mutation is semantically equivalent
to the original: both produce the same externally observable behavior (no port writes,
no state changes). No test — regardless of strategy — can distinguish between
"logged a message" and "did nothing" because the test infrastructure has no access
to log output.

#### Observations

1. **cargo-mutants limitations for void functions.** Both entry point methods return
   `()`, so cargo-mutants can only generate one mutation per method: replace the
   entire body. It cannot generate finer-grained mutations such as:
   - Deleting a single `api.put_MavlinkOut0(output)` call
   - Removing one `if let` branch (e.g., the Rx2 channel)
   - Swapping the order of `can_send_to_mavlink` and `can_send_to_vmm` checks

   These finer-grained mutations are available indirectly through the **exec helper
   functions** that `timeTriggered` calls (`get_frame_packet`, `can_send_to_mavlink`,
   `can_send_to_vmm`, `port_allowed`, etc.), which have non-void return types and
   operators that cargo-mutants can mutate. A mutation like
   `replace can_send_to_mavlink -> bool with true` is equivalent to "route every
   parsed frame to MavlinkOut", which effectively tests whether the per-channel
   routing logic is validated.

2. **Test specificity.** The `disallowed` test correctly passes when `timeTriggered`
   is emptied because hlr_15's postcondition (no output on any port) is vacuously
   satisfied by an empty body. This highlights that negative guarantees (frame
   *must not* produce output) are inherently weaker at detecting "do nothing"
   mutations than positive guarantees (frame *must* produce specific output).

3. **Initialize is a no-op.** The RxFirewall's `initialize` contains only a log
   statement. This is architecturally expected — the component has no DataPorts
   requiring initialization and no GUMBO state variables. The missed mutation does
   not indicate a testing weakness.

---

### App Exec Helper Functions

#### Scope

Mutations targeting the exec helper functions in
[`seL4_RxFirewall_RxFirewall_app.rs`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs)
that are called by `timeTriggered`. Entry point methods (`initialize`, `timeTriggered`,
`notify`) and logging functions (`log_info`, `log_warn_channel`) are excluded.

| Function | Line | Purpose |
|----------|------|---------|
| [`get_frame_packet`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L305) | 305 | Parses raw Ethernet frame via `firewall_core::EthFrame::parse` |
| [`can_send_to_mavlink`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L321) | 321 | Checks if packet is MAVLink UDP (src=14550, dst=14562) |
| [`udp_frame_from_raw_eth`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L339) | 339 | Splits raw frame into header + payload for MavlinkOut |
| [`udp_headers_from_raw_eth`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L352) | 352 | Copies first N bytes as UDP headers |
| [`udp_payload_from_raw_eth`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L376) | 376 | Copies remaining bytes as UDP payload |
| [`can_send_to_vmm`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L404) | 404 | Checks if packet is ARP or whitelisted UDP |
| [`udp_port_allowed`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L433) | 433 | Checks UDP dst port against allowlist |
| [`tcp_port_allowed`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L443) | 443 | Checks TCP dst port against allowlist |
| [`port_allowed`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L454) | 454 | Generic linear scan of port allowlist |
| [`info_protocol`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L474) | 474 | Logs dropped packet's IP protocol |

#### Results Summary

| Outcome | Count |
|---------|-------|
| Caught | 28 |
| Missed | 5 |
| Timeout | 3 |
| Unviable | 5 |
| **Total** | **41** |

**Mutation score: 28/36 caught (77.8%)** — excluding 5 unviable mutations that
did not compile.

#### Full Results

##### [`get_frame_packet`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L305) (line 305)

| Mutation | Result |
|----------|--------|
| Replace `-> Option<EthFrame>` with `None` | **Caught** |
| Replace `-> Option<EthFrame>` with `Some(Default::default())` | Unviable |

Returning `None` means no frame ever parses, so no output is produced on any port —
caught by the ARP, MAVLink, and whitelisted UDP tests. The `Some(Default::default())`
mutation is **unviable** because `EthFrame` does not implement `Default`.

##### [`can_send_to_mavlink`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L321) (lines 321–327)

```rust
fn can_send_to_mavlink(packet: &firewall_core::PacketType) -> bool {
    if let firewall_core::PacketType::Ipv4(ip) = packet {
        if let firewall_core::Ipv4ProtoPacket::Udp(udp) = &ip.protocol {
            return udp.src_port == MAV_UDP_SRC_PORT && udp.dst_port == MAV_UDP_DST_PORT;
        }
    }
    false
}
```

| Mutation | Result |
|----------|--------|
| Replace `-> bool` with `true` | **Caught** |
| Replace `-> bool` with `false` | **Caught** |
| Replace `==` with `!=` (src_port check, col 33) | **Caught** |
| Replace `==` with `!=` (dst_port check, col 69) | **Caught** |
| Replace `&&` with `\|\|` (col 53) | **MISSED** |

The `&&` → `||` miss *(near-miss)*: with `||`, `can_send_to_mavlink` returns true when
*either* port matches instead of *both*. The Claude MAVLink strategy always stamps both
`src_port=14550` and `dst_port=14562`, so the `||` mutation produces the same result
as `&&` for every generated test vector. A frame with only one port correct (e.g.,
`src=14550, dst=9999`) would catch this. See
[Generator Design: Near-Miss Fuzzing](#generator-design-near-miss-fuzzing) for why
increasing test counts does not help and how generators should be redesigned.

##### [`udp_frame_from_raw_eth`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L339) (line 339)

| Mutation | Result |
|----------|--------|
| Replace `-> UdpFrame_Impl` with `Default::default()` | **Caught** |

Returning a zeroed-out frame instead of the real header/payload split is detected
by the GUMBO postcondition checks (hlr_18 requires the output to match the input
frame's header and payload).

##### [`udp_headers_from_raw_eth`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L352) (lines 352–366)

```rust
fn udp_headers_from_raw_eth(value: SW::RawEthernetMessage) -> SW::EthIpUdpHeaders {
    let mut headers = [0u8; SW::SW_EthIpUdpHeaders_DIM_0];
    let mut i = 0;
    while i < SW::SW_EthIpUdpHeaders_DIM_0 {
        headers.set(i, value[i]);
        i += 1;
    }
    headers
}
```

| Mutation | Result |
|----------|--------|
| Replace `-> EthIpUdpHeaders` with `Default::default()` | Unviable |
| Replace `<` with `==` in loop (line 361) | **Caught** |
| Replace `<` with `>` in loop (line 361) | **Caught** |
| Replace `<` with `<=` in loop (line 361) | **Caught** |
| Replace `+=` with `-=` in loop (line 363) | **Caught** |
| Replace `+=` with `*=` in loop (line 363) | Timeout |

The loop condition and increment mutations are all caught — `<` → `==` makes the
loop execute only once (copying 1 byte), `<` → `>` skips the loop entirely,
`+=` → `-=` makes `i` go negative (infinite loop, but caught by panic). The
`+=` → `*=` mutation creates an infinite loop (`0 *= 1` stays 0), which cargo-mutants
detects as a timeout. The `Default::default()` return is unviable because
`EthIpUdpHeaders` (a fixed-size byte array type alias) does not implement `Default`
in this context.

##### [`udp_payload_from_raw_eth`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L376) (lines 376–390)

```rust
fn udp_payload_from_raw_eth(value: SW::RawEthernetMessage) -> SW::UdpPayload {
    let mut payload = [0u8; SW::SW_RawEthernetMessage_DIM_0 - SW::SW_EthIpUdpHeaders_DIM_0];
    let mut i = 0;
    while i < SW::SW_UdpPayload_DIM_0 {
        payload.set(i, value[i + SW::SW_EthIpUdpHeaders_DIM_0]);
        i += 1;
    }
    payload
}
```

| Mutation | Result |
|----------|--------|
| Replace `-> UdpPayload` with `Default::default()` | Unviable |
| Replace `-` with `+` in array size (line 376) | Unviable |
| Replace `-` with `/` in array size (line 376) | Unviable |
| Replace `<` with `==` in loop (line 385) | **Caught** |
| Replace `<` with `>` in loop (line 385) | **Caught** |
| Replace `<` with `<=` in loop (line 385) | **Caught** |
| Replace `+` with `-` in index (line 386) | **Caught** |
| Replace `+` with `*` in index (line 386) | **Caught** |
| Replace `+=` with `-=` in loop (line 387) | **Caught** |
| Replace `+=` with `*=` in loop (line 387) | Timeout |

Same pattern as `udp_headers_from_raw_eth`. The arithmetic mutations in the array
size expression (line 376) are unviable because they produce compile-time constant
expressions that overflow or don't match the expected array size type. The `+=` → `*=`
timeout is again an infinite loop from `0 *= 1`.

##### [`can_send_to_vmm`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L404) (lines 404–424)

```rust
fn can_send_to_vmm(packet: &firewall_core::PacketType) -> bool {
    match packet {
        firewall_core::PacketType::Arp(_) => true,
        firewall_core::PacketType::Ipv4(ip) => match &ip.protocol {
            firewall_core::Ipv4ProtoPacket::Udp(udp) => {
                let allowed = udp_port_allowed(udp.dst_port);
                if !allowed {
                    log_info("UDP packet filtered out");
                }
                allowed
            }
            _ => {
                info_protocol(ip.header.protocol);
                false
            }
        },
        firewall_core::PacketType::Ipv6 => {
            log_info("Not an IPv4 or Arp packet. Throw it away.");
            false
        }
    }
}
```

| Mutation | Result |
|----------|--------|
| Replace `-> bool` with `true` | **Caught** |
| Replace `-> bool` with `false` | **Caught** |
| Delete match arm `Udp(udp)` | **Caught** |
| Delete `!` in `if !allowed` (line 409) | **MISSED** |

The `delete !` miss: changing `if !allowed` to `if allowed` only affects *which
log message is printed* — it does not change the return value (`allowed` is
returned regardless of the `if` branch). The function's externally observable
behavior is identical. This is an expected miss — the mutation is semantically
equivalent because the `if` block only contains a log call.

##### [`udp_port_allowed`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L433) (line 433)

| Mutation | Result |
|----------|--------|
| Replace `-> bool` with `true` | **Caught** |
| Replace `-> bool` with `false` | **Caught** |

Both caught. Returning `true` always would allow non-whitelisted UDP through (violates
hlr_15). Returning `false` always would block whitelisted UDP (violates hlr_13).

##### [`tcp_port_allowed`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L443) (line 443)

| Mutation | Result |
|----------|--------|
| Replace `-> bool` with `true` | **MISSED** |
| Replace `-> bool` with `false` | **MISSED** |

Both missed. `tcp_port_allowed` is **dead code** in the current implementation.
`can_send_to_vmm`'s inner match on `ip.protocol` only has a `Udp(udp)` arm — TCP
packets fall through to the `_ =>` catch-all and are dropped before `tcp_port_allowed`
is ever called. No test can detect mutations in a function that is never called.

##### [`port_allowed`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L454) (lines 454–469)

```rust
fn port_allowed(allowed_ports: &[u16], port: u16) -> bool {
    let mut i: usize = 0;
    while i < allowed_ports.len() {
        if allowed_ports[i] == port {
            return true;
        }
        i += 1;
    }
    false
}
```

| Mutation | Result |
|----------|--------|
| Replace `-> bool` with `true` | **Caught** |
| Replace `-> bool` with `false` | **Caught** |
| Replace `<` with `==` in loop (line 462) | **Caught** |
| Replace `<` with `>` in loop (line 462) | **Caught** |
| Replace `<` with `<=` in loop (line 462) | **Caught** |
| Replace `==` with `!=` (line 463) | **Caught** |
| Replace `+=` with `-=` (line 466) | **Caught** |
| Replace `+=` with `*=` (line 466) | Timeout |

All non-timeout mutations caught. The `+=` → `*=` timeout is the same infinite
loop pattern (`0 *= 1` stays 0). Note: although `port_allowed` is called by
both `udp_port_allowed` and `tcp_port_allowed`, only the UDP call path is
exercised at runtime — but the UDP path alone is sufficient to detect all
mutations.

##### [`info_protocol`](../crates/seL4_RxFirewall_RxFirewall/src/component/seL4_RxFirewall_RxFirewall_app.rs#L474) (line 474)

| Mutation | Result |
|----------|--------|
| Replace body with `()` | **MISSED** |

Expected miss. `info_protocol` is a pure logging function — it writes to the log
and has no return value or side effects visible to the test harness.

#### Analysis

##### Missed Mutations

| # | Mutation | Root Cause |
|---|----------|-----------|
| 1 | `replace && with \|\| in can_send_to_mavlink` | *(near-miss)* — Claude's MAVLink strategy always stamps both ports correctly; see [Generator Design: Near-Miss Fuzzing](#generator-design-near-miss-fuzzing) |
| 2 | `delete ! in can_send_to_vmm` | The `!` only guards a log call — removing it doesn't change the return value |
| 3 | `replace tcp_port_allowed -> bool with true` | Dead code: `tcp_port_allowed` is never called at runtime |
| 4 | `replace tcp_port_allowed -> bool with false` | Dead code: same as above |
| 5 | `replace info_protocol with ()` | Pure logging function with no observable side effects |

Of the 5 missed mutations:
- **1 is a [near-miss fuzzing](#generator-design-near-miss-fuzzing) gap** (#1) — the
  `&&` → `||` miss would be caught by a generator that occasionally fuzzes one MAVLink
  port while keeping the other correct
- **2 are dead code** (#3, #4) — `tcp_port_allowed` is unreachable in the current
  implementation because `can_send_to_vmm` lacks a `Tcp` match arm
- **2 are semantically equivalent** (#2, #5) — the mutations change logging
  behavior only, which is invisible to the test harness

##### Timeout Mutations

| # | Mutation | Mechanism |
|---|----------|-----------|
| 1 | `replace += with *= in udp_headers_from_raw_eth` | `i *= 1` when `i=0` → infinite loop |
| 2 | `replace += with *= in udp_payload_from_raw_eth` | Same pattern |
| 3 | `replace += with *= in port_allowed` | Same pattern |

All 3 timeouts are `+=` → `*=` in loop increment expressions where `i` starts at 0.
`0 * 1 = 0`, so the loop counter never advances. cargo-mutants classifies these as
timeouts (the test hangs), which effectively means the mutation is detected — the
program's behavior is observably changed (it never terminates).

##### Unviable Mutations

| # | Mutation | Reason |
|---|----------|--------|
| 1 | `get_frame_packet` → `Some(Default::default())` | `EthFrame` does not implement `Default` |
| 2 | `udp_headers_from_raw_eth` → `Default::default()` | Array type alias lacks `Default` in this context |
| 3 | `udp_payload_from_raw_eth` → `Default::default()` | Same as above |
| 4 | Replace `-` with `+` in array size (line 376) | Compile-time constant overflow |
| 5 | Replace `-` with `/` in array size (line 376) | Resulting size doesn't match expected type |

All 5 unviable mutations fail to compile. These are not test gaps — they represent
mutations that cargo-mutants generated but that are invalid Rust.

#### Observations

1. **High kill rate for routing logic.** All mutations to `can_send_to_mavlink` (body
   replacements, `==` → `!=`), `can_send_to_vmm` (body replacements, match arm
   deletion), `udp_port_allowed`, and `port_allowed` are caught. The GUMBO contracts
   are effective at detecting behavioral changes in the frame classification and
   forwarding logic.

2. **One actionable test gap ([near-miss](#generator-design-near-miss-fuzzing)).** The
   `&&` → `||` miss in `can_send_to_mavlink` is the only missed mutation that represents
   a real testing weakness. A near-miss frame with `src_port=14550, dst_port=9999` would
   distinguish `&&` from `||` — the original rejects it, but the `||` mutation accepts it.

3. **Dead code confirmed.** `tcp_port_allowed` is unreachable because `can_send_to_vmm`
   has no `Tcp` match arm — TCP packets hit the `_ =>` catch-all. This was already
   identified in the coverage analysis; mutation testing independently confirms it.
   If TCP allowlist filtering is intended behavior, a `Tcp(tcp)` arm should be added
   to `can_send_to_vmm`.

4. **Logging mutations are inherently unmeasurable.** `info_protocol` and the
   `if !allowed` log guard are both invisible to the test harness. This is expected —
   mutation testing tools cannot distinguish "changed log output" from "changed nothing"
   unless the test suite captures and asserts on log output.

5. **Timeouts count as caught in practice.** The 3 `+=` → `*=` infinite loops
   demonstrate that the program's behavior is detectably changed — it hangs instead
   of completing. Including timeouts, the effective kill rate is 31/36 (86.1%).

---

### firewall_core Crate

#### Scope

Mutations targeting the [`firewall_core`](../crates/firewall_core/) crate — the
Ethernet frame parser that the RxFirewall depends on for frame classification. This crate
has its own unit tests in [`lib.rs`](../crates/firewall_core/src/lib.rs) and
[`net.rs`](../crates/firewall_core/src/net.rs), but here we test exclusively
with the RxFirewall Claude custom strategy tests via `--test-package`.

The crate contains two source files:

- [`lib.rs`](../crates/firewall_core/src/lib.rs) — type definitions (`EthFrame`,
  `PacketType`, `Ipv4Packet`, etc.) inside a `verus!` block, plus the
  [`EthFrame::parse`](../crates/firewall_core/src/lib.rs#L78) method outside it,
  and Verus spec functions
- [`net.rs`](../crates/firewall_core/src/net.rs) — network primitive types
  (`Address`, `Ipv4Address`, `EthernetRepr`, `Arp`, `Ipv4Repr`, `UdpRepr`, `TcpRepr`,
  `EtherType`, `IpProtocol`, etc.) with parse methods and `TryFrom`/`From` impls

Verus spec functions (inside `verus!` blocks) are proof-only and excluded from mutation.
cargo-mutants only mutates exec code.

#### Running

```bash
cargo mutants \
  --package firewall-core \
  --test-package seL4_RxFirewall_RxFirewall \
  -- claude
```

#### Results Summary

| Outcome | Count |
|---------|-------|
| Caught | 32 |
| Missed | 23 |
| Timeout | 3 |
| Unviable | 15 |
| **Total** | **73** |

**Mutation score: 32/58 caught (55.2%)** — excluding 15 unviable mutations.

#### Full Results

##### [`EthFrame::parse`](../crates/firewall_core/src/lib.rs#L78) (lib.rs line 78)

| Mutation | Result |
|----------|--------|
| Replace `-> Option<EthFrame>` with `None` | **Caught** |
| Replace `-> Option<EthFrame>` with `Some(Default::default())` | Unviable |

Returning `None` means no frame ever parses — caught by ARP, MAVLink, and whitelisted
UDP tests (same as the app-level `get_frame_packet` mutation). `Some(Default::default())`
is unviable because `EthFrame` does not implement `Default`.

##### [`Ipv4Address::from_bytes`](../crates/firewall_core/src/net.rs#L23) (net.rs lines 23–39)

| Mutation | Result |
|----------|--------|
| Replace `-> Ipv4Address` with `Default::default()` | Unviable |
| Replace `<` with `==` in loop (line 34) | **MISSED** |
| Replace `<` with `>` in loop (line 34) | **MISSED** |
| Replace `<` with `<=` in loop (line 34) | **Caught** |
| Replace `+=` with `-=` in loop (line 36) | **Caught** |
| Replace `+=` with `*=` in loop (line 36) | Timeout |

The `<` → `==` and `<` → `>` misses: these break the byte copy loop (copying only 1 byte
or 0 bytes), corrupting the parsed IPv4 source/destination addresses. However, the
RxFirewall never inspects parsed IPv4 addresses — routing decisions are based on protocol
type and port numbers, not IP addresses. The corrupted `Ipv4Address` fields are constructed
but never read by any test-observable code path.

**Specification gap: no IP address filtering.** This mutation survival reveals more than a
testing gap — it surfaces a *specification* deficiency. The RxFirewall accepts any source
or destination IP address as long as the protocol/port checks pass. For a firewall
component, this is at minimum a gap in defense-in-depth: a spoofed source IP or an
unexpected destination IP would pass through unchecked.

Whether this constitutes a bug depends on the system requirements. If the RxFirewall is
intended to filter only on protocol and port (as the current GUMBO contracts specify),
then the behavior is correct-by-spec. But if IP address validation is a desired security
property, the current specification is incomplete.

**Would Verus catch this?** No — not on its own. Verus verifies that code conforms to
its *contracts*. If the GUMBO contracts don't specify IP address constraints (and
currently they don't — the guarantees cover protocol type, port matching, and frame
forwarding), then Verus has nothing to flag. The code faithfully implements the spec;
the spec itself is what's missing the constraint. To close this gap, a GUMBO integration
constraint would need to be added to the model — e.g., a guarantee that output frames
have destination IPs within an allowed range. With such a constraint in place, HAMR would
generate Verus contracts on the port API, and Verus would then verify that the Rust
implementation enforces it. At that point, a broken `Ipv4Address::from_bytes` copy loop
would fail verification because the corrupted address wouldn't satisfy the constraint.

This is a case where mutation testing surfaces a **specification gap** rather than a
testing gap or implementation bug — the code and tests are consistent with each other,
but neither captures the full security intent.

##### [`Address::from_bytes`](../crates/firewall_core/src/net.rs#L62) (net.rs lines 62–78)

| Mutation | Result |
|----------|--------|
| Replace `-> Address` with `Default::default()` | Unviable |
| Replace `<` with `==` in loop (line 73) | **Caught** |
| Replace `<` with `>` in loop (line 73) | **Caught** |
| Replace `<` with `<=` in loop (line 73) | **Caught** |
| Replace `+=` with `-=` in loop (line 75) | **Caught** |
| Replace `+=` with `*=` in loop (line 75) | Timeout |

All non-timeout mutations caught. Unlike `Ipv4Address`, the MAC `Address` *is* checked —
`EthernetRepr::parse` calls `dst_addr.is_empty()` to reject frames with an all-zero
destination MAC. Breaking the copy loop corrupts the MAC, which can flip `is_empty()`'s
result and cause frames that should parse to be rejected (or vice versa).

##### [`Address::is_empty`](../crates/firewall_core/src/net.rs#L86) (net.rs lines 86–102)

| Mutation | Result |
|----------|--------|
| Replace `-> bool` with `true` | **Caught** |
| Replace `-> bool` with `false` | **MISSED** |
| Replace `<` with `==` in loop (line 95) | **Caught** |
| Replace `<` with `>` in loop (line 95) | **Caught** |
| Replace `<` with `<=` in loop (line 95) | **MISSED** |
| Replace `!=` with `==` (line 96) | **Caught** |
| Replace `+=` with `-=` (line 99) | **MISSED** |
| Replace `+=` with `*=` (line 99) | Timeout |

The `-> false` miss *(near-miss)*: `is_empty` is called on the destination MAC to reject
all-zero addresses. Returning `false` always means no frame is ever rejected for having
an empty MAC. The Claude strategies always stamp non-zero destination MACs, so all frames
pass the `is_empty` check regardless of the mutation. See
[Generator Design: Near-Miss Fuzzing](#generator-design-near-miss-fuzzing) — a test with
an all-zero destination MAC would catch this and the two loop mutations below.

The `< → <=` miss *(near-miss)*: the loop becomes `while i <= 6`, which *would*
access `self.0[6]` (out of bounds on a `[u8; 6]`) and panic — but the out-of-bounds
iteration is never reached. The Claude strategies always stamp non-zero destination
MACs (e.g., `0xff, 0xff, ...` for ARP, `0x02, 0x00, ...` for IPv4), so on the very
first iteration (`i=0`), `self.0[0] != 0` is true and the function returns `false`
immediately — the loop never advances to `i=6`. The only way to trigger the panic is
with an all-zero MAC, where the loop must check all 6 bytes, finds them all zero, and
then enters the `i=6` iteration. An all-zero MAC test would catch this mutation (via
panic), and simultaneously catch the `→ false` and `+= → -=` mutations above.

The `+= → -=` miss (line 99): the loop counter goes 0, -1 (wraps to `usize::MAX`), so
the `while i < len` condition fails on the second iteration. The function checks only
byte 0 — if it's non-zero (as in all Claude strategies), it returns `false` immediately
before the loop counter matters.

##### [`u16_from_be_bytes`](../crates/firewall_core/src/net.rs#L116) (net.rs line 116–118)

| Mutation | Result |
|----------|--------|
| Replace `-> u16` with `0` | **Caught** |
| Replace `-> u16` with `1` | **Caught** |
| Replace `+` with `-` (line 117) | **Caught** |
| Replace `+` with `*` (line 117) | **Caught** |
| Replace `*` with `+` (line 117) | **Caught** |
| Replace `*` with `/` (line 117) | **Caught** |

All caught. This function is used pervasively — for parsing EtherType, port numbers,
IPv4 length, ARP fields. Any arithmetic corruption cascades into misclassified frames.

##### [`EtherType::from_bytes`](../crates/firewall_core/src/net.rs#L144) (net.rs line 144)

| Mutation | Result |
|----------|--------|
| Replace `-> Option<EtherType>` with `None` | **Caught** |
| Replace `-> Option<EtherType>` with `Some(Default::default())` | Unviable |

##### [`EtherType::try_from`](../crates/firewall_core/src/net.rs#L154) (net.rs lines 154–162)

| Mutation | Result |
|----------|--------|
| Delete match arm `0x0800` (Ipv4) | **Caught** |
| Delete match arm `0x0806` (Arp) | **Caught** |
| Delete match arm `0x86DD` (Ipv6) | **MISSED** |

The IPv6 miss: deleting the `0x86DD → Ipv6` arm means IPv6 frames fail to parse
(`EtherType::try_from` returns `Err`), so `EthFrame::parse` returns `None`. IPv6
frames are disallowed by the RxFirewall — they should produce no output. Whether the
frame is rejected because IPv6 parsing fails or because `can_send_to_vmm` returns
`false` for `PacketType::Ipv6`, the externally observable behavior is the same: no
output on any port.

##### [`EtherType: From<EtherType> for u16`](../crates/firewall_core/src/net.rs#L166) (net.rs lines 166–174)

| Mutation | Result |
|----------|--------|
| Replace `-> Self` with `Default::default()` | **MISSED** |

The `From<EtherType> for u16` impl converts an `EtherType` enum variant back to its
wire format integer. The RxFirewall only *parses* frames (u16 → EtherType via `TryFrom`),
never serializes them (EtherType → u16 via `From`). This impl is dead code from the
RxFirewall's perspective.

##### [`EthernetRepr::parse`](../crates/firewall_core/src/net.rs#L202) (net.rs line 202)

| Mutation | Result |
|----------|--------|
| Replace `-> Option<EthernetRepr>` with `None` | **Caught** |
| Replace `-> Option<EthernetRepr>` with `Some(Default::default())` | Unviable |

##### [`ArpOp::from_bytes`](../crates/firewall_core/src/net.rs#L240) (net.rs line 240)

| Mutation | Result |
|----------|--------|
| Replace `-> Option<ArpOp>` with `None` | **Caught** |
| Replace `-> Option<ArpOp>` with `Some(Default::default())` | Unviable |

##### [`ArpOp::try_from`](../crates/firewall_core/src/net.rs#L250) (net.rs lines 250–257)

| Mutation | Result |
|----------|--------|
| Delete match arm `1` (Request) | **Caught** |
| Delete match arm `2` (Reply) | **MISSED** |

The Reply miss *(near-miss)*: Claude's ARP strategy always stamps `ArpOp=Request`
(bytes `0x00, 0x01` at frame offset 20–21). Deleting the Reply arm means ARP Reply
frames fail to parse, but no Claude test generates ARP Reply frames. See
[Generator Design: Near-Miss Fuzzing](#generator-design-near-miss-fuzzing) — the ARP
strategy should include Reply frames alongside Request.

##### [`ArpOp: From<ArpOp> for u16`](../crates/firewall_core/src/net.rs#L261) (net.rs line 261)

| Mutation | Result |
|----------|--------|
| Replace `-> Self` with `Default::default()` | **MISSED** |

Dead code — the RxFirewall never serializes `ArpOp` back to `u16`.

##### [`HardwareType::from_bytes`](../crates/firewall_core/src/net.rs#L290) (net.rs line 290)

| Mutation | Result |
|----------|--------|
| Replace `-> Option<HardwareType>` with `None` | **Caught** |
| Replace `-> Option<HardwareType>` with `Some(Default::default())` | Unviable |

##### [`HardwareType::try_from`](../crates/firewall_core/src/net.rs#L300) (net.rs lines 300–306)

| Mutation | Result |
|----------|--------|
| Delete match arm `1` (Ethernet) | **Caught** |

##### [`HardwareType: From<HardwareType> for u16`](../crates/firewall_core/src/net.rs#L310) (net.rs line 310)

| Mutation | Result |
|----------|--------|
| Replace `-> Self` with `Default::default()` | **MISSED** |

Dead code — the RxFirewall never serializes `HardwareType` back to `u16`.

##### [`Arp::parse`](../crates/firewall_core/src/net.rs#L350) (net.rs line 350)

| Mutation | Result |
|----------|--------|
| Replace `-> Option<Arp>` with `None` | **Caught** |
| Replace `-> Option<Arp>` with `Some(Default::default())` | Unviable |
| Delete `!` in `if !Self::allowed_ptype(&ptype)` (line 353) | **Caught** |

##### [`Arp::allowed_ptype`](../crates/firewall_core/src/net.rs#L380) (net.rs line 380)

| Mutation | Result |
|----------|--------|
| Replace `-> bool` with `true` | **MISSED** |
| Replace `-> bool` with `false` | **Caught** |

The `→ true` miss *(near-miss)*: `allowed_ptype` returns `false` only for
`EtherType::Arp` (rejecting ARP frames whose ptype field is itself ARP — a malformed
packet). Returning `true` always means this malformed-ARP check is bypassed. The Claude
ARP strategy always stamps `ptype=IPv4`, which passes `allowed_ptype` regardless. See
[Generator Design: Near-Miss Fuzzing](#generator-design-near-miss-fuzzing) — a test
with `ptype=Arp` (a malformed ARP packet) would catch this.

##### [`IpProtocol::try_from`](../crates/firewall_core/src/net.rs#L414) (net.rs lines 414–428)

| Mutation | Result |
|----------|--------|
| Delete match arm `0x00` (HopByHop) | **MISSED** |
| Delete match arm `0x01` (Icmp) | **MISSED** |
| Delete match arm `0x02` (Igmp) | **MISSED** |
| Delete match arm `0x06` (Tcp) | **MISSED** |
| Delete match arm `0x11` (Udp) | **Caught** |
| Delete match arm `0x2b` (Ipv6Route) | **MISSED** |
| Delete match arm `0x2c` (Ipv6Frag) | **MISSED** |
| Delete match arm `0x3a` (Icmpv6) | **MISSED** |
| Delete match arm `0x3b` (Ipv6NoNxt) | **MISSED** |
| Delete match arm `0x3c` (Ipv6Opts) | **MISSED** |

Only the UDP arm (0x11) deletion is caught. The RxFirewall's routing logic only has
two active paths: ARP (matched by `PacketType::Arp`) and UDP (matched via
`Ipv4ProtoPacket::Udp`). All other IP protocols — TCP, ICMP, IGMP, HopByHop,
Ipv6Route, Ipv6Frag, Icmpv6, Ipv6NoNxt, Ipv6Opts — hit the `_ =>` catch-all in
`can_send_to_vmm` and are dropped. If a protocol's match arm is deleted from
`IpProtocol::try_from`, that protocol byte causes `Ipv4Repr::parse` to return
`None` (frame fails to parse) instead of being parsed and then dropped. Either way,
no output is produced — the behavior is observationally equivalent.

The TCP arm (0x06) is a notable case: even though the app has `tcp_port_allowed`, it's
dead code (no `Tcp` match arm in `can_send_to_vmm`). Whether TCP fails to parse or
parses and gets dropped, the result is the same.

##### [`IpProtocol: From<IpProtocol> for u8`](../crates/firewall_core/src/net.rs#L433) (net.rs line 433)

| Mutation | Result |
|----------|--------|
| Replace `-> Self` with `Default::default()` | **MISSED** |

Dead code — the RxFirewall never serializes `IpProtocol` back to `u8`.

##### [`Ipv4Repr::parse`](../crates/firewall_core/src/net.rs#L475) (net.rs lines 475–485)

| Mutation | Result |
|----------|--------|
| Replace `-> Option<Ipv4Repr>` with `None` | **Caught** |
| Replace `-> Option<Ipv4Repr>` with `Some(Default::default())` | Unviable |
| Replace `!=` with `==` in IHL check (line 478) | **Caught** |
| Replace `>` with `<` in MTU check (line 481) | **Caught** |
| Replace `>` with `==` in MTU check (line 481) | **MISSED** |
| Replace `>` with `>=` in MTU check (line 481) | **MISSED** |

The MTU check misses *(near-miss)*: `if length > MAX_MTU` (where `MAX_MTU=9000`).
With `>` → `==`, only packets with length *exactly* 9000 are rejected. With `>` → `>=`,
packets with length 9000 are now rejected (previously accepted). The Claude strategies
stamp `length=60` (`0x003c`), which is far below the boundary. See
[Generator Design: Near-Miss Fuzzing](#generator-design-near-miss-fuzzing) — frames
with `length` near 9000 would test this boundary.

##### [`TcpRepr::parse`](../crates/firewall_core/src/net.rs#L509) (net.rs line 509)

| Mutation | Result |
|----------|--------|
| Replace `-> TcpRepr` with `Default::default()` | Unviable |

##### [`UdpRepr::parse`](../crates/firewall_core/src/net.rs#L538) (net.rs line 538)

| Mutation | Result |
|----------|--------|
| Replace `-> UdpRepr` with `Default::default()` | Unviable |

#### Analysis

##### Missed Mutations by Category

**Dead code — `From` serialization impls (4 mutations)**

| # | Mutation | Root Cause |
|---|----------|-----------|
| 1 | `From<EtherType> for u16` → `Default::default()` | RxFirewall never serializes EtherType to u16 |
| 2 | `From<ArpOp> for u16` → `Default::default()` | RxFirewall never serializes ArpOp to u16 |
| 3 | `From<HardwareType> for u16` → `Default::default()` | RxFirewall never serializes HardwareType to u16 |
| 4 | `From<IpProtocol> for u8` → `Default::default()` | RxFirewall never serializes IpProtocol to u8 |

All four `From` impls convert from enum to wire-format integer for serialization. The
RxFirewall is a *parser* — it reads frames, never writes them. These impls exist for
completeness in the `firewall_core` API but are never called by any RxFirewall code path.

**Observationally equivalent — non-UDP protocol arm deletions (9 mutations)**

| # | Mutation |
|---|----------|
| 5 | Delete `0x00` (HopByHop) arm |
| 6 | Delete `0x01` (Icmp) arm |
| 7 | Delete `0x02` (Igmp) arm |
| 8 | Delete `0x06` (Tcp) arm |
| 9 | Delete `0x2b` (Ipv6Route) arm |
| 10 | Delete `0x2c` (Ipv6Frag) arm |
| 11 | Delete `0x3a` (Icmpv6) arm |
| 12 | Delete `0x3b` (Ipv6NoNxt) arm |
| 13 | Delete `0x3c` (Ipv6Opts) arm |

The RxFirewall only routes ARP and UDP. All other IP protocols are dropped by
`can_send_to_vmm`'s catch-all. Whether a non-UDP protocol fails at parse time (arm
deleted) or at routing time (catch-all), the result is the same: no output. These
are observationally equivalent from the RxFirewall's perspective.

**[Near-miss fuzzing](#generator-design-near-miss-fuzzing) gaps (10 mutations)**

| # | Mutation | Near-miss input needed |
|---|----------|----------------------|
| 14 | `Ipv4Address::from_bytes`: `<` → `==` | *(observationally equivalent — RxFirewall never checks IPv4 addresses)* |
| 15 | `Ipv4Address::from_bytes`: `<` → `>` | *(same)* |
| 16 | `Address::is_empty` → `false` | All-zero destination MAC |
| 17 | `Address::is_empty`: `<` → `<=` | Same |
| 18 | `Address::is_empty`: `+=` → `-=` | Same |
| 19 | Delete `0x86DD` (Ipv6) in `EtherType::try_from` | *(observationally equivalent — IPv6 is dropped either way)* |
| 20 | Delete `2` (Reply) arm in `ArpOp::try_from` | ARP Reply frame |
| 21 | `Arp::allowed_ptype` → `true` | Malformed ARP with `ptype=Arp` |
| 22 | `Ipv4Repr::parse` MTU: `>` → `==` | Frame with `length` near 9000 |
| 23 | `Ipv4Repr::parse` MTU: `>` → `>=` | Frame with `length=9000` exactly |

Of these, #14–15 and #19 are genuinely observationally equivalent (the RxFirewall
never uses the mutated values). The remaining 7 (#16–18, #20–23) would be caught by
generators that include boundary-adjacent inputs as described in
[Generator Design: Near-Miss Fuzzing](#generator-design-near-miss-fuzzing).

##### Timeout Mutations

| # | Mutation | Mechanism |
|---|----------|-----------|
| 1 | `replace += with *= in Ipv4Address::from_bytes` | `0 *= 1` → infinite loop |
| 2 | `replace += with *= in Address::from_bytes` | Same pattern |
| 3 | `replace += with *= in Address::is_empty` | Same pattern |

Same `0 *= 1` infinite loop pattern as in the app-level mutations.

##### Unviable Mutations

| # | Mutation | Reason |
|---|----------|--------|
| 1 | `EthFrame::parse` → `Some(Default::default())` | `EthFrame` lacks `Default` |
| 2 | `Ipv4Address::from_bytes` → `Default::default()` | `Ipv4Address` lacks `Default` |
| 3 | `Address::from_bytes` → `Default::default()` | `Address` lacks `Default` |
| 4 | `EtherType::from_bytes` → `Some(Default::default())` | `EtherType` lacks `Default` |
| 5 | `EtherType::try_from` → `Ok(Default::default())` | Same |
| 6 | `EthernetRepr::parse` → `Some(Default::default())` | `EthernetRepr` lacks `Default` |
| 7 | `ArpOp::from_bytes` → `Some(Default::default())` | `ArpOp` lacks `Default` |
| 8 | `ArpOp::try_from` → `Ok(Default::default())` | Same |
| 9 | `HardwareType::from_bytes` → `Some(Default::default())` | `HardwareType` lacks `Default` |
| 10 | `HardwareType::try_from` → `Ok(Default::default())` | Same |
| 11 | `Arp::parse` → `Some(Default::default())` | `Arp` lacks `Default` |
| 12 | `IpProtocol::try_from` → `Ok(Default::default())` | `IpProtocol` lacks `Default` |
| 13 | `Ipv4Repr::parse` → `Some(Default::default())` | `Ipv4Repr` lacks `Default` |
| 14 | `TcpRepr::parse` → `Default::default()` | `TcpRepr` lacks `Default` |
| 15 | `UdpRepr::parse` → `Default::default()` | `UdpRepr` lacks `Default` |

All 15 unviable mutations fail because the target types use `#[derive(Debug)]` but
not `#[derive(Default)]`. The types are defined inside or alongside `verus!` blocks
where `Default` derivation is either unsupported or intentionally omitted.

#### Observations

1. **Core parsing functions are well-tested.** `u16_from_be_bytes` (6/6 caught),
   `EthernetRepr::parse`, `EtherType::from_bytes`, `Arp::parse`, `Address::from_bytes`,
   and `Ipv4Repr::parse` (critical mutations caught) — the parsing pipeline that the
   RxFirewall depends on for frame classification is thoroughly validated by the Claude
   tests.

2. **Serialization impls are dead code.** All four `From<X> for u16/u8` impls (EtherType,
   ArpOp, HardwareType, IpProtocol) are never called by the RxFirewall. These exist for
   the `firewall_core` API surface but are unreachable through the RxFirewall test paths.
   firewall_core's own unit tests (not used here) do cover these.

3. **Non-UDP protocol arms are observationally equivalent.** 9 of 23 misses are
   `IpProtocol::try_from` match arm deletions for protocols that the RxFirewall drops
   anyway. The frame either fails to parse (arm deleted) or parses and is dropped
   (catch-all) — same observable result. This is not a test gap; it reflects the
   RxFirewall's intentionally narrow routing scope.

4. **Input variety gaps are the dominant miss category.** 7 of 23 misses stem from
   the Claude strategies producing only "perfect" inputs for each category. These are
   all [near-miss fuzzing](#generator-design-near-miss-fuzzing) gaps: all-zero MACs,
   ARP Reply frames, malformed ARP ptype, and MTU boundary lengths would collectively
   catch them.

5. **High unviable rate.** 15 of 73 mutations (20.5%) are unviable because firewall_core
   types lack `Default`. This is expected for types defined in/around Verus verification
   blocks. The effective mutation pool is 58, of which 32 are caught (55.2%), or 35/58
   (60.3%) including timeouts.
