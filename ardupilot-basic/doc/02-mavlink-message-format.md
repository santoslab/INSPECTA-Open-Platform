# MAVLink Message Format and Vest Specification

This document describes the MAVLink v1 and v2 message formats as they relate to the Mavlink Firewall component, and walks through the Vest parser specification that defines the parser used by the firewall.

## MAVLink Overview

MAVLink (Micro Air Vehicle Link) is a lightweight messaging protocol for communicating with drones and between onboard drone components. Messages are exchanged between a ground control station (GCS) and the UAV over UDP (source port 14550, destination port 14562).

The Mavlink Firewall inspects MAVLink messages within UDP payloads to:
1. Verify that messages are well-formed (parseable).
2. Drop messages containing blacklisted commands (currently `MAV_CMD_FLASH_BOOTLOADER`).
3. Pass through all other well-formed messages.

## MAVLink v1 Packet Format

MAVLink v1 packets are identified by the magic byte `0xFE` at the start of the UDP payload.

| Byte Offset (within payload) | Field | Size | Description |
|------------------------------|-------|------|-------------|
| 0 | Magic (STX) | 1 byte | Protocol marker: `0xFE` for MAVLink v1 |
| 1 | Payload Length | 1 byte | Length of the message payload in bytes |
| 2 | Sequence | 1 byte | Packet sequence number (0-255, wrapping) |
| 3 | System ID | 1 byte | ID of the sending system (must be >= 1) |
| 4 | Component ID | 1 byte | ID of the sending component (must be >= 1) |
| 5 | Message ID | 1 byte | Identifies the message type |
| 6 .. 6+len-1 | Payload | len bytes | Message-specific data (up to 255 bytes) |
| 6+len .. 6+len+1 | Checksum | 2 bytes | CRC-16/MCRF4XX over the packet |

**Total packet size:** 8 + payload length bytes.

### Relevant v1 Message IDs

| Message ID | Name | Decimal |
|------------|------|---------|
| CommandInt | `COMMAND_INT` | 75 |
| CommandLong | `COMMAND_LONG` | 76 |
| CommandAck | `COMMAND_ACK` | 77 |

### Command Extraction (v1)

For CommandInt (ID 75) and CommandLong (ID 76) messages, the MAV command field is a little-endian u16 located at **bytes 28-29 of the message payload** (payload-relative offset, not frame-relative). This corresponds to the `command` field that appears after seven 4-byte parameter fields (7 x 4 = 28 bytes).

## MAVLink v2 Packet Format

MAVLink v2 packets are identified by the magic byte `0xFD`.

| Byte Offset (within payload) | Field | Size | Description |
|------------------------------|-------|------|-------------|
| 0 | Magic (STX) | 1 byte | Protocol marker: `0xFD` for MAVLink v2 |
| 1 | Payload Length | 1 byte | Length of the message payload in bytes |
| 2 | Incompatibility Flags | 1 byte | Flags that must be understood by the receiver |
| 3 | Compatibility Flags | 1 byte | Flags that can be ignored if not understood |
| 4 | Sequence | 1 byte | Packet sequence number |
| 5 | System ID | 1 byte | ID of the sending system (must be >= 1) |
| 6 | Component ID | 1 byte | ID of the sending component (must be >= 1) |
| 7-9 | Message ID | 3 bytes | 24-bit message identifier (little-endian) |
| 10 .. 10+len-1 | Payload | len bytes | Message-specific data (up to 255 bytes) |
| 10+len .. 10+len+1 | Checksum | 2 bytes | CRC-16/MCRF4XX |
| 10+len+2 .. 10+len+14 | Signature | 13 bytes | Optional; present if incompatibility flag bit 0 is set |

**Total packet size:** 12 + payload length + (13 if signed) bytes.

### Relevant v2 Message IDs

| Message ID | Name | Value (24-bit) |
|------------|------|----------------|
| CommandInt | `COMMAND_INT` | 75 |
| CommandLong | `COMMAND_LONG` | 76 |
| CommandAck | `COMMAND_ACK` | 77 |

