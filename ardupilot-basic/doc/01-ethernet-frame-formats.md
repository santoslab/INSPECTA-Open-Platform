# Ethernet Frame Formats

This document describes the network frame formats relevant to the RX Firewall, TX Firewall, and Mavlink Firewall components. Only the fields that the firewalls inspect are covered here; for complete protocol specifications, refer to the relevant RFCs and IEEE standards.

All frames are carried in a `RawEthernetMessage`, a fixed-size `[u8; 1600]` byte array. The firewalls examine specific byte offsets within this array to classify and validate frames.

## Ethernet II Header

The first 14 bytes of every frame contain the Ethernet II header.

| Byte Offset | Field | Size | Description |
|-------------|-------|------|-------------|
| 0-5 | Destination MAC Address | 6 bytes | Hardware address of the intended recipient |
| 6-11 | Source MAC Address | 6 bytes | Hardware address of the sender |
| 12-13 | EtherType | 2 bytes | Protocol identifier (big-endian) |

### EtherType Values

| Value | Protocol | Hex |
|-------|----------|-----|
| `0x0800` | IPv4 | `aframe#(12) == 8, aframe#(13) == 0` |
| `0x0806` | ARP | `aframe#(12) == 8, aframe#(13) == 6` |
| `0x86DD` | IPv6 | `aframe#(12) == 134, aframe#(13) == 221` |

### Firewall Checks on the Ethernet Header

**EtherType validity** (`valid_frame_ethertype`): The EtherType must be one of IPv4, ARP, or IPv6.

**Destination address validity** (`valid_frame_dst_addr`): The destination MAC address must not be all zeros (bytes 0-5 must not all be `0x00`). An all-zero destination indicates an invalid or uninitialized frame.

**Well-formed Ethernet header** (`frame_is_wellformed_eth2`): Both checks must pass -- valid EtherType AND valid destination address.

## ARP Packet

When the EtherType is `0x0806` (ARP), the ARP packet begins at byte 14.

| Byte Offset | Field | Size | Description |
|-------------|-------|------|-------------|
| 14-15 | Hardware Type (htype) | 2 bytes | Type of hardware address (big-endian) |
| 16-17 | Protocol Type (ptype) | 2 bytes | Type of protocol address (big-endian) |
| 18 | Hardware Address Length (hlen) | 1 byte | Length of hardware addresses |
| 19 | Protocol Address Length (plen) | 1 byte | Length of protocol addresses |
| 20-21 | Operation (op) | 2 bytes | ARP operation code (big-endian) |
| 22+ | Addresses | Variable | Sender/target hardware and protocol addresses |

### Firewall Checks on ARP

| Check | GumboLib Predicate | Condition |
|-------|-------------------|-----------|
| Valid hardware type | `valid_arp_htype` | Bytes 14-15 = `0x0001` (Ethernet) |
| Valid protocol type | `valid_arp_ptype` | Bytes 16-17 = `0x0800` (IPv4) or `0x86DD` (IPv6) |
| Valid operation | `valid_arp_op` | Byte 20 = `0x00` AND byte 21 = `0x01` (Request) or `0x02` (Reply) |
| Well-formed ARP | `wellformed_arp_frame` | All three checks pass |

**Composite ARP check** (`valid_arp`): A frame is a valid ARP frame if the Ethernet header is well-formed, the EtherType indicates ARP, and the ARP packet is well-formed.

## IPv4 Header

When the EtherType is `0x0800` (IPv4), the IPv4 header begins at byte 14.

| Byte Offset | Field | Size | Description |
|-------------|-------|------|-------------|
| 14 | Version + IHL | 1 byte | Upper nibble = version, lower nibble = header length in 32-bit words |
| 15 | DSCP / ECN | 1 byte | Differentiated services (not checked by firewalls) |
| 16-17 | Total Length | 2 bytes | Total length of the IPv4 packet in bytes (big-endian) |
| 18-19 | Identification | 2 bytes | Fragment identification (not checked) |
| 20-21 | Flags + Fragment Offset | 2 bytes | Fragmentation control (not checked) |
| 22 | TTL | 1 byte | Time to live (not checked) |
| 23 | Protocol | 1 byte | Upper-layer protocol identifier |
| 24-25 | Header Checksum | 2 bytes | (not checked by firewalls) |
| 26-29 | Source IP Address | 4 bytes | (not checked by firewalls) |
| 30-33 | Destination IP Address | 4 bytes | (not checked by firewalls) |

