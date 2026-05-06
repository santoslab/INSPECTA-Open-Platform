# TX Firewall

The TX Firewall validates outbound ethernet frames from the ArduPilot VM before they reach the physical network. It ensures only well-formed ARP and IPv4 frames are transmitted, and attaches the correct frame size for the ethernet driver.

## Requirements

From `requirements/Inspecta-HLRs.pdf`, Section 2.0:

| HLR ID | Requirement | Summary |
|--------|-------------|---------|
| RC_INSPECTA_00-HLR-7 | Copy through any ARP frame | Forward well-formed ARP frames with output size = 64 bytes |
| RC_INSPECTA_00-HLR-12 | Copy through any IPv4 frame | Forward well-formed IPv4 frames with output size = IPv4 total length + 14 (ethernet header) |
| RC_INSPECTA_00-HLR-14 | Do not copy disallowed frame | Drop frames that are not well-formed ARP or IPv4 |
| RC_INSPECTA_00-HLR-16 | No output on empty input | Produce no output when the input port has no data |

### What "Well-Formed" Means

**ARP frame** (HLR-7):
- Ethernet header: valid EtherType (`0x0800`, `0x0806`, or `0x86DD`) and destination MAC not all zeros
- EtherType is ARP (`0x0806`)
- ARP: hardware type = `0x0001`, protocol type = `0x0800` or `0x86DD`, operation = `0x0001` or `0x0002`

**IPv4 frame** (HLR-12):
- Ethernet header: valid EtherType and destination MAC not all zeros
- EtherType is IPv4 (`0x0800`)
- IPv4: version+IHL = `0x45`, total length <= 9000, protocol is one of the 10 allowed values

See [01-ethernet-frame-formats.md](01-ethernet-frame-formats.md) for complete byte-level details.

## GUMBO Contracts

### Integration Guarantees (`SW.sysml`, lines 337-355)

The TX Firewall guarantees that every output message is either a valid ARP frame with size 64, or a valid IPv4 frame with the correct computed size:

```gumbo
guarantee valid_tx_out_message_port0:
    (GumboLib::valid_arp(EthernetFramesTxOut0.amessage)
        and GumboLib::valid_output_arp_size(EthernetFramesTxOut0))
    or (GumboLib::valid_ipv4(EthernetFramesTxOut0.amessage)
        and GumboLib::valid_output_ipv4_size(EthernetFramesTxOut0.amessage, EthernetFramesTxOut0));
```

This integration guarantee flows to the `LowLevelEthernetDriver`, which assumes it as an integration constraint on its input ports.

### Compute Guarantees (`SW.sysml`, lines 357-407)

The compute contracts specify behavior for each dispatch cycle. Shown here for port 0 (ports 1-3 are identical):

**HLR-7: Forward valid ARP** (`hlr_07_tx0_can_send_valid_arp`)
```gumbo
(HasEvent(EthernetFramesTxIn0) and GumboLib::valid_arp(EthernetFramesTxIn0)) implies
    (HasEvent(EthernetFramesTxOut0)
        and (EthernetFramesTxIn0 == EthernetFramesTxOut0.amessage)
        and GumboLib::valid_output_arp_size(EthernetFramesTxOut0));
```
If the input is a valid ARP frame, the output must contain the same frame data (`amessage`) with size 64.

**HLR-12: Forward valid IPv4** (`hlr_12_tx0_can_send_valid_ipv4`)
```gumbo
(HasEvent(EthernetFramesTxIn0) and GumboLib::valid_ipv4(EthernetFramesTxIn0)) implies
    (HasEvent(EthernetFramesTxOut0)
        and (EthernetFramesTxIn0 == EthernetFramesTxOut0.amessage)
        and GumboLib::valid_output_ipv4_size(EthernetFramesTxIn0, EthernetFramesTxOut0));
```
If the input is a valid IPv4 frame, the output must contain the same frame data with size = `two_bytes_to_u16_be(frame[16], frame[17]) + 14`.

**HLR-14: Drop disallowed** (`hlr_14_tx0_disallow`)
```gumbo
(HasEvent(EthernetFramesTxIn0)
    and not GumboLib::tx_allow_outbound_frame(EthernetFramesTxIn0)) implies
    NoSend(EthernetFramesTxOut0);
```

**HLR-16: No output on empty input** (`hlr_16_tx0_no_input`)
```gumbo
(not HasEvent(EthernetFramesTxIn0)) implies NoSend(EthernetFramesTxOut0);
```

## Implementation

**Source file:** `hamr/microkit/crates/seL4_TxFirewall_TxFirewall/src/component/seL4_TxFirewall_TxFirewall_app.rs`

### Key Functions

#### `get_frame_packet` (lines 48-61)