### Incompatibility Flags

| Flag Value | Name | Meaning |
|------------|------|---------|
| `0x01` | Signed | A 13-byte signature is appended after the checksum |

### Command Extraction (v2)

The same as v1: the MAV command field is at **payload bytes 28-29** (little-endian u16). The absolute position within the UDP payload differs from v1 because v2 has additional header fields (incompatibility flags, compatibility flags, and a 3-byte message ID).

## Blacklisted Command

| Command | Value | Description |
|---------|-------|-------------|
| `MAV_CMD_FLASH_BOOTLOADER` | 42650 | Commands the vehicle to flash its bootloader firmware. This is a safety-critical command that could be exploited to compromise the UAV's firmware. |

The Mavlink Firewall drops any MAVLink message (v1 or v2) where:
- The message ID is CommandInt (75) or CommandLong (76), AND
- The command field at payload bytes 28-29 equals 42650 (`MAV_CMD_FLASH_BOOTLOADER`).

## Requirements Summary

| HLR | Description |
|-----|-------------|
| HLR-19 | Drop `MAV_CMD_FLASH_BOOTLOADER` command messages (CommandInt or CommandLong with command=42650) |
| HLR-20 | Drop malformed (unparseable) MAVLink messages |
| HLR-21 | No output when no input is available |
| HLR-22 | Pass through well-formed, non-blacklisted messages unchanged |

## The Vest Specification

The MAVLink parser is generated from a Vest specification file located at:
```
hamr/microkit/crates/mavlink_parser_vest/src/mavlink.vest
```

Vest is a domain-specific language for specifying binary message formats. The Vest tool generates verified Rust parser combinators from these specifications, producing both:
- **Executable parsers** (`parse_mavlink_msg`) for runtime use.
- **Specification combinators** (`spec_mavlink_msg`) for use in Verus proofs.

### Vest DSL Syntax

Vest specifications define message formats as sequences of typed fields with optional constraints. Key syntax elements:

- **`field_name: type`** -- a named field with a given type (e.g., `seq: u8`)
- **`@field_name: type`** -- a "binding" field whose value can be referenced by later fields (e.g., `@len: u8` makes `len` available for dependent sizing)
- **`[u8; @len]`** -- a byte array whose length depends on a previously-bound field
- **`choose(@field) { Case1 => type1, Case2 => type2 }`** -- a tagged union that selects a variant based on a previously-bound field
- **`type | constraint`** -- a refined type with a constraint (e.g., `u8 | 1..` means "u8 with value >= 1")
- **`enum { Name = value, ... }`** -- an enumeration mapping names to numeric values. A trailing `...` indicates an open enum (unknown values are accepted).

### Walkthrough of `mavlink.vest`

#### Top-Level Message

```vest
mavlink_msg = {
    @magic: protocol_magic,
    msg: choose(@magic) {
        MavLink1 => mavlink_v1_msg,
        MavLink2 => mavlink_v2_msg,
    },
}
```

The top-level message reads a `protocol_magic` byte, then dispatches to either the v1 or v2 message format based on its value.

#### Protocol Magic

```vest
protocol_magic = enum {
    MavLink1 = 0xFE,
    MavLink2 = 0xFD,
}
```

A u8 enum distinguishing v1 (`0xFE`) from v2 (`0xFD`). Any other value fails parsing (the message is malformed).

#### MAVLink v2 Message

```vest
mavlink_v2_msg = {
    @len: u8,
    @incompat_flags: incompat_flags,
    compat_flags: u8,
    seq: u8,
    sysid: u8 | 1..,
    compid: u8 | 1..,
    msgid: message_ids_v2,
    payload: [u8; @len],
    checksum: u16,
    signature: choose(@incompat_flags) {
        0x01 => [u8; 13],
        _ => [u8; 0],
    },
}
```

Key points:
- `@len` binds the payload length for use in `[u8; @len]`.
- `sysid` and `compid` must be >= 1 (the `| 1..` constraint).
- `msgid` is a 3-byte (u24) enum.
- The payload is parsed as raw bytes (`[u8; @len]`), not as a structured message type.
- The optional signature is present only when `incompat_flags` is `0x01` (Signed).

