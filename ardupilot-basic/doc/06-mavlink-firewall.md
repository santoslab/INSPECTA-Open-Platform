# Mavlink Firewall

The Mavlink Firewall performs deep packet inspection on MAVLink messages carried within UDP payloads. It receives `UdpFrame_Impl` messages from the RX Firewall (already identified as MAVLink traffic), parses the UDP payload as a MAVLink message, checks it against a blacklist, and either forwards it as a reassembled `RawEthernetMessage` or drops it.

## Requirements

From `requirements/Inspecta-HLRs.pdf`, Section 3.0:

| HLR ID | Requirement | Summary |
|--------|-------------|---------|
| RC_INSPECTA_00-HLR-19 | Drop flash_bootloader mav_command message | Drop if the MAVLink message is CommandInt or CommandLong with command = `MAV_CMD_FLASH_BOOTLOADER` (42650) |
| RC_INSPECTA_00-HLR-20 | Drop malformed mavlink messages | Drop if the MAVLink message in the payload cannot be parsed |
| RC_INSPECTA_00-HLR-21 | No output on empty input | Produce no output when the input port has no data |
| RC_INSPECTA_00-HLR-22 | Copy through well-formed, not-blacklisted messages | Forward a MAVLink message if it is parseable and not blacklisted |

### Blacklist Check (HLR-19)

A message is blacklisted if **any** of the following hold:

**MAVLink v2 (`0xFD`):**
- Message ID (bytes 7-9, u24) = CommandInt (75) or CommandLong (76), AND
- Payload bytes 28-29 (little-endian u16) = 42650 (`MAV_CMD_FLASH_BOOTLOADER`)

**MAVLink v1 (`0xFE`):**
- Message ID (byte 5, u8) = CommandInt (75) or CommandLong (76), AND
- Payload bytes 28-29 (little-endian u16) = 42650 (`MAV_CMD_FLASH_BOOTLOADER`)

See [02-mavlink-message-format.md](02-mavlink-message-format.md) for complete MAVLink packet format details.

## GUMBO Contracts

### Uninterpreted Functions (`SW.sysml`, lines 458-465)

The Mavlink Firewall defines two `@spec def` functions whose semantics are too complex for GUMBO to express directly (they require parsing a binary protocol):

```gumbo
functions
    @spec def msg_is_wellformed(msg: UdpPayload): Base_Types::Boolean;
    @spec def msg_is_mav_cmd_flash_bootloader(msg: UdpPayload): Base_Types::Boolean;

    def msg_is_blacklisted(msg: UdpPayload): Base_Types::Boolean :=
        msg_is_mav_cmd_flash_bootloader(msg);
```

- **`msg_is_wellformed`**: True if the UDP payload can be successfully parsed as a MAVLink v1 or v2 message.
- **`msg_is_mav_cmd_flash_bootloader`**: True if the parsed message is a CommandInt or CommandLong with `MAV_CMD_FLASH_BOOTLOADER`.
- **`msg_is_blacklisted`**: Currently equals `msg_is_mav_cmd_flash_bootloader`. Designed as an extension point for adding future blacklisted commands.

The developer supplies the runtime and Verus spec implementations of these functions in the component code.

### Compute Guarantees (`SW.sysml`, lines 467-516)

Shown for port 0 (ports 1-3 are identical):

**HLR-19: Drop blacklisted command** (`hlr_19_mav0_drop_mav_cmd_flash_bootloader`)
```gumbo
(HasEvent(In0) and msg_is_wellformed(In0.payload) and msg_is_mav_cmd_flash_bootloader(In0.payload))
    implies NoSend(Out0);
```

**HLR-20: Drop malformed** (`hlr_20_mav0_drop_malformed_msg`)
```gumbo
(HasEvent(In0) and not msg_is_wellformed(In0.payload)) implies NoSend(Out0);
```

**HLR-21: No output on empty input** (`hlr_21_mav0_no_input`)
```gumbo
(not HasEvent(In0)) implies NoSend(Out0);
```

**HLR-22: Pass through allowed messages** (`hlr_22_mav0_allow`)
```gumbo
(HasEvent(In0) and msg_is_wellformed(In0.payload) and not msg_is_blacklisted(In0.payload))
    implies (HasEvent(Out0) and GumboLib::mav_input_eq_output(In0, Out0));
```

