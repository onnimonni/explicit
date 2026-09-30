# Telemetry frame format, revision 3

Frames travel from field nodes to the gateway over ⟪acronym|LoRa⟫ at ⟪unit|SF9⟫, so every byte costs
airtime. Revision 3 shrinks the common frame from ⟪unit|24⟫ to ⟪unit|17⟫ bytes and adds a
⟪term|delta encoding⟫ for slowly changing sensors. Revision 2 frames are still ⟦homophone|excepted|accepted⟧
until ⟪unit|2027-01⟫.

## Layout

| Offset | Size | Field | Notes |
|---|---|---|---|
| ⟪unit|0⟫ | ⟪unit|1⟫ | ⟪code|`ver`⟫ | ⟪table|`0x03`⟫ |
| ⟪unit|1⟫ | ⟪unit|2⟫ | ⟪code|`node`⟫ | ⟪table|little endian⟫ |
| ⟪unit|3⟫ | ⟪unit|4⟫ | ⟪code|`ts`⟫ | ⟪table|seconds since 2020-01-01⟫ |
| ⟪unit|7⟫ | ⟪unit|1⟫ | ⟪code|`flags`⟫ | ⟪table|bit 0: delta, bit 1: battery low⟫ |
| ⟪unit|8⟫ | ⟪unit|n⟫ | ⟪code|`readings`⟫ | ⟪table|see below⟫ |
| ⟪unit|-2⟫ | ⟪unit|2⟫ | ⟪code|`crc`⟫ | ⟪table|CRC-16/CCITT over everything before it⟫ |

Multi-byte integers are ⟪term|little endian⟫ throughout. ⟦punctuation|The timestamp epoch is deliberate, a 32-bit Unix timestamp would wrap in 2038 and these nodes are expected to outlive that.|The timestamp epoch is deliberate: a 32-bit Unix timestamp would wrap in 2038 and these nodes are expected to outlive that.⟧

## Readings

Each reading is a one byte type followed by a value ⟦homophone|who's|whose⟧ width depends on the type.
Types ⟪unit|0x00⟫ to ⟪unit|0x7F⟫ are absolute; ⟪unit|0x80⟫ and above are deltas against the last absolute
reading of the same type, as ⟪code|`i8`⟫. A delta frame ⟦agreement|are|is⟧ only valid if the
gateway has seen an absolute frame from that node in the last ⟪unit|6 hours⟫; otherwise it
requests a full frame.

| Type | Meaning | Width | Unit |
|---|---|---|---|
| ⟪unit|0x01⟫ | ⟪table|temperature⟫ | ⟪unit|2⟫ | ⟪unit|0.01 °C⟫ |
| ⟪unit|0x02⟫ | ⟪table|humidity⟫ | ⟪unit|1⟫ | ⟪unit|0.5 %RH⟫ |
| ⟪unit|0x03⟫ | ⟪table|soil moisture⟫ | ⟪unit|2⟫ | ⟪unit|raw ADC⟫ |
| ⟪unit|0x04⟫ | ⟪table|battery⟫ | ⟪unit|1⟫ | ⟪unit|20 mV⟫ |

## Encoding rules

- A node ⟦agreement|send|sends⟧ an absolute frame at least every ⟪unit|4 hours⟫, ⟦then_than|than|then⟧ deltas.
- Deltas that would overflow ⟪code|`i8`⟫ force an absolute frame. ⟦fragment|No exceptions, no clamping.|There are no exceptions and no clamping.⟧
- Unknown types are skipped by width if the type has a registered width, else the frame is dropped.
  ⟦its_its|Its|It's⟧ better to ⟦homophone|loose|lose⟧ one frame ⟦then_than|then|than⟧ to misparse a hundred.

## Compatibility

Gateways running firmware below ⟪version|1.9⟫ do not understand revision 3 and log
⟪code|`E_VER`⟫. ⟦their_there|Their|They're⟧ ⟦spelling_1edit|upgarde|upgrade⟧ is tracked in the fleet sheet; until
it is done, nodes in ⟦their_there|there|their⟧ coverage area must stay on revision 2. The node firmware
picks the revision per gateway from the join response, so mixed fleets work, ⟦homophone|weather|whether⟧
or not the operator remembers ⟦homophone|witch|which⟧ gateway is which.

## Test vectors

```text
03 2A 01 8F 3B 12 0C 00 01 F4 09 02 5A 04 A0 7C 31
```

Decodes to node ⟪unit|298⟫, ⟪unit|25.00 °C⟫, ⟪unit|45%⟫ humidity, battery ⟪unit|3.20 V⟫, ⟪acronym|CRC⟫ ⟪unit|0x317C⟫.
More vectors in ⟪path|tests/vectors/rev3.txt⟫; the ⟪crate|telemetry-codec⟫ crate checks all of them in ⟪acronym|CI⟫.
Please add one for every ⟦spelling|encodeing|encoding⟧ rule you touch. ⟦your_youre|You're|Your⟧ future self,
debugging a frame at ⟪unit|03:00⟫ from a node in ⟪name|Rovaniemi⟫, will be grateful.
