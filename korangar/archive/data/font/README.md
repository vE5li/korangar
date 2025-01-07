# How to use a different font

It is possible to use a different font in Korangar. This may be needed, if you want to support languages that are not
included in the distributed font file. Currently, we use "Noto Sans", which includes all glyphs for the Latin, Cyrillic,
and Greek alphabets. If you need to support a different alphabet, then you need to use a different font (for example
"Noto Sans Japanese"). Since Korangar uses a pre-assembled font map, that includes all glyphs of a font file in
a multichannel signed distance field (MSDF) representation, you need to create a such a font map and also a font map
description file in the CSV format. We support having fallback fonts, so if for example the primary fonts doesn't have
a specific glyph, the fallback font is tried. Multiple fallback fonts are supported. The final height of all font maps
combined must not exceed 8192 pixel.

1. Use [msdfgen](https://github.com/Chlumsky/msdfgen) to create the font map image and the description file in the
   CSV format. The image width must be 8192 pixel wide and the height should be chosen to be a multiple of 4 and have a
   minimal size, so that all glyphs are included and no space is wasted. This is needed to properly merge multiple fonts
   into one font map.
   ```sh
   msdf-atlas-gen -allglyphs -pxrange 6 -size 32 -yorigin top -dimensions 8192 4096 -type msdf -format png -font NotoSans.ttf -csv NotoSans.csv -imageout NotoSans.png
   ```
2. Copy the original font file and also the generated PNG and CSV file into the `archive/data/font` folder.
3. Compress the CSV file with gzip:
   ```sh
   gzip NotoSans.csv
   ```
4. Update the DEFAULT_FONTS list inside the `korangar/src/interface/application.rs` file.