The output equality predicate `mav_input_eq_output` verifies that the reassembled `RawEthernetMessage` preserves all bytes from the input `UdpFrame_Impl`: headers go to bytes 0-41 and payload goes to bytes 42-1599.

## Developer-Supplied Specifications

The developer must provide both runtime (exec) and Verus (spec) implementations for the uninterpreted GUMBO functions.

### Verus Spec Functions (lines 50-90 of app file)

**`msg_is_wellformed__developer_verus`** (line 50):
```rust
pub open spec fn msg_is_wellformed__developer_verus(payload: SW::UdpPayload) -> bool {
    spec_mavlink_msg().spec_parse(payload@).is_some()
}
```
Delegates to the Vest-generated spec combinator. A payload is well-formed if the spec parser produces `Some`.

**`msg_is_mav_cmd_flash_bootloader__developer_verus`** (line 55):
```rust
pub open spec fn msg_is_mav_cmd_flash_bootloader__developer_verus(payload: SW::UdpPayload) -> bool {
    match spec_mavlink_msg().spec_parse(payload@) {
        Some((_, msg)) => spec_msg_is_flash_bootloader(msg),
        None => false,
    }
}
```
Parses the payload at the spec level, then checks if the parsed message is a flash bootloader command.

**`spec_msg_is_flash_bootloader`** (line 64):
Dispatches to v1 or v2 specific checks:
```rust
pub open spec fn spec_msg_is_flash_bootloader(msg: SpecMavlinkMsg) -> bool {
    spec_msg_v1_is_flash_bootloader(msg) || spec_msg_v2_is_flash_bootloader(msg)
}
```

**`spec_payload_get_cmd`** (line 83):
Extracts the command field from payload bytes 28-29:
```rust
pub open spec fn spec_payload_get_cmd(payload: Seq<u8>) -> Option<u16> {
    if payload.len() >= 30 {
        Some(u16::spec_from_le_bytes(payload.subrange(28, 30)))
    } else {
        None
    }
}
```

### Runtime (Exec) Functions

**`msg_is_wellformed__developer_gumbox`** (line 38):
```rust
pub fn msg_is_wellformed__developer_gumbox(payload: SW::UdpPayload) -> bool {
    parse_mavlink_msg(&payload).is_ok()
}
```

**`can_send`** (lines 124-135):
The combined well-formed + not-blacklisted check:
```rust
fn can_send(payload: SW::UdpPayload) -> (r: bool)
    ensures
        (msg_is_wellformed(payload) && !msg_is_blacklisted(payload)) == (r == true)
{
    match parse_mavlink_msg(&payload) {
        Ok((_, msg)) => !ex_msg_is_blacklisted(&msg),
        Err(_) => false,
    }
}
```

**`msg_is_flash_bootloader`** (lines 148-174):
Pattern matches on v1/v2, checks message IDs (CommandInt=75, CommandLong=76), extracts command via `payload_get_cmd`:
```rust
fn msg_is_flash_bootloader(msg: &MavlinkMsg) -> (r: bool)
    ensures r == spec_msg_is_flash_bootloader(msg@)
{
    let command = match &msg.msg {
        MavlinkMsgMsg::MavLink1(v1_msg) => match v1_msg.msgid {
            MessageIdsV1::CommandInt | MessageIdsV1::CommandLong =>
                payload_get_cmd(v1_msg.payload),
            _ => None,
        },
        MavlinkMsgMsg::MavLink2(v2_msg) => {
            let msgid = v2_msg.msgid.as_u32();
            match msgid {
                MessageIdsV2::CommandInt | MessageIdsV2::CommandLong =>
                    payload_get_cmd(v2_msg.payload),
                _ => None,
            }
        },
    };
    match command {
        Some(cmd) => cmd == MavCmd::FlashBootloader,
        None => false,
    }
}
```

**`payload_get_cmd`** (lines 179-188):
Extracts the command as a little-endian u16 from payload bytes 28-29:
```rust
fn payload_get_cmd(payload: &[u8]) -> (o: Option<u16>)
    ensures o == spec_payload_get_cmd(payload@),
{
    if payload.len() >= 30 {
        Some(u16::ex_from_le_bytes(slice_subrange(payload, 28, 30)))
    } else {
        None
    }
}
```

