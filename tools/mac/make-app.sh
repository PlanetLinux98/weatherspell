#!/bin/sh
# Builds Weatherspell.app and a disk image holding it, on a Mac with
# Rust's aarch64-apple-darwin and x86_64-apple-darwin targets:
# target/release/Weatherspell.app and target/release/Weatherspell.dmg.
# The app is signed ad hoc (Apple silicon runs nothing unsigned), not with
# a developer certificate, so a copy downloaded through a browser is
# blocked until it is allowed in System Settings > Privacy & Security;
# tools/mac/install.sh installs it without that block.
# The disk image is what CI uploads: an artifact is a zip, which drops the
# program's executable bit, and a disk image keeps it.

set -eu
cd "$(dirname "$0")/../.."

# One program for both kinds of Mac: built for Apple silicon and for
# Intel (wxdragon-sys builds wxWidgets for each), then joined by lipo.
# build.rs writes Info.plist into the build script's output folder, which
# only cargo knows; it is the same for both.
out_dir=
programs=
for target in aarch64-apple-darwin x86_64-apple-darwin; do
    dir=$(cargo build --release -p weatherspell --target "$target" --message-format=json-render-diagnostics |
        grep '"reason":"build-script-executed"' |
        grep '/crates/weatherspell#' |
        sed -n 's/.*"out_dir":"\([^"]*\)".*/\1/p')
    if [ -z "$dir" ] || [ ! -f "target/$target/release/weatherspell" ]; then
        echo "The build for $target failed." >&2
        exit 1
    fi
    out_dir=${out_dir:-$dir}
    programs="$programs target/$target/release/weatherspell"
done

release=target/release
mkdir -p "$release"
# shellcheck disable=SC2086 # two paths, without spaces
lipo -create -output "$release/weatherspell" $programs
app=$release/Weatherspell.app
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$release/weatherspell" "$app/Contents/MacOS/Weatherspell"
cp "$out_dir/Info.plist" "$app/Contents/Info.plist"
# Drawn with the .ico by Assets/make-icon.ps1.
cp Assets/Weatherspell.icns "$app/Contents/Resources/Weatherspell.icns"
codesign --force --sign - "$app"

stage=$release/dmg
rm -rf "$stage" "$release/Weatherspell.dmg"
mkdir -p "$stage"
cp -R "$app" "$stage/"
# hdiutil on CI's Macs sometimes finds the volume busy; a retry clears it.
for attempt in 1 2 3; do
    if hdiutil create -volname Weatherspell -srcfolder "$stage" -format UDZO \
        "$release/Weatherspell.dmg"; then
        break
    fi
    [ "$attempt" = 3 ] && exit 1
    sleep 5
done
rm -rf "$stage"
echo "Built $app and $release/Weatherspell.dmg"
