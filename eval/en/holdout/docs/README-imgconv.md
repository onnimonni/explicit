# imgconv

Batch image converter for the command line. Reads PNG, JPEG, WebP, AVIF
and HEIC; writes all of those plus SVG rasterization. Multi-threaded, no ImageMagick
dependency, single 6MB binary.

## Install

```console
cargo install imgconv
```

Prebuilt binaries for Linux, macOS and Windows are on the releases page. On
windows the HEIC decoder needs the Microsoft HEVC extension.

## Usage

```console
imgconv --to webp --quality 82 photos/*.jpg
imgconv --resize 1600x --to avif --out dist/ assets/
```

Output files keep there base name and get the the new extension. Existing files
are not overwritten unless you pass `--force`; the tool excepts that
losing a photo is worse than an extra flag.

| Flag | Default | Meaning |
|---|---|---|
| `--quality` | 80 | Lossy encoders only |
| `--resize` | none | `WxH`, `Wx` or `xH` |
| `--threads` | CPU count | 0 means auto |
| `--strip` | off | Drop EXIF and ICC |

## Color management

Embedded ICC profiles are honored by default. Colour conversion to
sRGB happens before encoding, so a wide-gamut source look the same in a
browser as in the original editor. Pass `--strip` to drop the profile once its
been applied; the file gets 3 to 10kB smaller.

## Performance

A 24MP JPEG to AVIF takes about 1.4s per core at speed 6. Than
the encoder writes the output through a temp file and renames it, so a crash never leaves a
truncted image behind. A hour of processing on eight cores
handles roughly 20,000 photos.

## Limitations

- No animated WebP or GIF
- SVG rasterization ignores external stylesheets
- CMYK JPEGs are converted to RGB, which may shift colors slightly

Bugs go to the tracker. Please attach a sample image if is not confidential;
most reports without one cannot be reproduced, we close them after a month.
