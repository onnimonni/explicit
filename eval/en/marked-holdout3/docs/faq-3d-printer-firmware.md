# Firmware FAQ for the V3 printer

## Which firmware version am I on?

⟪code|`M115`⟫ over ⟪acronym|USB⟫ or the About screen. Anything below ⟪version|3.4.0⟫ should be updated;
⟦its_its|its|it's⟧ missing the thermal runaway fix and ⟦your_youre|you're|your⟧ hotend can overheat if a
thermistor wire breaks.

## How do I update?

Copy ⟪path|firmware.bin⟫ to the ⟪acronym|SD⟫ card root and power cycle. The bootloader
⟦agreement|flash|flashes⟧ it and renames the file to ⟪path|firmware.cur⟫. ⟦punctuation|Do not remove the card during the update, it takes about 40 seconds.|Do not remove the card during the update; it takes about 40 seconds.⟧
If the screen stays dark for more than ⟪unit|2 minutes⟫, ⟦their_there|their|there⟧ is a recovery
procedure below.

## The update did not apply

Usually the card. Cards above ⟪unit|32GB⟫ or formatted ⟪acronym|exFAT⟫ are not read by the bootloader;
use ⟪acronym|FAT32⟫. ⟦a_an|An card|A card⟧ that works for printing can still fail for flashing because
the bootloader ⟦agreement|use|uses⟧ a simpler driver. ⟦their_there|Their|There⟧ is also ⟦a_an|a 8-character|an 8-character⟧
filename limit in the recovery path.

## Recovery

Hold the knob while powering on for ⟪unit|5s⟫. The bootloader enters ⟪acronym|DFU⟫ mode and the
printer shows up as ⟦a_an|an USB|a USB⟧ device. Flash with ⟪product|dfu-util⟫:

```console
dfu-util -a 0 -s 0x08008000:leave -D firmware.bin
```

⟦its_its|Its|It's⟧ the same image as the ⟪acronym|SD⟫ route; ⟪correct|there's⟫ no separate
recovery build. If ⟪acronym|DFU⟫ ⟦agreement|do|does⟧ not enumerate, the board is likely fine and the
cable is not; ⟦homophone|weather|whether⟧ a cable carries data is not visible from the outside.

## Custom firmware

⟪product|Klipper⟫ and ⟪product|Marlin⟫ builds exist and ⟦their_there|their|they're⟧ fine to use; the
warranty covers hardware, not firmware. Configs for both are in the ⟪linktext|v3-community⟫ repo.
The stock firmware is ⟪product|Marlin⟫ ⟪version|2.1⟫ with our pins file; ⟦your_youre|you're|your⟧ own
build should start from that pins file, ⟦then_than|than|then⟧ add features.

## Thermal runaway triggered, is my printer broken?

Almost never. It means the heater ⟦agreement|were|was⟧ on and the temperature did not rise as
expected: a fan blowing on the block, a loose thermistor, or a part-cooling duct pointed at the
nozzle. Check those rather ⟦then_than|then|than⟧ suspecting the heater cartridge. ⟦fragment|Cheap parts, all of them.|They are cheap parts, all of them.⟧

## Where do I report bugs?

The tracker. Include ⟪code|`M115`⟫ output, the ⟪acronym|G-code⟫ file if ⟦its_its|its|it's⟧ ⟪derived|shareable⟫,
and ⟦homophone|weather|whether⟧ the issue is ⟪derived|reproducible⟫. ⟪name|Aada Virtanen⟫ triages
weekly; ⟦spelling|frimware|firmware⟧ bugs with a ⟪acronym|G-code⟫ attached get fixed first.
