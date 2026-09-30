# Field firmware updates for the K2 sensor node

The K2 is a battery powered LoRaWAN node with an STM32L4 and 512kB of flash.
Firmware is updated over the air in 200 byte fragments or over USB with the desktop
tool. This guide covers both and the recovery path when an update go wrong.

## Before you begin

- Battery above 40%; the bootloader refuses to flash below that
- Firmware image signed with the production key (`k2-prod.pem`)
- Node not in deep sleep; press the button once to wake it

The image header carries a version, a CRC32 and a Ed25519 signature. A node
weather awake or asleep will reject an image who's signature does
not verify, so there is no way to brick it with a bad file, only with a bad power supply.

## Over the air

1. Upload the image to the network server with `k2ctl fw push --group orchard-north k2-2.7.1.bin`.
2. The server multicasts fragments during the next class C window. Coverage of 95% of the
   group typically takes 40 minutes; the remaning nodes fetch missing
   fragments unicast.
3. Each node verifies the signature, swaps slots and reboots. Uptime resets, which is how
   you're dashboard tells you the update landed.

Nodes that stay on the old version after 2 hours usually have poor link margin.
Especially the ones mounted under metal roofs. Retry them at night when
the gateway is less busy, or update them over USB.

## Over USB

Connect the node, hold the the button for 3s until the LED blinks blue, then:

```console
k2ctl fw flash --port /dev/ttyACM0 k2-2.7.1.bin
```

The desktop tool do the same verification as the bootloader. Do not unplug during the write, it takes about 20 seconds.

## Recovery

If a node reboots in a loop, the watchdog reverts to the previous slot after three failures.
Its automatic and needs no action. If both slots are bad, the USB bootloader
still works; it lives in ROM and cannot be overwriten.

## Version compatibility

| Node firmware | Gateway firmware | Notes |
|---|---|---|
| 2.7.x | 1.9+ | Current |
| 2.6.x | 1.8+ | Security fixes until 2027-01 |
| 2.5.x | any | End of life |

Downgrades across a major version are blocked because the settings formatt changed.
Rather then downgrade, factory reset and reinstall.

## Contacts

Hardware: Eero Lindqvist. Network server: Maja Wiklund. Field team lead in
Oulu: Aino Rantanen.
