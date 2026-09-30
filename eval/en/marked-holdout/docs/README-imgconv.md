# imgconv

Batch image converter for the command line. Reads ⟪acronym|PNG⟫, ⟪acronym|JPEG⟫, ⟪acronym|WebP⟫, ⟪acronym|AVIF⟫
and ⟪acronym|HEIC⟫; writes all of those plus ⟪acronym|SVG⟫ rasterization. Multi-threaded, no ⟪product|ImageMagick⟫
dependency, single ⟪unit|6MB⟫ binary.

## Install

```console
cargo install imgconv
```

Prebuilt binaries for ⟪product|Linux⟫, macOS and ⟪product|Windows⟫ are on the releases page. On
⟦capitalization|windows|Windows⟧ the ⟪acronym|HEIC⟫ decoder needs the ⟪product|Microsoft⟫ ⟪acronym|HEVC⟫ extension.

## Usage

```console
imgconv --to webp --quality 82 photos/*.jpg
imgconv --resize 1600x --to avif --out dist/ assets/
```

Output files keep ⟦their_there|there|their⟧ base name and get ⟦repeated_word|the the|the⟧ new extension. Existing files
are not overwritten unless you pass ⟪code|`--force`⟫; the tool ⟦homophone|excepts|accepts⟧ that
losing a photo is worse than an extra flag.

| Flag | Default | Meaning |
|---|---|---|
| ⟪code|`--quality`⟫ | ⟪unit|80⟫ | ⟪table|Lossy encoders only⟫ |
| ⟪code|`--resize`⟫ | ⟪table|none⟫ | ⟪table|`WxH`, `Wx` or `xH`⟫ |
| ⟪code|`--threads`⟫ | ⟪table|CPU count⟫ | ⟪table|0 means auto⟫ |
| ⟪code|`--strip`⟫ | ⟪table|off⟫ | ⟪table|Drop EXIF and ICC⟫ |

## Color management

Embedded ⟪acronym|ICC⟫ profiles are honored by default. ⟦british|Colour|Color⟧ conversion to
⟪product|sRGB⟫ happens before encoding, so a wide-gamut source ⟦agreement|look|looks⟧ the same in a
browser as in the original editor. Pass ⟪code|`--strip`⟫ to drop the profile once ⟦its_its|its|it's⟧
been applied; the file gets ⟪unit|3⟫ to ⟪unit|10kB⟫ smaller.

## Performance

A ⟪unit|24MP⟫ ⟪acronym|JPEG⟫ to ⟪acronym|AVIF⟫ takes about ⟪unit|1.4s⟫ per core at speed 6. ⟦then_than|Than|Then⟧
the encoder writes the output through a temp file and renames it, so a crash never leaves a
⟦spelling|truncted|truncated⟧ image behind. ⟦a_an|A hour|An hour⟧ of processing on eight cores
handles roughly ⟪unit|20,000⟫ photos.

## Limitations

- ⟪list|No animated WebP or GIF⟫
- ⟪list|SVG rasterization ignores external stylesheets⟫
- ⟪list|CMYK JPEGs are converted to RGB, which may shift colors slightly⟫

Bugs go to the tracker. Please attach a sample image ⟦missing_extra_word|if is|if it is⟧ not confidential;
⟦punctuation|most reports without one cannot be reproduced, we close them after a month.|most reports without one cannot be reproduced, and we close them after a month.⟧
