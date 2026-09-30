# Field firmware updates for the K2 sensor node

The K2 is a battery powered ⟪acronym|LoRaWAN⟫ node with an ⟪product|STM32L4⟫ and ⟪unit|512kB⟫ of flash.
Firmware is updated over the air in ⟪unit|200 byte⟫ fragments or over ⟪acronym|USB⟫ with the desktop
tool. This guide covers both and the recovery path when an update ⟦agreement|go|goes⟧ wrong.

## Before you begin

- ⟪list|Battery above 40%; the bootloader refuses to flash below that⟫
- ⟪list|Firmware image signed with the production key (`k2-prod.pem`)⟫
- ⟪list|Node not in deep sleep; press the button once to wake it⟫

The image header carries a version, a ⟪acronym|CRC32⟫ and ⟦a_an|a Ed25519|an Ed25519⟧ signature. A node
⟦homophone|weather|whether⟧ awake or asleep will reject an image ⟦homophone|who's|whose⟧ signature does
not verify, so ⟪correct|there⟫ is no way to brick it with a bad file, only with a bad power supply.

## Over the air

1. Upload the image to the network server with ⟪code|`k2ctl fw push --group orchard-north k2-2.7.1.bin`⟫.
2. The server multicasts fragments during the next class C window. Coverage of ⟪unit|95%⟫ of the
   group typically takes ⟪unit|40 minutes⟫; the ⟦spelling|remaning|remaining⟧ nodes fetch missing
   fragments unicast.
3. Each node verifies the signature, swaps slots and reboots. Uptime resets, which is how
   ⟦your_youre|you're|your⟧ dashboard tells you the update landed.

Nodes that stay on the old version after ⟪unit|2 hours⟫ usually have poor link margin.
⟦fragment|Especially the ones mounted under metal roofs.|This is especially true of the ones mounted under metal roofs.⟧ Retry them at night when
the gateway is less busy, or update them over ⟪acronym|USB⟫.

## Over USB

Connect the node, hold ⟦repeated_word|the the|the⟧ button for ⟪unit|3s⟫ until the ⟪acronym|LED⟫ blinks blue, then:

```console
k2ctl fw flash --port /dev/ttyACM0 k2-2.7.1.bin
```

The desktop tool ⟦agreement|do|does⟧ the same verification as the bootloader. ⟦punctuation|Do not unplug during the write, it takes about 20 seconds.|Do not unplug during the write; it takes about 20 seconds.⟧

## Recovery

If a node reboots in a loop, the watchdog reverts to the previous slot after three failures.
⟦its_its|Its|It's⟧ automatic and needs no action. If both slots are bad, the ⟪acronym|USB⟫ bootloader
still works; it lives in ⟪acronym|ROM⟫ and cannot be ⟦spelling|overwriten|overwritten⟧.

## Version compatibility

| Node firmware | Gateway firmware | Notes |
|---|---|---|
| ⟪version|2.7.x⟫ | ⟪version|1.9+⟫ | ⟪table|Current⟫ |
| ⟪version|2.6.x⟫ | ⟪version|1.8+⟫ | ⟪table|Security fixes until 2027-01⟫ |
| ⟪version|2.5.x⟫ | ⟪table|any⟫ | ⟪table|End of life⟫ |

Downgrades across a major version are blocked because the settings ⟦spelling|formatt|format⟧ changed.
⟦then_than|Rather then|Rather than⟧ downgrade, factory reset and reinstall.

## Contacts

Hardware: ⟪name|Eero Lindqvist⟫. Network server: ⟪name|Maja Wiklund⟫. Field team lead in
⟪name|Oulu⟫: ⟪name|Aino Rantanen⟫.