This is documented as a "workaround for current vest deficiency" -- Vest cannot yet parse dependent payload structures with truncation, so the command field must be extracted manually from the raw payload bytes.

**`raw_eth_from_udp_frame`** (lines 93-122):
Reassembles a `UdpFrame_Impl` back into a `RawEthernetMessage`:
```rust
fn raw_eth_from_udp_frame(value: SW::UdpFrame_Impl) -> (r: SW::RawEthernetMessage)
    ensures GumboLib::mav_input_eq_output_spec(value, r),
```
Copies headers (42 bytes) followed by payload (1558 bytes) into a 1600-byte array. The loop invariant proves byte-by-byte equality, satisfying the `mav_input_eq_output` GUMBO predicate.

### `timeTriggered` (lines 202-297)

The compute entry point is straightforward:

```rust
if let Some(udp_frame) = api.get_In0() {
    if can_send(udp_frame.payload) {
        let output = raw_eth_from_udp_frame(udp_frame);
        api.put_Out0(output);
    }
}
```

For each port: if input is present and `can_send` returns true (well-formed AND not blacklisted), reassemble and forward. Otherwise, produce no output.

### Heap Allocator

The Vest parser library requires a heap allocator, which is not available by default in a `no_std` seL4 Microkit environment. The component sets up a 16KB static heap:

```rust
const HEAP_SIZE: usize = 16 * 1024;
static HEAP: StaticHeap<HEAP_SIZE> = StaticHeap::new();
#[global_allocator]
static GLOBAL_ALLOCATOR: StaticDlmalloc<RawOneShotMutex> = StaticDlmalloc::new(HEAP.bounds());
```

## Vest Specification Cross-Reference

The Vest specification at `hamr/microkit/crates/mavlink_parser_vest/src/mavlink.vest` defines the parser structure. See [02-mavlink-message-format.md](02-mavlink-message-format.md) for a full walkthrough.

Key mapping between Vest and the application code:

| Vest Definition | Generated Rust Type | Used In |
|----------------|-------------------|---------|
| `mavlink_msg` | `MavlinkMsg` / `SpecMavlinkMsg` | `parse_mavlink_msg`, `spec_mavlink_msg` |
| `mavlink_v1_msg` | `MavlinkV1Msg` | `msg_is_flash_bootloader` v1 branch |
| `mavlink_v2_msg` | `MavlinkV2Msg` | `msg_is_flash_bootloader` v2 branch |
| `protocol_magic` | `ProtocolMagic` enum | Dispatches v1 vs v2 |
| `message_ids_v1` | `MessageIdsV1` | Checked for CommandInt/CommandLong |
| `message_ids_v2` | `MessageIdsV2` | Checked for CommandInt/CommandLong |
| `mav_cmd` | `MavCmd` | `FlashBootloader = 42650` |

The payload is parsed as raw `[u8; @len]` by Vest. The command field extraction at bytes 28-29 is hand-written in `payload_get_cmd` because Vest does not yet support dependent payload structure refinement with MAVLink v2's trailing-zero truncation.

## Traceability Matrix

| HLR | GUMBO Guarantee | Verus Ensures | Code Path |
|-----|-----------------|---------------|-----------|
| HLR-19 | `hlr_19_mav{0-3}_drop_mav_cmd_flash_bootloader` | `msg_is_wellformed(payload) && msg_is_mav_cmd_flash_bootloader(payload) ==> Out.is_none()` | `can_send` returns false (`parse_mavlink_msg` succeeds, `ex_msg_is_blacklisted` returns true) |
| HLR-20 | `hlr_20_mav{0-3}_drop_malformed_msg` | `!msg_is_wellformed(payload) ==> Out.is_none()` | `can_send` returns false (`parse_mavlink_msg` returns `Err`) |
| HLR-21 | `hlr_21_mav{0-3}_no_input` | `!In.is_some() ==> Out.is_none()` | `get_In` returns `None` |
| HLR-22 | `hlr_22_mav{0-3}_allow` | `msg_is_wellformed(payload) && !msg_is_blacklisted(payload) ==> Out.is_some() && mav_input_eq_output_spec(In, Out)` | `can_send` returns true -> `raw_eth_from_udp_frame` -> `put_Out` |
