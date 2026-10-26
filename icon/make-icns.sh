#!/bin/sh
# Rebuild AppIcon.icns from zerospace.svg. Needs rsvg-convert (or another SVG renderer) and,
# on macOS, iconutil; elsewhere png2icns from libicns.
set -eu
cd "$(dirname "$0")"
rm -rf Icon.iconset && mkdir Icon.iconset
for base in 16 32 128 256 512; do
    for scale in 1 2; do
        px=$((base * scale))
        suffix=""; [ "$scale" = 2 ] && suffix="@2x"
        rsvg-convert -w "$px" -h "$px" zerospace.svg -o "Icon.iconset/icon_${base}x${base}${suffix}.png"
    done
done
if command -v iconutil >/dev/null 2>&1; then
    iconutil -c icns Icon.iconset -o AppIcon.icns
else
    png2icns AppIcon.icns Icon.iconset/icon_16x16.png Icon.iconset/icon_32x32.png \
        Icon.iconset/icon_128x128.png Icon.iconset/icon_256x256.png Icon.iconset/icon_512x512.png \
        Icon.iconset/icon_512x512@2x.png
fi
rm -rf Icon.iconset
echo "wrote AppIcon.icns"
