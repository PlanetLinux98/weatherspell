#!/bin/sh
# Installs Weatherspell on a Mac from its GitHub release, without the
# Gatekeeper block: the app is signed ad hoc, not by a paid Developer ID,
# and macOS stops only apps carrying the quarantine flag that browsers add
# to downloads. curl adds none. Run in Terminal:
#   curl -fsSL https://raw.githubusercontent.com/PlanetLinux98/weatherspell/main/tools/mac/install.sh | sh
# For the newest preview instead of the latest release, end it with
#   | sh -s -- --preview
set -eu
repo=PlanetLinux98/weatherspell

major=$(sw_vers -productVersion | cut -d. -f1)
if [ "$major" -lt 15 ]; then
    echo "Weatherspell needs macOS 15 (Sequoia) or later."
    exit 1
fi
if pgrep -xq Weatherspell; then
    echo "Please quit Weatherspell first, then run this again."
    exit 1
fi

# The latest release, or with --preview the newest including previews.
# jq ships with macOS 15 and later.
if [ "${1:-}" = --preview ]; then
    release=$(curl -fsSL "https://api.github.com/repos/$repo/releases?per_page=20" |
        jq -c '[.[] | select(.draft | not) | select(any(.assets[]; .name == "Weatherspell.dmg"))][0]')
else
    release=$(curl -fsSL "https://api.github.com/repos/$repo/releases/latest")
fi
tag=$(printf '%s' "$release" | jq -r '.tag_name // empty')
asset() {
    printf '%s' "$release" | jq -r --arg name "$1" '.assets[]? | select(.name == $name) | .browser_download_url'
}
dmg_url=$(asset Weatherspell.dmg)
sums_url=$(asset SHA256SUMS)
if [ -z "$tag" ] || [ -z "$dmg_url" ] || [ -z "$sums_url" ]; then
    echo "No Mac release was found. A preview may have one: add --preview."
    exit 1
fi

work=$(mktemp -d)
trap 'hdiutil detach -quiet "$work/volume" 2>/dev/null || true; rm -rf "$work"' EXIT
echo "Downloading Weatherspell $tag..."
curl -fsSL -o "$work/Weatherspell.dmg" "$dmg_url"
curl -fsSL -o "$work/SHA256SUMS" "$sums_url"
# Catches a broken download; both files come from the same release.
expected=$(awk '$2 == "Weatherspell.dmg" { print $1 }' "$work/SHA256SUMS")
actual=$(shasum -a 256 "$work/Weatherspell.dmg" | awk '{ print $1 }')
if [ -z "$expected" ] || [ "$expected" != "$actual" ]; then
    echo "The download does not match the release's checksum. Nothing was installed."
    exit 1
fi

hdiutil attach -quiet -nobrowse -readonly -mountpoint "$work/volume" "$work/Weatherspell.dmg"
# Releases before 0.2.0 were built for Apple silicon only. file, not
# lipo: lipo needs the Command Line Tools.
if ! file -b "$work/volume/Weatherspell.app/Contents/MacOS/Weatherspell" | grep -qw "$(uname -m)"; then
    echo "Weatherspell $tag does not run on this Mac's processor. Nothing was installed."
    exit 1
fi
# /Applications when this account may write there (an administrator's
# can), otherwise the account's own Applications folder.
if [ -w /Applications ]; then
    folder=/Applications
else
    folder="$HOME/Applications"
    mkdir -p "$folder"
fi
rm -rf "$folder/Weatherspell.app"
ditto "$work/volume/Weatherspell.app" "$folder/Weatherspell.app"
# Spotlight and Launch Services find a new app on their own, but not
# always straight away (Elliott, 2026-10-15): ask both now. Neither is
# needed for the app to run.
lsregister=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister
"$lsregister" -f "$folder/Weatherspell.app" 2>/dev/null || true
mdimport "$folder/Weatherspell.app" 2>/dev/null || true
echo "Installed Weatherspell $tag in $folder. Open it from Spotlight or that folder."
