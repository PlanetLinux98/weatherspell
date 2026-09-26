# Changelog

All notable changes to Weatherspell are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
While Weatherspell is in the `0.x` series, behaviour may change between minor
versions.

## [Unreleased]

### Added
- Manage Locations (Ctrl+L): put saved locations in order with Move Up
  and Move Down (Alt+U, Alt+D), add, rename with a nickname such as
  "Home" (F2), remove (Delete), and choose for each whether its new alerts
  are spoken. Nothing changes until OK. Ctrl+1 to Ctrl+9 switch to the
  first nine locations, which the Locations menu also lists (#1).

### Changed
- Find Location is now Add Location, on Ctrl+Shift+L; Ctrl+L opens Manage
  Locations (#1).

### Fixed
- An alert check set the whole text again even when nothing in it had
  changed, dropping any selection (NVDA said "unselected") and scrolling
  the view back to the caret. Now only changed text is set, an automatic
  refresh waits while text is selected, and a refresh keeps the view where
  it was (#16).
- The Location box was cut off along the bottom at large text sizes (#16).
- Ctrl+PageDown, Ctrl+PageUp and Ctrl+Shift+A moved to a heading without
  NVDA saying anything; the heading is now spoken, and so is "No next
  section" at the end (#17).
- A forecast that could not be fetched at launch could go unspoken, and on
  a switch the failure could be spoken before the location's name; the
  forecast it falls back to is now dated with its day when it is not
  today's. F5 now says when the forecast has been updated (#17).
- Find Location says how many places it found after moving to the results
  rather than while NVDA is announcing them, and says when a search took
  too long instead of leaving "Searching" (#17).
- An alert with no end time, such as a hurricane watch, was read as ending
  when its message expired ("Hurricane watch until 1:00 pm today", the time
  of the next update), and the offline view dropped it then. Such an alert
  now reads without "until", and its details say when the message expires
  (#20).
- The "your time" in brackets named the wrong day for a location ahead of
  or behind this PC ("8:08 pm yesterday your time" for a sunrise due that
  evening). It now names your own day, as alert times do, unless both
  times fall on your today (#21).
- Wording: "1 degree" and "1 centimetre"; "it's 13 degrees with drizzle"
  rather than "and drizzle"; "light snow, heavier at times" rather than
  "light snow with snow at times"; "The sun does not set today" (or rise)
  in polar day and night; station names read as words ("Gander
  International Airport"); an Environment Canada site with no forecast text
  says so instead of that the text could not be fetched this time; the
  offline alerts line reads "none were in effect when last checked" (#21).
- Puerto Rico, the US Virgin Islands, Guam, the Northern Mariana Islands
  and American Samoa got neither National Weather Service text nor alerts,
  and were saved with no territory in their name ("San Juan"). They are now
  named and covered like the states; American Samoa, which has no NWS
  forecast text, gets its alerts (#19).
- Some Canadian postal codes were placed far from their area and got a
  neighbouring town's forecast (T2P, downtown Calgary, got Strathmore's),
  and a third of them were named with a long list of neighbourhoods that
  was spoken in every announcement. Canadian codes now sit at the middle of
  their area and carry the town's name ("Calgary") (#18).
- A settings file that could not be read was replaced, with every saved
  location in it, by the next save, and said so only in the status bar
  behind the first-run dialog. It is now kept as settings.json.bad, and a
  message says so (#22).
- An unexpected error in a refresh or an alert check, such as from a
  damaged cache file, showed the .NET error dialog and left the text at
  "Fetching"; it now ends in a Problem section or the status bar, and a
  damaged cache file is ignored (#22).
- A second launch opened a second window, each announcing every alert and
  saving over the other's settings; it now brings the first window
  forward. A place already saved is switched to instead of being saved
  twice (#22).
- Narrator read the forecast and an alert's details only as a whole: all
  of it whenever the text took focus, and nothing as the arrow keys moved
  through it. Both are now read line by line in Narrator as in NVDA (#23).

## [0.1.0-alpha.2] - 2026-09-15

A second preview for testing, ahead of 0.1.0. Download `Weatherspell.exe`
and run it; there is nothing to install. Windows 10 (version 1903 or later)
or Windows 11. Settings and saved locations from alpha.1 carry over.

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
- Each saved location's last forecast and alerts are kept on disk (under
  `%APPDATA%\Weatherspell\cache`), so a launch or a switch that cannot
  reach the weather service still shows them, dated the same way. The
  Alerts section of a location whose check failed says when it was last
  checked and lists the alerts still in effect from then, in place of
  "couldn't be checked" alone. While a location is being fetched the text
  says so, and a short spoken notification says when its forecast is
  ready; the reason a fetch failed is now the underlying one ("The remote
  name could not be resolved") rather than "An error occurred while
  sending the request".

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

[Unreleased]: https://github.com/PlanetLinux98/weatherspell/compare/v0.1.0-alpha.2...HEAD
[0.1.0-alpha.2]: https://github.com/PlanetLinux98/weatherspell/releases/tag/v0.1.0-alpha.2
[0.1.0-alpha.1]: https://github.com/PlanetLinux98/weatherspell/releases/tag/v0.1.0-alpha.1