### Firewall Checks on IPv4

| Check | GumboLib Predicate | Condition |
|-------|-------------------|-----------|
| Version and IHL | `valid_ipv4_vers_ihl` | Byte 14 = `0x45` (version 4, IHL = 5, no options) |
| Total length | `valid_ipv4_length` | Big-endian u16 at bytes 16-17 <= 9000 |
| Valid protocol | `valid_ipv4_protocol` | Byte 23 is one of: 0 (HopByHop), 1 (ICMP), 2 (IGMP), 6 (TCP), 17 (UDP), 43 (IPv6Route), 44 (IPv6Frag), 58 (ICMPv6), 59 (IPv6NoNxt), 60 (IPv6Opts) |
| Well-formed IPv4 | `wellformed_ipv4_frame` | All three checks pass |

### IPv4 Protocol Numbers

| Value | Decimal | Protocol | Hex |
|-------|---------|----------|-----|
| `0x00` | 0 | Hop-by-Hop Options | |
| `0x01` | 1 | ICMP | |
| `0x02` | 2 | IGMP | |
| `0x06` | 6 | TCP | |
| `0x11` | 17 | UDP | |
| `0x2B` | 43 | IPv6 Routing | |
| `0x2C` | 44 | IPv6 Fragment | |
| `0x3A` | 58 | ICMPv6 | |
| `0x3B` | 59 | IPv6 No Next Header | |
| `0x3C` | 60 | IPv6 Destination Options | |

**Note:** The maximum IPv4 total length of 9000 bytes is chosen because it is the standard jumbo frame size for Maximum Transmission Unit (MTU). In practice, most frames will be closer to 1500 bytes.

**Composite IPv4 check** (`valid_ipv4`): A frame is a valid IPv4 frame if the Ethernet header is well-formed, the EtherType indicates IPv4, and the IPv4 header is well-formed.

## UDP Header

When the IPv4 protocol field is `0x11` (UDP), the UDP header begins at byte 34 (14 bytes Ethernet + 20 bytes IPv4).

| Byte Offset | Field | Size | Description |
|-------------|-------|------|-------------|
| 34-35 | Source Port | 2 bytes | Sender's port number (big-endian) |
| 36-37 | Destination Port | 2 bytes | Receiver's port number (big-endian) |
| 38-39 | Length | 2 bytes | Length of UDP header + data (not checked) |
| 40-41 | Checksum | 2 bytes | (not checked by firewalls) |
| 42+ | Payload | Variable | Application data (e.g., MAVLink messages) |

### Firewall Checks on UDP

**RX Firewall classification** -- the RX Firewall uses UDP port numbers to route frames:

| Classification | GumboLib Predicate | Condition |
|---------------|-------------------|-----------|
| MAVLink source port | `udp_is_mavlink_src_port` | Big-endian u16 at bytes 34-35 = 14550 |
| MAVLink destination port | `udp_is_mavlink_dst_port` | Big-endian u16 at bytes 36-37 = 14562 |
| MAVLink UDP frame | `udp_is_mavlink` | Both source port = 14550 AND destination port = 14562 |
| Allowed direct UDP port | `udp_is_valid_direct_dst_port` | Destination port is in `UDP_ALLOWED_PORTS` = [68] |
| Is UDP protocol | `ipv4_is_udp` | Byte 23 = 17 (`0x11`) |

**Port significance:**
- **14550** is the standard MAVLink ground control station (GCS) port.
- **14562** is the standard MAVLink ArduPilot port.
- **68** is the DHCP client port (the only currently whitelisted direct UDP port).

