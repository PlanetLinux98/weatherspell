# Weatherspell

A text-based weather app for Windows: current conditions, the forecast, and
severe-weather alerts for any place in the world, written out in clear text.
Built keyboard-first and screen-reader-first, and usable by anyone who wants the
weather in a simple and straightforward manner.

> **Pre-release.** Weatherspell is being built in the open and is currently
> pre-release. Keep an eye on the [Releases](https://github.com/PlanetLinux98/weatherspell/releases)
> page, and the [Changelog](CHANGELOG.md).

## What it will do

- Show current conditions and a multi-day forecast for any location you
  choose, as readable text.
- Remember your locations so switching between them is a keystroke away.
- Surface urgent and severe weather alerts for the selected location.
- Run solely as a single exe: no account, no API key, no installer.

## Running it

Weatherspell is a single `Weatherspell.exe`. Download it from a release, put it
wherever you like, and run it. It uses the .NET Framework 4.8 that is natively
part of Windows 10 (version 1903 and later) and Windows 11.

Your saved locations and settings live in `%APPDATA%\Weatherspell`, so they
survive moving or updating the exe.

Once released, it will also be installable with winget:
`winget install PlanetLinux98.Weatherspell`.

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
- Postal codes for Canada, the UK, Australia, New Zealand and Ireland:
  [GeoNames](https://www.geonames.org/) postal code data, licensed
  [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/), built into the
  exe. Open-Meteo's own search covers postal codes for the US and some
  other countries. GeoNames has only the first part of Canadian and Irish
  codes and the UK outward code, so a full code finds its area, named after
  the area's largest place.

## Building from source

Requires the [.NET SDK](https://dotnet.microsoft.com/download) (10.0 or later)
on Windows. No Visual Studio or C++ toolchain is needed.

```bash
dotnet build Weatherspell.slnx -c Release
```

The exe lands in `src\Weatherspell\bin\Release\net48\`. Run the tests with:

```bash
dotnet test Weatherspell.slnx
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for how changes are made and reviewed,
and [NOTES.md](NOTES.md) for the design decisions behind the project.

## Licence

[MIT](LICENSE).
