# Changelog

All notable changes to Weatherspell are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
While Weatherspell is in the `0.x` series, behaviour may change between minor
versions. Notes for the alpha and beta previews before 0.1.0 are on the
[Releases](https://github.com/PlanetLinux98/weatherspell/releases) page.

## [Unreleased]

### Added
- Follows the light or dark mode chosen in Windows (#24).

### Changed
- Weatherspell is rebuilt in Rust with wxWidgets, to streamline
  development and as the base for Mac and Linux versions later. It's still
  one executable, and it keeps using your saved locations and settings
  from prior versions (#24).
- Manage Locations: Rename becomes Edit (Alt+E or F2), and the choice of
  whether a location's alerts are announced moves into the Edit Location
  dialog, which names the place (#24).

### Fixed
- With Narrator, the heading was said twice after Ctrl+PageDown,
  Ctrl+PageUp or Ctrl+Shift+A (#23).

## [0.1.0] - 2026-10-04

The first release. Simply download `Weatherspell.exe` and run it. For
Windows 10 (version 1903 or later) and Windows 11. Press F1 for the user
guide.

### Added
- The forecast as text you can arrow through, select and copy, in sections:
  Alerts, Right now, Rest of today, the coming days, and Sources.
  Ctrl+PageDown and Ctrl+PageUp move between them, and every command is in
  a menu with its shortcut (#5, #15, #17, #23).
- In Canada and the United States, including most US territories, the
  forecast in the words of official sources (Environment and Climate
  Change Canada or the National Weather Service), with current conditions
  from the nearest station (#3, #19).
- Everywhere else, a forecast written from Open-Meteo data: the rest of
  today by part of day, and each coming day as a day and a night (#2).
- Shows the location's own local time, with your PC time in parentheses
  when they differ. Each location's forecast is in one unit system: metric
  in Canada, and elsewhere per your Windows region (#2, #6, #21).
- Urgent weather alerts for Canada and the United States, most severe
  first. Press Enter on an alert, or double-click it, for its details;
  Ctrl+Shift+A moves to the alerts section. Every saved location is
  checked every 10 minutes by default (configurable), and new alerts can
  be announced as they arrive (#4, #19, #20).
- Add Location (Ctrl+Shift+L) finds a place by name, postal code or
  coordinates (#1, #18).
- Manage Locations (Ctrl+L): put locations in order, rename them with a
  nickname such as "Home", remove them, and choose which locations' alerts
  are announced. Ctrl+1 to Ctrl+9 switch to the first nine (#1).
- The forecast refreshes itself, every 30 minutes by default
  (configurable), without moving the caret, and says how old it is once
  it's out of date. Each location's last forecast and alerts are kept,
  and shown with their age when the weather service can't be reached
  (#5, #6, #7, #16).
- Settings: how often the forecast refreshes and alerts are checked, and
  which new alerts are announced (#6).
- The window follows the display scale and the Windows Text size setting,
  and opens how it was last closed (#5, #16).
- A user guide (F1), and an About dialog with credits for each source (#8).

[Unreleased]: https://github.com/PlanetLinux98/weatherspell/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/PlanetLinux98/weatherspell/releases/tag/v0.1.0
