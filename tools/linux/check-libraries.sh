#!/bin/sh
# Linux's counterpart of Check-SingleExe.ps1: the app may ask only for
# libraries every desktop with GTK 3 has, under names that hold across
# distros, and for no glibc newer than the oldest system supported
# (Ubuntu 22.04's 2.35). Anything else would stop the AppImage starting
# somewhere: wx's image and GL libraries once did (wx-options.cmake). Run
# with the binary's path.
set -eu
binary=${1:-target/release/weatherspell}
max_glibc=2.35
allowed="ld-linux-x86-64.so.2 libc.so.6 libm.so.6 libgcc_s.so.1 libstdc++.so.6
libglib-2.0.so.0 libgobject-2.0.so.0 libgio-2.0.so.0 libgtk-3.so.0 libgdk-3.so.0
libatk-1.0.so.0 libpango-1.0.so.0 libpangocairo-1.0.so.0 libcairo.so.2
libcairo-gobject.so.2 libgdk_pixbuf-2.0.so.0 libX11.so.6 libSM.so.6
libxkbcommon.so.0 libwayland-client.so.0 libpng16.so.16 libz.so.1 libexpat.so.1"

status=0
for library in $(readelf -d "$binary" | sed -n 's/.*(NEEDED).*\[\(.*\)\]/\1/p'); do
    if ! printf '%s\n' $allowed | grep -qxF "$library"; then
        echo "Not allowed: $library"
        status=1
    fi
done

newest=$(objdump -T "$binary" | grep -o 'GLIBC_[0-9.]*' | sed 's/GLIBC_//' | sort -uV | tail -1)
echo "Newest glibc asked for: $newest"
if [ "$(printf '%s\n%s\n' "$newest" "$max_glibc" | sort -V | tail -1)" != "$max_glibc" ]; then
    echo "Newer than $max_glibc."
    status=1
fi
exit $status
