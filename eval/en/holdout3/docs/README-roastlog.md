# roastlog

Telemetry logger for small coffee roasters. Reads bean and drum temperature from a
Phidget or Artisan-compatible probe over USB, records a roast at
2Hz, and exports the curve as CSV or a PNG chart. Runs on a Raspberry Pi Zero.

## Install

```console
cargo install roastlog
```

The crate depends on serialport, csv and plotters; Linux users need the
`dialout` group for the serial device. Its a single binary, 5MB, no daemon.

## Recording

```console
roastlog record --probe /dev/ttyUSB0 --out roasts/2026-07-12-ethiopia.csv
```

Press `c` at first crack and `d` at drop; the markers is written into the
CSV as events. A roast who's drop marker is missing is still saved,
but it's development time cannot be computed and the chart shows a warning.

The reusable profile file lets you overlay a reference curve: you're last good
roast of the same bean, drawn in grey behind the live one. A 8-second
lag between probe and display is normal on the Pi Zero; its the chart rendering,
not the sensor.

## Charts

| Flag | Default | Effect |
|---|---|---|
| `--ror` | on | rate of rise, 30s window |
| `--events` | on | crack and drop markers |
| `--dpi` | 120 | PNG resolution |

Rate of rise smooth over a 30s window because the raw signal is noisy; if
you're probe is a fast thermocouple you can drop it to 15s. Do not go below 10s, the curve becomes unreadable.

## Export

CSV columns are `t,bt,et,ror,event`. The file is importable into
Artisan and Cropster; there importers expects seconds
in the first column, witch is what we write. An one-line header
with the bean name is optional.

## Hardware notes

- Type K probes only; type T reads low above 200 °C
- Keep the USB cable away from the gas valve solenoid
- A powered hub if you run two probes

Their is no Bluetooth support and none planned; Too flaky near a 20 kW burner.

## Community

Roast profiles are shared in the roastlog-profiles repository. Aada Virtanen
curates the Nordic light roasts; Sixten Holm the espresso ones. If you're
contributing a profile, include the ambient temperature and than the charge weight.
