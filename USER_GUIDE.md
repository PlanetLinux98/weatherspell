# Weatherspell User Guide

Welcome to Weatherspell! Weatherspell is an accessible, text-based,
lightweight weather app that makes it fast and simple to get the forecast
and urgent alerts whenever you need them.

This guide covers everything Weatherspell does, from adding your first
location to reading alerts, on Windows, the Mac and Linux. Keys are given
as Windows and Linux use them, with the Mac's after them in brackets where
they differ, such as "press F5 (Cmd+R on the Mac)", or just "F5 (Cmd+R)"
further on. Enter is the Return key on a Mac keyboard. [Keyboard
shortcuts](#keyboard-shortcuts) lists every key for each system.

- [Getting started](#getting-started)
- [Reading the forecast](#reading-the-forecast)
- [Adding a location](#adding-a-location)
- [Managing your locations](#managing-your-locations)
- [Weather alerts](#weather-alerts)
- [Changing settings](#changing-settings)
- [Keyboard shortcuts](#keyboard-shortcuts)
- [Where your settings are kept](#where-your-settings-are-kept)
- [Getting help](#getting-help)
- [Credits and licences](#credits-and-licences)

## Getting started

### Installing on Windows

Weatherspell runs on 64-bit Windows 10 (version 1709 and later) and
Windows 11.

Weatherspell is a single executable, `Weatherspell.exe`, with nothing to
install: download it from the
[Releases page](https://github.com/PlanetLinux98/weatherspell/releases),
put it wherever you like, and open it.

Once Weatherspell is available in winget, you can also install it with:

```
winget install PlanetLinux98.Weatherspell
```

### Installing on the Mac

Weatherspell runs on macOS 15 (Sequoia) and later, on Macs with Apple
silicon or Intel.

To install it, open Terminal (in the Utilities folder inside Applications,
or search for it in Spotlight), paste this line, and press Return:

```
curl -fsSL https://raw.githubusercontent.com/PlanetLinux98/weatherspell/main/tools/mac/install.sh | sh
```

It downloads the latest release, checks it against the release's
checksums, and copies Weatherspell into your Applications folder (or the
Applications folder in your home folder, if your account can't add apps
for everyone). You can then open Weatherspell from Spotlight or the
Applications folder.

Weatherspell isn't notarized by Apple at this time. The command above
installs it so it opens without a warning. If you download
`Weatherspell.dmg` from the Releases page in a browser instead, macOS
blocks Weatherspell the first time it opens, until you allow it in System
Settings, under Privacy & Security.

### Installing on Linux

Weatherspell runs on 64-bit PCs with GTK 3, on Ubuntu 22.04, Debian 12,
Fedora 36, Linux Mint 21, openSUSE Leap 16 or later, Arch, and other
systems at least as new. It's been tested with GNOME and Orca at this
time, and should work on other desktops. Its announcements (of new alerts,
and that the forecast is ready) need Ubuntu 24.04, Linux Mint 22, Debian 13
or a system as new at this time; on older ones, the forecast and alerts
show as usual, but your screen reader may not announce them.

Weatherspell comes as one file, an AppImage:

1. Download `Weatherspell-x86_64.AppImage` from the
   [Releases page](https://github.com/PlanetLinux98/weatherspell/releases),
   and put it wherever you like.
2. Allow it to run as a program, in its properties in your file manager,
   or with this command in a terminal:

   ```
   chmod +x Weatherspell-x86_64.AppImage
   ```

3. Open it.

It uses your system's own GTK, so it looks and reads like your other apps.
If it doesn't open, your system may need FUSE, usually the `fuse3`
package. An AppImage manager, such as Gear Lever, can add Weatherspell to
your desktop's list of apps.

### Your first location

The first time you open Weatherspell, the Add Location dialog opens so you
can add your first location. Type a place name, postal code or coordinates,
then press Enter or choose Search. Select your place in the Results list and
choose Add. [Adding a location](#adding-a-location) has more on searching.

### Updating Weatherspell

Your locations and settings stay as they were when you update.

- **Windows**: replace `Weatherspell.exe` with the newer one from the
  Releases page. If you installed with winget, run this instead:

  ```
  winget upgrade PlanetLinux98.Weatherspell
  ```

- **The Mac**: quit Weatherspell, and run the install command from
  [Installing on the Mac](#installing-on-the-mac) again. It replaces the
  app with the latest release.
- **Linux**: download the newer `Weatherspell-x86_64.AppImage`, allow it
  to run as a program, and put it in place of the old one.

## Reading the forecast

The main window has three parts: the Location box at the top, the Forecast
box below it, and the status bar along the bottom (on the Mac, a status
line). The Location box shows which of your locations you're looking at,
the Forecast box holds the forecast text, and the status bar gives the
time of the last update and where it came from.

When you open Weatherspell or switch locations, the Forecast box says the
forecast is being fetched until it arrives. Your screen reader then
announces that it's ready, along with any alerts in effect.

The forecast text is divided into sections, each starting with a heading:

- **Alerts**: any weather alerts in effect, or "No alerts in effect."
- **Right now**: the current conditions, and a line with the humidity, dew
  point, pressure, visibility and cloud cover.
- **Rest of today**: what's ahead for the rest of the day and night,
  the times of sunrise and sunset, and the UV index during the day.
- **A section for each coming day**, headed with its date, such as "Monday,
  September 28".
- **Sources**: where this location's forecast comes from.

### Moving between sections

To move to the next section, choose Next Section from the View menu, or
press Ctrl+PageDown (Cmd+] on the Mac). Previous Section (Ctrl+PageUp,
or Cmd+[) moves back. To go straight to the alerts, choose Alerts from the
View menu, or press Ctrl+Shift+A (Cmd+Shift+A).

### Where the forecast comes from

In Canada and the United States, the forecast text and current conditions
come from each country's own weather service: Environment and Climate
Change Canada, and the National Weather Service. Right now usually names
the weather station that reported the conditions, and when. Everywhere else,
Weatherspell gets the forecast from Open-Meteo data, and parses it to
write in clear, plain text. Additional official weather sources may be
added in the future.

### Units and times

Weatherspell uses metric or imperial units according to your system's
region settings, with one system for each location's whole forecast:

- **Windows**: the region chosen in Settings, under Time & language.
- **The Mac**: the measurement system in System Settings, under General,
  then Language & Region.
- **Linux**: your region's formats (on GNOME, in Settings, under System,
  then Region & Language).

Forecasts for Canada are always metric, since Environment and Climate
Change Canada publishes them that way.

Times are given in the location's own time, in your system's 12-hour or
24-hour format. When your computer is in a different time zone, your
time follows in brackets, for example "6:40 am (9:40 am your time)".

### Copying the forecast

You can select and copy the forecast text as you would any other text, or
use Copy and Select All in its context menu. To open the context menu
from the keyboard, press Shift+F10, or the Applications key on Windows or
the Menu key on Linux. On the Mac, Control-click the text, or press
VO+Shift+M with VoiceOver.

### Keeping the forecast up to date

Weatherspell refreshes the forecast every 30 minutes by default, or as
often as you choose in [Settings](#changing-settings), and your place
in the text stays where it was. If you have text selected when a refresh
comes in, the new forecast waits until you clear the selection, for up to
5 minutes, so as to avoid interfering with your reason for selecting the text.

To refresh now, choose Refresh from the File menu, or press F5 (Cmd+R).

Once the forecast on screen is 30 minutes old, a line under Right now says
so, such as "Showing the forecast from 45 minutes ago."

If Weatherspell can't reach the weather service, it keeps showing the last
forecast it has, and that line gives the reason, such as "Showing the
forecast from 2 hours ago; couldn't reach the weather service." This also
happens when you open Weatherspell without an internet connection, using
the forecast it saved the last time.

## Adding a location

To add a location, choose Add Location from the Locations menu, or press
Ctrl+Shift+L (Cmd+Shift+L). It's also the Add... button in
Manage Locations, and the Add Location dialog opens on its own the first
time you open Weatherspell.

Type a place name, postal code or coordinates in the search box, then press
Enter or choose Search. The places found appear in the Results list with
their region and country, and their population when it's known, to tell
places with the same name apart. Select the one you want and choose Add
(pressing Enter or double-clicking it adds it too). Weatherspell then shows
the new location's forecast.

If the place is already one of your locations, Weatherspell switches to it
instead of saving it twice.

### Searching by postal code

Postal codes work for the United States and some other countries. For
Canada, the United Kingdom, Australia, New Zealand and Ireland, Weatherspell
has its own list of postal codes built in. For Canada, Ireland and the
United Kingdom, that list has only the first part of each code, so a full
code finds its area, named after the area's largest place.

### Searching by coordinates

You can also type or paste coordinates, in most of the ways they're
usually written, such as "44.54, -78.54" or "44°32'24"N 78°32'24"W". A link
copied from a map website works too, as long as the coordinates are in it
(short sharing links don't include them).

The forecast is then for that exact point, named after the place it's in,
with its coordinates in words, such as "44.54 north, 78.54 west". If nothing
nearby has a name, such as out at sea, the point can still be added under
its coordinates.

## Managing your locations

### Switching locations

To switch to another of your locations, choose it in the Location box. The
Locations menu also lists your first nine locations, with a check beside
the one you're viewing, and Ctrl+1 to Ctrl+9 (Cmd+1 to Cmd+9)
switch straight to them.

Weatherspell opens on the location you viewed last.

### Using Manage Locations

To change the order of your locations, give them nicknames, remove them or
choose which ones notify you about alerts, choose Manage Locations from the
Locations menu, or press Ctrl+L (Cmd+L).

The Saved locations list shows each location's full name, with its
nickname first if it has one, such as "Home (Peterborough, Ontario,
Canada)". Select a location, then use:

- **Move Up** and **Move Down** to change its place in the order, which is
  also the order of the Location box and of Ctrl+1 to Ctrl+9.
- **Add...** to add another location.
- **Edit...** to give it a nickname, such as "Home" or "Cottage", and to
  choose whether its new alerts are announced. Edit Location names the
  place you're editing. Leave the nickname field empty to use the full
  location name again. "Notify me about alerts for this location" is on
  for every location by default; if you toggle it off, "no alert
  notifications" follows the location's name in the list. Its Alerts
  section in the forecast text still shows every alert either way.
- **Remove** to remove it from your locations.

Each button's Alt key (Cmd key on the Mac), such as Alt+U for Move Up,
presses it and leaves you in the list, so you can move a location step by
step. Delete removes the selected location and F2 edits it.

Choose OK to keep your changes, or Cancel to undo all of them. Since Cancel
undoes everything, nothing asks you to confirm along the way.

## Weather alerts

Weather alerts are available at this time for Canada (Environment and
Climate Change Canada), the United States (the National Weather Service),
most of Europe (each country's national weather service via MeteoAlarm),
including the United Kingdom and Ireland, Australia (the Bureau of
Meteorology), New Zealand (MetService), Hong Kong (the Hong Kong
Observatory), the Philippines (PAGASA) and South Africa (the South African
Weather Service), with more regions planned. For other locations, the Alerts
section says "Alerts are not available for this region."

The Alerts section, at the top of the forecast text, has a line for each
alert in effect, most severe first. Each line gives the alert, when it ends
and who issued it, such as "Flood warning until 8:00 am Tuesday, from the
National Weather Service. Press Enter for details." In jurisdictions that
issue alert colours or levels, such as much of Europe, the UK, New Zealand,
Canada and South Africa, alerts begin with their level, as in "Orange
rainfall warning". With no alerts in effect, it says "No alerts in effect."

The first time Weatherspell checks alerts for a location in Australia, it
downloads the Bureau of Meteorology's warning areas, about 30 MB, so that
first forecast can take a little longer. They're kept for next time. The
Bureau's alerts come over FTP, so on a network that blocks it, Australia's
alerts can't be checked. Tsunami warnings are shown for locations within
about 3 kilometres of the coast.

To go straight to the alerts, choose Alerts from the View menu, or press
Ctrl+Shift+A (Cmd+Shift+A).

### Reading an alert's details

To read an alert in full, press Enter on its line, or double-click it. The
details window gives the alert's headline, its timing, the area it covers,
its full description and any instructions. Official page opens the issuer's
own page for the alert in your web browser, and Close returns you to the
forecast. Alerts for Europe end with a note from MeteoAlarm saying that its
own website may be more up to date.

### Alert announcements

When a new alert is issued for one of your locations, your screen reader
announces it, starting with the location's name. When you open Weatherspell
or switch locations, the announcement that the forecast is ready also names
any alerts already in effect.

Weatherspell checks for alerts every 10 minutes by default, for every
location with "Notify me about alerts for this location" turned on, not only
the one you're viewing. In [Settings](#changing-settings), you can change
how often it checks, and choose whether to announce all new alerts, only
severe and extreme ones, or none. Locations in Europe are checked at least
every 10 minutes whatever this setting says, as required by MeteoAlarm's
terms.

An alert that's updated isn't announced again, unless the update raises it,
such as from yellow to orange, or from a watch to a warning.

Alerts are announced while Weatherspell is open, and some screen readers
may only announce them while Weatherspell is the active window. On Linux,
announcements need a recent system; see
[Installing on Linux](#installing-on-linux).

If an alert check doesn't get through, the Alerts section says so and shows
the alerts from the last check that did, with how long ago that was.

## Changing settings

To change settings, choose Settings from the Settings menu. On the Mac,
Settings is in the Weatherspell menu, and Cmd+comma opens it. There are
two groups:

- **Forecast**: "Refresh the forecast every" 15 minutes, 30 minutes (the
  default), 1 hour or 2 hours.
- **Alerts and announcements**: "Check for alerts every" 5, 10 (the
  default), 15 or 30 minutes (locations in Europe are checked at least every
  10 minutes), and "Announce new alerts": All new alerts (the default),
  Severe and extreme only, or Off.

Choose OK or Apply to save your changes; Cancel closes Settings without
saving them.

### Text size and the window

Weatherspell's text follows your system's text size:

- **Windows**: the Text size and display scale settings, so its text and
  windows grow with them.
- **The Mac**: Bigger Text, Smaller Text and Actual Text Size in the View
  menu (Cmd+Plus, Cmd+Minus and Cmd+0) change the size of Weatherspell's
  text and windows while it's open.
- **Linux**: your desktop's text size and scaling (on GNOME, Large Text
  and the text scaling factor). The main window follows a change while
  it's open; a dialog that's open at the time keeps its size until you
  open it again.

Weatherspell also follows your system's light or dark mode, and changes
with it. On Windows, with a contrast theme turned on, it uses the theme's
colours. If you turn one on while Weatherspell is open in dark mode,
please restart Weatherspell to see them.

The main window opens where you last left it, at the same size, and
maximized if it was. On Linux in a Wayland session, your desktop decides
where windows open, so Weatherspell keeps only the size. To put the window
back where and how it first opened, choose Reset Window Size and Position
from the View menu.

## Keyboard shortcuts

Every command is also in the menus, with its shortcut shown beside it.

### Main window

| Action | Windows and Linux | Mac |
| --- | --- | --- |
| Refresh the forecast | F5 | Cmd+R |
| Manage Locations | Ctrl+L | Cmd+L |
| Add Location | Ctrl+Shift+L | Cmd+Shift+L |
| Switch to one of your first nine locations | Ctrl+1 to Ctrl+9 | Cmd+1 to Cmd+9 |
| Move to the Location box | Alt+O | Shift+Tab from the Forecast box |
| Next section | Ctrl+PageDown | Cmd+] |
| Previous section | Ctrl+PageUp | Cmd+[ |
| Go to the alerts | Ctrl+Shift+A | Cmd+Shift+A |
| Settings | No shortcut | Cmd+comma |
| Bigger, smaller or actual text size | Your system's settings | Cmd+Plus, Cmd+Minus, Cmd+0 |
| Open this guide | F1 | Cmd+? |
| Exit | Alt+F4 | Cmd+Q |

### Forecast text

| Action | Windows and Linux | Mac |
| --- | --- | --- |
| Open an alert's details, from its line | Enter | Return |
| Select all | Ctrl+A | Cmd+A |
| Copy | Ctrl+C | Cmd+C |
| Open the context menu | Shift+F10, or the Applications key (Windows) or Menu key (Linux) | Control-click, or VO+Shift+M with VoiceOver |

### Manage Locations

| Action | Windows and Linux | Mac |
| --- | --- | --- |
| Move Up | Alt+U | Cmd+U |
| Move Down | Alt+D | Cmd+D |
| Add | Alt+A | Cmd+A |
| Edit | Alt+E or F2 | Cmd+E or F2 |
| Remove | Alt+R or Delete | Cmd+R or Delete |

These keys work from the Saved locations list, and you stay in the list.

## Where your settings are kept

Weatherspell keeps your locations and settings, and the last forecast for
each location, in your own account's folders:

- **Windows**: everything is in `%APPDATA%\Weatherspell`. To open that
  folder, type `%APPDATA%\Weatherspell` into File Explorer's address bar
  or the Run dialog (Windows+R).
- **The Mac**: your locations and settings are in
  `~/Library/Application Support/Weatherspell`, and the saved forecasts in
  `~/Library/Caches/Weatherspell`. To open one, choose Go to Folder from
  Finder's Go menu (Cmd+Shift+G), and type its name.
- **Linux**: your locations and settings are in `~/.config/weatherspell`,
  and the saved forecasts in `~/.cache/weatherspell` (or wherever your
  system's XDG folders point).

Everything there stays when Weatherspell is moved, replaced or updated.
It doesn't travel with Weatherspell, though, so a copy on another computer
starts fresh.

If Weatherspell ever can't read its settings, it tells you and starts
without your saved locations. The file it couldn't read is kept as
`settings.json.bad` in the same folder, so nothing is lost.

### Removing Weatherspell

- **Windows**: delete `Weatherspell.exe`, or run this if you installed it
  with winget:

  ```
  winget uninstall PlanetLinux98.Weatherspell
  ```

- **The Mac**: quit Weatherspell, and move it from your Applications
  folder to the Trash.
- **Linux**: delete `Weatherspell-x86_64.AppImage`. If an AppImage manager
  added it to your list of apps, remove it there instead.

To remove your locations and settings as well, delete the folders listed
under [Where your settings are kept](#where-your-settings-are-kept).

## Getting help

To open this guide, choose User Guide from the Help menu, or press F1
(Cmd+?). It opens in your web browser, or default HTML viewer.

About Weatherspell, in the Help menu (in the Weatherspell menu on the
Mac), gives the version you're running and short credits, with buttons
for the full credits and licences and for Weatherspell's website.

If something doesn't work as expected, or reads or behaves badly with your
screen reader, magnifier or other assistive technology, please
[report it on GitHub](https://github.com/PlanetLinux98/weatherspell/issues/new/choose).
Ideas for Weatherspell are welcome there too. Reporting needs a free GitHub
account, and including the version from About, and your system, helps.

## Supporting Weatherspell

Weatherspell is free for everyone. If it's helped you and you'd like to
support its development, choose Donate from the Help menu, which opens
Weatherspell's page on [Ko-fi](https://ko-fi.com/planetlinux) in your web
browser. You can give once or monthly, by card or PayPal, without a Ko-fi
account. Donations go toward the time, testing and tools that keep
Weatherspell going, and are always appreciated, but never expected.

## Credits and licences

Weatherspell is free and open source, released under the MIT licence below.
It is built on data and software that are openly licensed or free to use,
credited here as each one asks. The Sources section at the end of every
forecast names the sources used for that location.

### Weather

- [Weather data by Open-Meteo.com](https://open-meteo.com/), licensed
  [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/): forecasts for
  every location, and current conditions where no official observation is
  used. Weatherspell writes its own sentences from this data.
- Data Source: [Environment and Climate Change Canada](https://weather.gc.ca/),
  under its [Data Server End-use Licence](https://eccc-msc.github.io/open-data/licence/readme_en/):
  forecast text, current conditions and alerts in Canada.
- The [National Weather Service](https://www.weather.gov/): forecast text,
  current conditions and alerts in the United States, in the public domain.
- Each European country's national weather service, through
  [MeteoAlarm](https://meteoalarm.org/) (EUMETNET), under
  [its terms](https://meteoalarm.org/en/live/page/terms-and-conditions),
  which are those of
  [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) with a few
  additions: alerts in Europe, including the United Kingdom and Ireland. The
  outlines of MeteoAlarm's warning regions, simplified, are built into
  Weatherspell.
- [MetService](https://www.metservice.com/), licensed
  [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/): alerts in New
  Zealand.
- © [Bureau of Meteorology](https://www.bom.gov.au/), for your own use under
  [its copyright terms](https://www.bom.gov.au/copyright): alerts in
  Australia, from its anonymous FTP service. The outlines of its warning
  areas aren't built into Weatherspell: it downloads them from the Bureau
  the first time they're needed.
- The [Hong Kong Observatory](https://www.hko.gov.hk/), from its open data
  on [DATA.GOV.HK](https://data.gov.hk/): alerts in Hong Kong.
- [PAGASA](https://www.pagasa.dost.gov.ph/), licensed
  [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/): alerts in the
  Philippines. The outlines of the provinces, simplified, are built into
  Weatherspell, from [geoBoundaries](https://www.geoboundaries.org/)' copy
  of the boundaries published by NAMRIA and the Philippine Statistics
  Authority, licensed
  [CC BY 3.0 IGO](https://creativecommons.org/licenses/by/3.0/igo/).
- The [South African Weather Service](https://www.weathersa.co.za/): alerts
  in South Africa, in the public domain.

### Places

- Place search: Open-Meteo, with location data based on
  [GeoNames](https://www.geonames.org/).
- Postal codes for Canada, the UK, Australia, New Zealand and Ireland:
  [GeoNames](https://www.geonames.org/) postal code data, licensed
  [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/), built into
  Weatherspell.
- Names for places found by their coordinates:
  © [OpenStreetMap contributors](https://www.openstreetmap.org/copyright),
  licensed [ODbL](https://opendatacommons.org/licenses/odbl/), looked up
  through [Nominatim](https://nominatim.org/).

### Software

Weatherspell's licence:

> MIT License
>
> Copyright (c) 2026 PlanetLinux98
>
> Permission is hereby granted, free of charge, to any person obtaining a
> copy of this software and associated documentation files (the
> "Software"), to deal in the Software without restriction, including
> without limitation the rights to use, copy, modify, merge, publish,
> distribute, sublicense, and/or sell copies of the Software, and to permit
> persons to whom the Software is furnished to do so, subject to the
> following conditions:
>
> The above copyright notice and this permission notice shall be included
> in all copies or substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
> OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
> MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN
> NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
> DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
> OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE
> USE OR OTHER DEALINGS IN THE SOFTWARE.

Weatherspell is built with [wxWidgets](https://www.wxwidgets.org/), the
libraries that come with it, and libraries written in Rust; on Linux, the
AppImage also holds the AppImage runtime that starts it. Their licences,
and the credit each one asks for, are in the
[third-party notices](THIRD-PARTY-NOTICES.md).