Parses a `RawEthernetMessage` into a structured `EthFrame` using the `firewall_core` library:

```rust
fn get_frame_packet(frame: &SW::RawEthernetMessage) -> (r: Option<EthFrame>)
    requires
        frame@.len() == SW_RawEthernetMessage_DIM_0
    ensures
        valid_arp_spec(*frame) == firewall_core::res_is_arp(r),
        valid_ipv4_spec(*frame) == firewall_core::res_is_ipv4(r),
        valid_ipv4_spec(*frame) ==> firewall_core::ipv4_length_bytes_match(frame, r),
```

The Verus `ensures` clauses establish the critical link between:
- **GumboLib spec predicates** (`valid_arp_spec`, `valid_ipv4_spec`) that encode the GUMBO-level byte checks
- **firewall_core parse results** (`res_is_arp`, `res_is_ipv4`) that represent the structured parse outcome

This bridge allows Verus to verify that the structured parsing in `firewall_core` agrees with the byte-level predicates in GumboLib.

#### `can_send_packet` (lines 18-34)

Determines whether a parsed frame should be forwarded and computes the output size:

```rust
fn can_send_packet(packet: &PacketType) -> (r: Option<u16>)
    requires
        (packet is Ipv4) ==> (firewall_core::ipv4_valid_length(*packet))
    ensures
        (packet is Arp || packet is Ipv4) == r.is_some(),
        packet is Arp ==> (r == Some(64u16)),
        packet is Ipv4 ==> (r == Some((packet->Ipv4_0.header.length + EthernetRepr::SIZE) as u16)),
```

- ARP: returns `Some(64)` (fixed size per HLR-7)
- IPv4: returns `Some(ipv4_total_length + 14)` (ethernet header size per HLR-12)
- IPv6: returns `None` (dropped)

#### `timeTriggered` (lines 72-199)

The main compute entry point. For each of the 4 ports:

```rust
if let Some(frame) = api.get_EthernetFramesTxIn0() {     // Check for input
    if let Some(eth) = Self::get_frame_packet(&frame) {    // Parse frame
        if let Some(size) = can_send_packet(&eth.eth_type) { // Check if allowed + get size
            let out = SW::SizedEthernetMessage_Impl {
                sz: size,
                amessage: frame,
            };
            api.put_EthernetFramesTxOut0(out);              // Forward with size
        }
    }
}
```

The three nested `if let` expressions implement the requirements:
1. **No input** (`get_EthernetFramesTxIn0()` returns `None`): nothing happens -> HLR-16
2. **Malformed frame** (`get_frame_packet()` returns `None`): nothing happens -> HLR-14
3. **Disallowed frame type** (`can_send_packet()` returns `None`): nothing happens -> HLR-14
4. **Valid ARP or IPv4** (`can_send_packet()` returns `Some(size)`): output emitted -> HLR-7/HLR-12

The Verus `ensures` clauses on `timeTriggered` (lines 85-146) directly correspond to the GUMBO compute guarantees, with GumboLib predicates appearing as `GumboLib::valid_arp_spec(...)`, `GumboLib::valid_ipv4_spec(...)`, etc.

### The `firewall_core` Library

The TX Firewall depends on `firewall_core` (`hamr/microkit/crates/firewall_core/`) for parsing:

- `EthFrame::parse(frame)` -- Parses a byte slice into an `EthFrame` containing an `EthernetRepr` header and a `PacketType` (Arp, Ipv4, or Ipv6). Returns `None` for malformed frames.
- The parse function has Verus postconditions that align its results with GumboLib spec predicates, enabling end-to-end verification.

## Traceability Matrix

| HLR | GUMBO Guarantee | Verus Ensures | Code Path |
|-----|-----------------|---------------|-----------|
| HLR-7 | `hlr_07_tx{0-3}_can_send_valid_arp` | `valid_arp_spec(input) ==> output.is_some() && output.amessage == input && output.sz == 64` | `get_frame_packet` -> `can_send_packet` returns `Some(64)` -> `put_EthernetFramesTxOut` |
| HLR-12 | `hlr_12_tx{0-3}_can_send_valid_ipv4` | `valid_ipv4_spec(input) ==> output.is_some() && output.amessage == input && output.sz == ipv4_len + 14` | `get_frame_packet` -> `can_send_packet` returns `Some(len + 14)` -> `put_EthernetFramesTxOut` |
| HLR-14 | `hlr_14_tx{0-3}_disallow` | `!tx_allow_outbound_frame_spec(input) ==> output.is_none()` | `get_frame_packet` returns `None` or `can_send_packet` returns `None` |
| HLR-16 | `hlr_16_tx{0-3}_no_input` | `!input.is_some() ==> output.is_none()` | `get_EthernetFramesTxIn` returns `None`, no output API called |
