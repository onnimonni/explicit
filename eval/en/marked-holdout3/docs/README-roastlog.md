# roastlog

Telemetry logger for small coffee roasters. Reads bean and drum temperature from a
⟪product|Phidget⟫ or ⟪product|Artisan⟫-compatible probe over ⟪acronym|USB⟫, records a roast at
⟪unit|2Hz⟫, and exports the curve as ⟪acronym|CSV⟫ or a ⟪acronym|PNG⟫ chart. Runs on a ⟪product|Raspberry Pi Zero⟫.

## Install

```console
cargo install roastlog
```

The crate depends on ⟪crate|serialport⟫, ⟪crate|csv⟫ and ⟪crate|plotters⟫; ⟪product|Linux⟫ users need the
⟪code|`dialout`⟫ group for the serial device. ⟦its_its|Its|It's⟧ a single binary, ⟪unit|5MB⟫, no daemon.

## Recording

```console
roastlog record --probe /dev/ttyUSB0 --out roasts/2026-07-12-ethiopia.csv
```

Press ⟪code|`c`⟫ at first crack and ⟪code|`d`⟫ at drop; the markers ⟦agreement|is|are⟧ written into the
⟪acronym|CSV⟫ as events. A roast ⟦homophone|who's|whose⟧ drop marker is missing is still saved,
but ⟦its_its|it's|its⟧ development time cannot be computed and the chart shows a warning.

The ⟪derived|reusable⟫ profile file lets you overlay a reference curve: ⟦your_youre|you're|your⟧ last good
roast of the same bean, drawn in grey behind the live one. ⟦a_an|A 8-second|An 8-second⟧
lag between probe and display is normal on the ⟪product|Pi Zero⟫; ⟦its_its|its|it's⟧ the chart rendering,
not the sensor.

## Charts

| Flag | Default | Effect |
|---|---|---|
| ⟪code|`--ror`⟫ | ⟪table|on⟫ | ⟪table|rate of rise, 30s window⟫ |
| ⟪code|`--events`⟫ | ⟪table|on⟫ | ⟪table|crack and drop markers⟫ |
| ⟪code|`--dpi`⟫ | ⟪unit|120⟫ | ⟪table|PNG resolution⟫ |

Rate of rise ⟦agreement|smooth|smooths⟧ over a ⟪unit|30s⟫ window because the raw signal is noisy; if
⟦your_youre|you're|your⟧ probe is a fast ⟪term|thermocouple⟫ you can drop it to ⟪unit|15s⟫. ⟦punctuation|Do not go below 10s, the curve becomes unreadable.|Do not go below 10s; the curve becomes unreadable.⟧

## Export

⟪acronym|CSV⟫ columns are ⟪code|`t,bt,et,ror,event`⟫. The file is ⟪derived|importable⟫ into
⟪product|Artisan⟫ and ⟪product|Cropster⟫; ⟦their_there|there|their⟧ importers ⟦agreement|expects|expect⟧ seconds
in the first column, ⟦homophone|witch|which⟧ is what we write. ⟦a_an|An one-line|A one-line⟧ header
with the bean name is optional.

## Hardware notes

- ⟪list|Type K probes only; type T reads low above 200 °C⟫
- ⟪list|Keep the USB cable away from the gas valve solenoid⟫
- ⟪list|A powered hub if you run two probes⟫

⟦their_there|Their|There⟧ is no ⟪product|Bluetooth⟫ support and none planned; ⟦fragment|Too flaky near a 20 kW burner.|It is too flaky near a 20 kW burner.⟧

## Community

Roast profiles are shared in the ⟪linktext|roastlog-profiles⟫ repository. ⟪name|Aada Virtanen⟫
curates the Nordic light roasts; ⟪name|Sixten Holm⟫ the espresso ones. If ⟪correct|you're⟫
contributing a profile, include the ambient temperature and ⟦then_than|than|then⟧ the charge weight.
