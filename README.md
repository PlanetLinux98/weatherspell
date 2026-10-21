# Weatherspell

A text-based weather app for Windows, macOS and Linux: current conditions,
the forecast, and severe-weather alerts for any place in the world, written
out in clear text. Built keyboard-first and screen-reader-first, and usable
by anyone who wants the weather in a simple and straightforward manner.

- [Download the latest release](https://github.com/PlanetLinux98/weatherspell/releases/latest)
- [User guide](USER_GUIDE.md): every feature and shortcut
- [Changelog](CHANGELOG.md): what's new in each version

## What it does

- Shows current conditions and a multi-day forecast for any location you
  choose, as readable text.
- Remembers your locations so switching between them is a keystroke away.
- Shows urgent weather alerts for your locations, and can announce new ones
  as they arrive. Alerts are available for Canada and the United States at
  this time, with more regions planned.
- Comes as one file on each system, with no account and no API key.
- Follows each system's own keys, menus, units, time format, text size and
  light or dark mode.

## Running it

Weatherspell 0.2.0, the first release for the Mac and Linux, is still to
come. Until then, the Mac and Linux versions are in the newest preview on
the [Releases page](https://github.com/PlanetLinux98/weatherspell/releases);
on the Mac, end the install line below with `| sh -s -- --preview` in
place of `| sh`.

### Windows

Weatherspell is a single `Weatherspell.exe`. Download it from the
[latest release](https://github.com/PlanetLinux98/weatherspell/releases/latest),
put it wherever you like, and run it. It needs nothing else on 64-bit
Windows 10 (version 1709 and later) and Windows 11.

Once Weatherspell is available in winget, you can also install it with:

```
winget install PlanetLinux98.Weatherspell
```

Your saved locations and settings live in `%APPDATA%\Weatherspell`, so they
survive moving or updating the executable.

### The Mac

Weatherspell runs on macOS 15 (Sequoia) and later, on Macs with Apple
silicon or Intel. To install it, paste this line into Terminal and press
Return:

```
curl -fsSL https://raw.githubusercontent.com/PlanetLinux98/weatherspell/main/tools/mac/install.sh | sh
```

It downloads the latest release, checks it against the release's
checksums, and copies Weatherspell into your Applications folder. Run the
same line again to update.

Weatherspell isn't notarized by Apple at this time. The command above
installs it so it opens without a warning; `Weatherspell.dmg` on the
release page is the disk image it installs from. If you download the disk
image in a browser instead, macOS blocks Weatherspell the first time it
opens, until you allow it in System Settings, under Privacy & Security.

Your saved locations and settings live in
`~/Library/Application Support/Weatherspell`.

### Linux

Weatherspell runs on 64-bit PCs with GTK 3, on Ubuntu 22.04, Debian 12,
Fedora 36, Linux Mint 21, openSUSE Leap 16 or later, Arch, and other
systems at least as new. It's been tested with GNOME and Orca at this
time, and should work on other desktops. Its announcements (of new alerts,
and that the forecast is ready) need Ubuntu 24.04, Linux Mint 22, Debian 13
or a system as new at this time; on older ones, the forecast and alerts
show as usual, but your screen reader may not announce them.

Download `Weatherspell-x86_64.AppImage` from the
[latest release](https://github.com/PlanetLinux98/weatherspell/releases/latest),
allow it to run as a program (in its properties in your file manager, or
with `chmod +x Weatherspell-x86_64.AppImage`), and open it. It uses your
system's own GTK, so it looks and reads like your other apps. If it
doesn't open, your system may need FUSE, usually the `fuse3` package.

Your saved locations and settings live in `~/.config/weatherspell`.

The [user guide](USER_GUIDE.md#getting-started) covers updating and
removing Weatherspell on each system.

## Accessibility

Every control is the system's own: Win32 on Windows, AppKit on the Mac and
GTK on Linux, each with its visible text as its accessible name.
Everything is reachable from the keyboard, and the forecast itself is
plain text you can easily navigate, arrow through, and copy. It's been
tested with NVDA and Narrator on Windows, VoiceOver on the Mac and Orca on
Linux. If something behaves badly with your screen reader, magnifier or
other assistive technology, that is a bug: please
[report it](https://github.com/PlanetLinux98/weatherspell/issues/new/choose).

## Data sources

Weatherspell uses these openly licensed sources, and credits them in
About Weatherspell and on the Sources line of every forecast. The list grows as
sources are added.

- Forecasts and place-name search: [Open-Meteo](https://open-meteo.com/),
  licensed [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).
- Forecast text, current conditions and alerts in Canada:
  [Environment and Climate Change Canada](https://weather.gc.ca/), from the
  [MSC Datamart](https://eccc-msc.github.io/open-data/), under its
  [data licence](https://eccc-msc.github.io/open-data/licence/readme_en/).
- Forecast text, current conditions and alerts in the United States: the
  [National Weather Service](https://www.weather.gov/), from its weather.gov
  forecast pages and its API, public domain.
- Postal codes for Canada, the UK, Australia, New Zealand and Ireland:
  [GeoNames](https://www.geonames.org/) postal code data, licensed
  [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/), built into
  the app. Open-Meteo's own search covers postal codes for the US and some
  other countries. GeoNames has only the first part of Canadian and Irish
  codes and the UK outward code, so a full code finds its area, named after
  the area's largest place.
- Names for places found by their coordinates:
  [OpenStreetMap](https://www.openstreetmap.org/copyright) data,
  © OpenStreetMap contributors, licensed
  [ODbL](https://opendatacommons.org/licenses/odbl/), looked up through
  [Nominatim](https://nominatim.org/) once per search.

The software Weatherspell is built with, such as wxWidgets, is credited
in the [third-party notices](THIRD-PARTY-NOTICES.md).

## Building from source

Weatherspell is written in Rust, with its windows made by
[wxWidgets](https://www.wxwidgets.org/) through the
[wxDragon](https://github.com/AllenDang/wxDragon) bindings. Every system
needs [Rust](https://rustup.rs/) (stable), CMake and Ninja. The first
build downloads and compiles wxWidgets, which takes several minutes.

On Windows, you also need Visual Studio 2022 or its Build Tools, with the
"Desktop development with C++" workload, which brings the C++ compiler,
the Windows SDK, CMake and Ninja. From a Developer PowerShell for Visual
Studio, which puts CMake and Ninja on the path:

```
cargo build --release -p weatherspell
```

The executable lands in `target\release\weatherspell.exe`.

On the Mac, you also need Xcode's Command Line Tools, and CMake and Ninja
(from Homebrew, for example). The app is built for Apple silicon and Intel
together, so add both of Rust's targets first:

```
rustup target add aarch64-apple-darwin x86_64-apple-darwin
sh tools/mac/make-app.sh
```

The app and its disk image land in `target/release`.

On Linux, you also need a C++ compiler, pkg-config, libclang and the GTK 3
development files. On Ubuntu or Debian:

```
sudo apt install build-essential cmake ninja-build pkg-config libclang-dev libssl-dev libgtk-3-dev libxkbcommon-dev libwayland-dev libx11-dev libxtst-dev libsm-dev libice-dev libpng-dev libjpeg-dev libexpat1-dev libtiff-dev
cargo build --release -p weatherspell
```

The program lands in `target/release/weatherspell`;
`tools/linux/make-appimage.sh` packs it as an AppImage.

On every system, run the tests with:

```
cargo test -p weatherspell-core --all-features
cargo test --release -p weatherspell -p wx-accessibility
```

The C# app that was Weatherspell 0.1 is still in `src\`, for fixes to
0.1 only. It builds on Windows with the
[.NET SDK](https://dotnet.microsoft.com/download) and
`dotnet build Weatherspell.slnx`.

See [CONTRIBUTING.md](CONTRIBUTING.md) for how changes are made and reviewed,
and [NOTES.md](NOTES.md) for the design decisions behind the project.

## Licence

[MIT](LICENSE).
