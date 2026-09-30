# Telemetry frame format, revision 3

Frames travel from field nodes to the gateway over LoRa at SF9, so every byte costs
airtime. Revision 3 shrinks the common frame from 24 to 17 bytes and adds a
delta encoding for slowly changing sensors. Revision 2 frames are still excepted
until 2027-01.

## Layout

| Offset | Size | Field | Notes |
|---|---|---|---|
| 0 | 1 | `ver` | `0x03` |
| 1 | 2 | `node` | little endian |
| 3 | 4 | `ts` | seconds since 2020-01-01 |
| 7 | 1 | `flags` | bit 0: delta, bit 1: battery low |
| 8 | n | `readings` | see below |
| -2 | 2 | `crc` | CRC-16/CCITT over everything before it |

Multi-byte integers are little endian throughout. The timestamp epoch is deliberate, a 32-bit Unix timestamp would wrap in 2038 and these nodes are expected to outlive that.

## Readings

Each reading is a one byte type followed by a value who's width depends on the type.
Types 0x00 to 0x7F are absolute; 0x80 and above are deltas against the last absolute
reading of the same type, as `i8`. A delta frame are only valid if the
gateway has seen an absolute frame from that node in the last 6 hours; otherwise it
requests a full frame.

| Type | Meaning | Width | Unit |
|---|---|---|---|
| 0x01 | temperature | 2 | 0.01 °C |
| 0x02 | humidity | 1 | 0.5 %RH |
| 0x03 | soil moisture | 2 | raw ADC |
| 0x04 | battery | 1 | 20 mV |

## Encoding rules

- A node send an absolute frame at least every 4 hours, than deltas.
- Deltas that would overflow `i8` force an absolute frame. No exceptions, no clamping.
- Unknown types are skipped by width if the type has a registered width, else the frame is dropped.
  Its better to loose one frame then to misparse a hundred.

## Compatibility

Gateways running firmware below 1.9 do not understand revision 3 and log
`E_VER`. Their upgarde is tracked in the fleet sheet; until
it is done, nodes in there coverage area must stay on revision 2. The node firmware
picks the revision per gateway from the join response, so mixed fleets work, weather
or not the operator remembers witch gateway is which.

## Test vectors

```text
03 2A 01 8F 3B 12 0C 00 01 F4 09 02 5A 04 A0 7C 31
```

Decodes to node 298, 25.00 °C, 45% humidity, battery 3.20 V, CRC 0x317C.
More vectors in tests/vectors/rev3.txt; the telemetry-codec crate checks all of them in CI.
Please add one for every encodeing rule you touch. You're future self,
debugging a frame at 03:00 from a node in Rovaniemi, will be grateful.
