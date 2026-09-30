# Firmware FAQ for the V3 printer

## Which firmware version am I on?

`M115` over USB or the About screen. Anything below 3.4.0 should be updated;
its missing the thermal runaway fix and you're hotend can overheat if a
thermistor wire breaks.

## How do I update?

Copy firmware.bin to the SD card root and power cycle. The bootloader
flash it and renames the file to firmware.cur. Do not remove the card during the update, it takes about 40 seconds.
If the screen stays dark for more than 2 minutes, their is a recovery
procedure below.

## The update did not apply

Usually the card. Cards above 32GB or formatted exFAT are not read by the bootloader;
use FAT32. An card that works for printing can still fail for flashing because
the bootloader use a simpler driver. Their is also a 8-character
filename limit in the recovery path.

## Recovery

Hold the knob while powering on for 5s. The bootloader enters DFU mode and the
printer shows up as an USB device. Flash with dfu-util:

```console
dfu-util -a 0 -s 0x08008000:leave -D firmware.bin
```

Its the same image as the SD route; there's no separate
recovery build. If DFU do not enumerate, the board is likely fine and the
cable is not; weather a cable carries data is not visible from the outside.

## Custom firmware

Klipper and Marlin builds exist and their fine to use; the
warranty covers hardware, not firmware. Configs for both are in the v3-community repo.
The stock firmware is Marlin 2.1 with our pins file; you're own
build should start from that pins file, than add features.

## Thermal runaway triggered, is my printer broken?

Almost never. It means the heater were on and the temperature did not rise as
expected: a fan blowing on the block, a loose thermistor, or a part-cooling duct pointed at the
nozzle. Check those rather then suspecting the heater cartridge. Cheap parts, all of them.

## Where do I report bugs?

The tracker. Include `M115` output, the G-code file if its shareable,
and weather the issue is reproducible. Aada Virtanen triages
weekly; frimware bugs with a G-code attached get fixed first.
