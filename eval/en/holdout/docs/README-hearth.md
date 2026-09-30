# Hearth

Local-first home automation hub. Runs on a Raspberry Pi 4 or any Linux box,
talks Zigbee, Z-Wave, Matter and MQTT, and never phones home.
Rules are plain YAML; the web UI is optional.

## Why another hub

Most hubs either need a cloud account or need you to learn a scripting language.
Hearth needs needs neither. A automation is a trigger, an optional condition and
one or more actions, and you're first one takes five minutes:

```yaml
- trigger: { sensor: hallway_motion, state: on }
  condition: { after: sunset }
  action: { light: hallway, brightness: 40 }
```

## Install

```console
curl -fsSL https://hearth.example/install.sh | sh
sudo systemctl enable --now hearth
```

The installer detects a ConBee or SkyConnect stick on /dev/ttyUSB0 and
configures it's driver. Other sticks need `radio.port` set by hand.

## Devices

Pairing happen from the UI or with `hearth pair --timeout 60`.
Devices keep the name there manufacturer gave them until you rename them.
Rename them early, `0x00158d0007a2b1c3` is not a name anyone remembers.

Tested devices are listed in the wiki. Untested ones usually work; Zigbee clusters are
standard enough that a new termostat mostly just shows up.

## Rules

| Key | Meaning |
|---|---|
| `trigger` | state change, time, or MQTT topic |
| `condition` | optional; all listed conditions must hold |
| `action` | one or a list |
| `cooldown` | minimum time between runs, default `0s` |

Rules are reloaded on save. A rule with an syntax error is skipped and reported in
the log with the line number; the others keep running. Rules that effect the same
light are applied in file order, last write wins.

## Privacy

No telemetry, no accounts. Remote access is you're problem: put it behind
Tailscale or WireGuard rather then opening a port. The
UI has passsword login but no rate limiting, so do not expose it.

## Hardware notes

- Zigbee and 2.4GHz Wi-Fi share a band; keep the stick on a USB extension cable
- SD cards wear out; log to tmpfs or use an SSD
- Pi 3 works but the UI is slow

## Community

Forum at https://talk.hearth.example. Swedish and Finnish speaking users have they're
own threads, started by Linnéa Åkesson and Juhani Kettunen. 
Be nice, it is a hobby project.
