# Changelog

All notable changes to Weatherspell are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
While Weatherspell is in the `0.x` series, behaviour may change between minor
versions.

## [Unreleased]

### Added
- A Settings dialog (Settings menu): how often the forecast on screen is
  refreshed (15 minutes to 2 hours, default 30), how often every saved
  location's alerts are checked (5 to 30 minutes, default 10), and which
  new alerts are spoken (all, severe and extreme only, off). OK, Cancel
  and Apply; a value edited by hand into `settings.json` is kept and
  listed.
- The forecast refreshes itself while the window is open, without moving
  focus or the caret: the text is replaced under the reader and the caret
  stays on the same words, as it now does for F5 too. A refresh that
  fails leaves the text on screen and dates it on the first line under
  Right now ("Showing the forecast from 20 minutes ago; couldn't reach the
  weather service"); the line also appears on its own once the text is 30
  minutes old, and there is no "Updated just now" line any more.

### Changed
- Each location's whole text is in one unit system, so its numbers never
  disagree with the official sentences beside them: Canada is metric, as
  Environment Canada writes it; the United States follows the Windows
  region, with the National Weather Service text requested to match
  (metric text for a metric region); everywhere else follows the Windows
  region.

### Fixed
- Alert checks could stay off after the first location was added on a
  first run, until the next launch.
- NVDA said nothing when a combo box was arrowed through without opening
  its list, in the main window and in Settings.
- NVDA read the whole forecast when the mouse pointer moved across it; it
  now reads the paragraph under the pointer, as in any edit control.
- A weather service that took too long to answer was treated as a
  cancelled refresh: the status bar could stay at "Fetching" and an
  automatic refresh failed silently. It now counts as a failed fetch, with
  the reason stated.

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