**Composite UDP checks:**
- `valid_ipv4_udp`: Well-formed Ethernet + IPv4 + EtherType is IPv4 + protocol is UDP.
- `valid_ipv4_udp_port`: Valid IPv4 UDP + destination port is in the allowed list + NOT a MAVLink frame.
- `valid_ipv4_udp_mavlink`: Valid IPv4 UDP + is a MAVLink frame (src=14550, dst=14562).

## TCP Header

When the IPv4 protocol field is `0x06` (TCP), the TCP header begins at byte 34.

| Byte Offset | Field | Size | Description |
|-------------|-------|------|-------------|
| 34-35 | Source Port | 2 bytes | Sender's port number (big-endian) |
| 36-37 | Destination Port | 2 bytes | Receiver's port number (big-endian) |
| 38+ | Sequence Number, etc. | Variable | (not checked by firewalls) |

### Firewall Checks on TCP

| Check | GumboLib Predicate | Condition |
|-------|-------------------|-----------|
| Is TCP protocol | `ipv4_is_tcp` | Byte 23 = 6 (`0x06`) |
| Allowed TCP port | `tcp_is_valid_port` | Big-endian u16 at bytes 36-37 = 5760 |

**Note:** Port 5760 is a standard MAVLink TCP port. The TCP port checking predicates are defined in GumboLib and present in the GUMBO contracts, but TCP routing is not currently active in the RX Firewall implementation.

## RX Firewall Decision Logic

The RX Firewall uses the composite predicates to make its routing decision (`rx_allow_outbound_frame`):

```
rx_allow_outbound_frame(frame) =
    valid_arp(frame)                   -- route to VMM
    OR valid_ipv4_udp_mavlink(frame)   -- route to Mavlink Firewall
    OR valid_ipv4_udp_port(frame)      -- route to VMM
```

Any frame that does not match one of these three categories is dropped.

## TX Firewall Decision Logic

The TX Firewall uses a simpler decision (`tx_allow_outbound_frame`):

```
tx_allow_outbound_frame(frame) =
    valid_arp(frame)    -- forward with size 64
    OR valid_ipv4(frame) -- forward with size = IPv4 total length + 14
```

## Byte Offset Reference Table

A consolidated reference of all byte offsets checked by the firewalls:

| Bytes | Field | Values Checked |
|-------|-------|---------------|
| 0-5 | Ethernet Destination MAC | Must not be all zeros |
| 12-13 | Ethernet EtherType | `0x0800` (IPv4), `0x0806` (ARP), `0x86DD` (IPv6) |
| 14 | IPv4 Version + IHL | Must be `0x45` |
| 14-15 | ARP Hardware Type | Must be `0x0001` |
| 16-17 | ARP Protocol Type | `0x0800` or `0x86DD` |
| 16-17 | IPv4 Total Length | Must be <= 9000 (big-endian) |
| 20-21 | ARP Operation | `0x0001` (Request) or `0x0002` (Reply) |
| 23 | IPv4 Protocol | One of: 0, 1, 2, 6, 17, 43, 44, 58, 59, 60 |
| 34-35 | UDP/TCP Source Port | 14550 for MAVLink GCS identification |
| 36-37 | UDP/TCP Destination Port | 14562 (MAVLink), 68 (DHCP), 5760 (MAVLink TCP) |

## Byte Conversion in GUMBO

The GumboLib library provides helper functions for interpreting multi-byte fields:

- **`two_bytes_to_u16_be(byte0, byte1)`**: Converts two bytes to a big-endian u16: `byte0 * 256 + byte1`. Used for EtherType, IPv4 length, and port numbers.
- **`two_bytes_to_u16_le(byte0, byte1)`**: Converts two bytes to a little-endian u16: `byte1 * 256 + byte0`. Used in some internal computations.
- **`three_bytes_to_u32(byte0, byte1, byte2)`**: Converts three bytes to a u32. Used for MAVLink v2 message IDs.
