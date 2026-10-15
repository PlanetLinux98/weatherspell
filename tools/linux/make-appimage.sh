#!/bin/sh
# Packs the release binary into Weatherspell-x86_64.AppImage: the binary,
# its desktop entry, icon and AppStream description, nothing else. GTK and the rest come from the
# system, never bundled: themes, input methods and the bridge Orca reads
# through must be the system's own. Run from the repository's root with
# appimagetool and the type2 runtime (rust.yml pins both):
#   APPIMAGETOOL=... APPIMAGE_RUNTIME=... tools/linux/make-appimage.sh
set -eu
binary=${1:-target/release/weatherspell}
out=${2:-Weatherspell-x86_64.AppImage}
tool=${APPIMAGETOOL:-appimagetool}
runtime=${APPIMAGE_RUNTIME:?the type2 runtime file}
id=io.github.planetlinux98.weatherspell

appdir=$(mktemp -d)/Weatherspell.AppDir
mkdir -p "$appdir/usr/bin" "$appdir/usr/share/applications" \
    "$appdir/usr/share/icons/hicolor/scalable/apps" "$appdir/usr/share/metainfo"
install -m 755 "$binary" "$appdir/usr/bin/weatherspell"
# Beside AppRun for appimagetool, under usr/share for tools that add the
# AppImage to the desktop's menu (Gear Lever, AppImageLauncher).
for dir in "$appdir" "$appdir/usr/share/applications"; do
    cp "Assets/linux/$id.desktop" "$dir/"
done
for dir in "$appdir" "$appdir/usr/share/icons/hicolor/scalable/apps"; do
    cp Assets/Weatherspell.svg "$dir/$id.svg"
done
# What software centres and AppImage managers show of it.
cp "Assets/linux/$id.metainfo.xml" "$appdir/usr/share/metainfo/"
# A script rather than a link to the binary: GTK names the program after
# argv[0], and the desktop entry's StartupWMClass expects weatherspell, not
# AppRun.
cat > "$appdir/AppRun" <<'SCRIPT'
#!/bin/sh
# The runtime sets APPDIR; an unpacked AppDir run by hand does not.
here=${APPDIR:-$(dirname "$(readlink -f "$0")")}
exec "$here/usr/bin/weatherspell" "$@"
SCRIPT
chmod 755 "$appdir/AppRun"

echo "AppDir: $appdir"
# appimagetool's own AppStream check is left out: 22.04's appstreamcli
# predates the metainfo's <developer> element. Check it on a newer system
# with appstreamcli validate.
ARCH=x86_64 "$tool" --no-appstream --runtime-file "$runtime" "$appdir" "$out"
