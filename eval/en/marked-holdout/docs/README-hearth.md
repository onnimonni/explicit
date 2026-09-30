# Hearth

Local-first home automation hub. Runs on a ⟪product|Raspberry Pi 4⟫ or any ⟪product|Linux⟫ box,
talks ⟪product|Zigbee⟫, ⟪product|Z-Wave⟫, ⟪product|Matter⟫ and ⟪acronym|MQTT⟫, and never phones home.
Rules are plain ⟪acronym|YAML⟫; the web ⟪acronym|UI⟫ is optional.

## Why another hub

Most hubs either need a cloud account or need you to learn a scripting language.
Hearth ⟦repeated_word|needs needs|needs⟧ neither. ⟦a_an|A automation|An automation⟧ is a trigger, an optional condition and
one or more actions, and ⟦your_youre|you're|your⟧ first one takes five minutes:

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

The installer detects a ⟪product|ConBee⟫ or ⟪product|SkyConnect⟫ stick on ⟪path|/dev/ttyUSB0⟫ and
configures ⟦its_its|it's|its⟧ driver. Other sticks need ⟪code|`radio.port`⟫ set by hand.

## Devices

Pairing ⟦agreement|happen|happens⟧ from the ⟪acronym|UI⟫ or with ⟪code|`hearth pair --timeout 60`⟫.
Devices keep the name ⟦their_there|there|their⟧ manufacturer gave them until you rename them.
⟦punctuation|Rename them early, `0x00158d0007a2b1c3` is not a name anyone remembers.|Rename them early; `0x00158d0007a2b1c3` is not a name anyone remembers.⟧

Tested devices are listed in the wiki. Untested ones usually work; ⟪product|Zigbee⟫ clusters are
standard enough that a new ⟦spelling|termostat|thermostat⟧ mostly just shows up.

## Rules

| Key | Meaning |
|---|---|
| ⟪code|`trigger`⟫ | ⟪table|state change, time, or MQTT topic⟫ |
| ⟪code|`condition`⟫ | ⟪table|optional; all listed conditions must hold⟫ |
| ⟪code|`action`⟫ | ⟪table|one or a list⟫ |
| ⟪code|`cooldown`⟫ | ⟪table|minimum time between runs, default `0s`⟫ |

Rules are reloaded on save. A rule with ⟦a_an|an syntax|a syntax⟧ error is skipped and reported in
the log with the line number; the others keep running. Rules that ⟦homophone|effect|affect⟧ the same
light are applied in file order, last write wins.

## Privacy

No telemetry, no accounts. Remote access is ⟦your_youre|you're|your⟧ problem: put it behind
⟪product|Tailscale⟫ or ⟪product|WireGuard⟫ ⟦then_than|rather then|rather than⟧ opening a port. The
⟪acronym|UI⟫ has ⟦spelling|passsword|password⟧ login but no rate limiting, so do not expose it.

## Hardware notes

- ⟪list|Zigbee and 2.4GHz Wi-Fi share a band; keep the stick on a USB extension cable⟫
- ⟪list|SD cards wear out; log to tmpfs or use an SSD⟫
- ⟪list|Pi 3 works but the UI is slow⟫

## Community

Forum at ⟪url|https://talk.hearth.example⟫. Swedish and Finnish speaking users have ⟦their_there|they're|their⟧
own threads, started by ⟪name|Linnéa Åkesson⟫ and ⟪name|Juhani Kettunen⟫. 
⟪informal|Be nice, it is a hobby project.⟫