#### MAVLink v1 Message

```vest
mavlink_v1_msg = {
    @len: u8,
    seq: u8,
    sysid: u8 | 1..,
    compid: u8 | 1..,
    msgid: message_ids_v1,
    payload: [u8; @len],
    checksum: u16,
}
```

Similar to v2 but without incompatibility/compatibility flags, with a 1-byte message ID, and without the optional signature.

#### Message ID Enums

```vest
message_ids_v2 = enum {
    CommandInt = 75,
    CommandLong = 76,
    CommandAck = 77,
    Reserved = 0x800000,
    ...
}

message_ids_v1 = enum {
    CommandInt = 75,
    CommandLong = 76,
    CommandAck = 77,
    ...
}
```

The v2 message ID is a u24 (3 bytes). The `Reserved = 0x800000` entry forces the enum to be generated as a u24 type. The `...` makes both enums open, accepting any value (unknown message IDs are valid MAVLink messages).

#### Incompatibility Flags

```vest
incompat_flags = enum {
    Signed = 0x01,
    ...
}
```

Open enum allowing unknown flag values to be accepted.

#### MAV Command Enum

```vest
mav_cmd = enum {
    FlashBootloader = 42650,
    ...
}
```

The command enum used by the blacklist check. Currently only `FlashBootloader` is named, with the `...` accepting all other command values.

### Commented-Out Payload Type Refinements

The Vest specification includes commented-out definitions for `command_long`, `command_int`, and `command_ack` structures. These would allow Vest to parse the payload as a structured type based on the message ID:

```vest
// payload: [u8; @len] >>= choose (@msgid) {
//     CommandInt => command_int,
//     CommandLong => command_long,
//     CommandAck => command_ack,
//     _ => [u8; @len],
// },
```

This dependent refinement (`>>=`) is not currently used because of a complication with MAVLink v2 payload truncation: MAVLink v2 implementations truncate trailing zero bytes from payloads before transmission, which means the actual payload length may be shorter than the expected structure size. Handling this correctly in Vest would require additional combinator support.

Instead, the payload is parsed as raw bytes, and the command field is extracted manually at bytes 28-29 in the application code.

## What Vest Generates vs. What Is Hand-Written

### Vest-Generated (in `mavlink_parser_vest` crate)

| Artifact | Purpose |
|----------|---------|
| `parse_mavlink_msg(&[u8])` | Executable parser: returns `Ok((remaining, MavlinkMsg))` or `Err` |
| `spec_mavlink_msg()` | Verus spec combinator: `spec_parse(seq)` returns `Option<(usize, SpecMavlinkMsg)>` |
| `MavlinkMsg`, `MavlinkV1Msg`, `MavlinkV2Msg` | Rust types for parsed messages |
| `SpecMavlinkMsg`, `SpecMavlinkMsgMsg` | Verus spec types for verification |
| `MessageIdsV1`, `MessageIdsV2`, `MavCmd` | Enum types with named constants |

### Hand-Written (in `seL4_MavlinkFirewall_MavlinkFirewall_app.rs`)

| Function | Purpose |
|----------|---------|
| `payload_get_cmd(payload)` | Extracts the command field from payload bytes 28-29 (little-endian u16) |
| `spec_payload_get_cmd(payload)` | Verus spec version of the above |
| `msg_is_flash_bootloader(msg)` | Checks if a parsed message is a `MAV_CMD_FLASH_BOOTLOADER` command |
| `spec_msg_is_flash_bootloader(msg)` | Verus spec version |
| `can_send(payload)` | Combined check: parse + not blacklisted |
| `raw_eth_from_udp_frame(frame)` | Reassembles a `UdpFrame_Impl` back into a `RawEthernetMessage` |

The hand-written code bridges the gap between what Vest can express (structural parsing) and what the firewall needs (semantic inspection of parsed payload fields).
