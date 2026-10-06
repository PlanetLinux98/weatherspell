# Weatherspell

A text-based weather app for Windows: current conditions, the forecast, and
severe-weather alerts for any place in the world, written out in clear text.
Built keyboard-first and screen-reader-first, and usable by anyone who wants the
weather in a simple and straightforward manner.

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
- Runs as a single executable: no account, no API key, no installer.

## Running it

Weatherspell is a single `Weatherspell.exe`. Download it from the
[latest release](https://github.com/PlanetLinux98/weatherspell/releases/latest),
put it wherever you like, and run it. It needs nothing else on
Windows 10 (version 1709 and later) and Windows 11.

You can also install it with winget:

```
winget install PlanetLinux98.Weatherspell
```

Your saved locations and settings live in `%APPDATA%\Weatherspell`, so they
survive moving or updating the exe. The user guide covers updating and
removing Weatherspell.

## Accessibility

Every control is a standard Windows Win32 control with its visible text as an
accessible name, everything is reachable from the keyboard, and the forecast
itself is plain text you can easily navigate, arrow through, and copy. If something
behaves badly with your screen reader, magnifier or other assistive technology, that is
a bug: please [report it](https://github.com/PlanetLinux98/weatherspell/issues/new/choose).

## Data sources

Weatherspell uses these openly licensed sources, and credits them in
Help > About and on the Sources line of every forecast. The list grows as
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
  [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/), built into the
  exe. Open-Meteo's own search covers postal codes for the US and some
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
[wxDragon](https://github.com/AllenDang/wxDragon) bindings. To build it
on Windows, you need:

- [Rust](https://rustup.rs/) (stable).
- Visual Studio 2022 or its Build Tools, with the "Desktop development
  with C++" workload, which brings the C++ compiler, the Windows SDK,
  CMake and Ninja.

From a Developer PowerShell for Visual Studio, which puts CMake and Ninja
on the path:

```
cargo build --release -p weatherspell
```

The first build downloads and compiles wxWidgets, which takes several
minutes. The exe lands in `target\release\weatherspell.exe`. Run the
tests with:

```
cargo test -p weatherspell-core --all-features
cargo test --release -p weatherspell -p wx-accessibility
```

The C# app that was Weatherspell 0.1 is still in `src\`, for fixes to
0.1 only. It builds with the [.NET SDK](https://dotnet.microsoft.com/download)
and `dotnet build Weatherspell.slnx`.

See [CONTRIBUTING.md](CONTRIBUTING.md) for how changes are made and reviewed,
and [NOTES.md](NOTES.md) for the design decisions behind the project.

## Licence

[MIT](LICENSE).
