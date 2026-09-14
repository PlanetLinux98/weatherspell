# Changelog

All notable changes to Weatherspell are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
While Weatherspell is in the `0.x` series, behaviour may change between minor
versions.

## [Unreleased]

## [0.1.0-alpha.1] - 2026-09-13

A first preview for testing, ahead of 0.1.0. Download `Weatherspell.exe`
and run it; there is nothing to install. Windows 10 (version 1903 or later)
or Windows 11.

### Added
- Weather alerts for Canada and the United States, first in the text, most
  severe first, each on one line with when it ends and who issued it; Enter
  on the line opens the full text with an Official page button, and
  Ctrl+Shift+A jumps to the Alerts heading. Every saved location is checked
  every ten minutes and a new alert is spoken through the screen reader
  without moving focus ("Peterborough: severe thunderstorm warning until
  6:00 pm today"), once per alert. Elsewhere the text says alerts are not
  available for the region, and a source that cannot be reached is stated,
  never shown as a quiet day.
- Find a place by name or postal code (Ctrl+L) and save it; the location
  last viewed comes back on the next launch. Postal codes for Canada, the
  UK, Australia, New Zealand and Ireland come from a GeoNames table built
  into the exe (Open-Meteo's search has none for them), each resolving to
  its area (the first three characters of a Canadian or Irish code, the UK
  outward code) named after the area's largest place.
- In Canada and the United States, the forecast in the words of Environment
  Canada or the National Weather Service: each period as the service wrote
  it ("Tonight", "Saturday", "Saturday night"), under the same headings, and
  current conditions from the nearest station with its name and observation
  time. Open-Meteo still supplies the hourly data, sun and UV, and stands in
  when the official text cannot be fetched; the Sources line says which.
- A forecast written in words from Open-Meteo data: right now, the rest of
  today by part of day, each coming day as a daytime and a night sentence
  ("Saturday", "Saturday night"), sun and UV, and details such as humidity
  and pressure. Whole degrees, chances rounded to tens, "kilometres an hour"
  rather than symbols, and the location's own times with yours in brackets
  when they differ. Units follow the Windows region setting.
- Standard Windows menus, so every shortcut is read out where it is shown:
  Ctrl+PageDown and Ctrl+PageUp jump between sections, F5 refreshes.
- The window follows the Windows display scale and the Text size
  accessibility setting, in the system font.

[Unreleased]: https://github.com/PlanetLinux98/weatherspell/compare/v0.1.0-alpha.1...HEAD
[0.1.0-alpha.1]: https://github.com/PlanetLinux98/weatherspell/releases/tag/v0.1.0-alpha.1
