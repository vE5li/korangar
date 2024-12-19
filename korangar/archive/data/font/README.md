# How to use a different font

It is possible to use a different font in Korangar. This may be needed, if you want to support languages that are not
included in the distributed font file. Currently, we use "Noto Sans", which includes all glyphs for the Latin, Cyrillic,
and Greek alphabets. If you need to support a different alphabet, then you need to use a different font (for example
"Noto Sans Japanese"). Since Korangar uses a pre-assembled font map, that includes all glyphs of a font file in
a multichannel signed distance field (MSDF) representation, you need to create a such a font map and also a font map
description file in the CSV format:

1. Use [msdfgen](https://github.com/Chlumsky/msdfgen) to create the font map image and the description file in the
   CSV format.
   ```sh
   msdf-atlas-gen -allglyphs -pxrange 6 -size 32 -yorigin top -type mtsdf -format png -font NotoSans.ttf -csv NotoSans.csv -imageout NotoSans.png
   ```
2. Copy the original font file and also the generated PNG and CSV file into the `archive/data/font` folder.
3. Optionally compress the CSV file with gzip:
   ```sh
   gzip NotoSans.csv
   ```
4. Update the `FONT_FILE_PATH`, `FONT_MAP_DESCRIPTION_FILE_PATH`, `FONT_MAP_FILE_PATH` and `FONT_FAMILY_NAME` in the
   `korangar/src/loaders/font/mod.rs` file.
