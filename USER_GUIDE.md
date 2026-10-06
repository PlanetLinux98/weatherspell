# Weatherspell User Guide

Welcome to Weatherspell! Weatherspell is an accessible, text-based,
lightweight weather app that makes it fast and simple to get the forecast
and urgent alerts whenever you need them.

This guide covers everything Weatherspell does, from adding your first
location to reading alerts.

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

Weatherspell runs on 64-bit Windows 10 (version 1709 and later) and
Windows 11.

Weatherspell is a single executable, `Weatherspell.exe`, with nothing to
install: download
it from the [Releases page](https://github.com/PlanetLinux98/weatherspell/releases),
put it wherever you like, and open it.

You can also install it with winget:

```
winget install PlanetLinux98.Weatherspell
```

The first time you open Weatherspell, the Add Location dialog opens so you
can add your first location. Type a place name, postal code or coordinates,
then press Enter or choose Search. Select your place in the Results list and
choose Add. [Adding a location](#adding-a-location) has more on searching.

### Updating Weatherspell

To update, replace `Weatherspell.exe` with the newer one from the Releases
page. If you installed with winget, run this instead:

```
winget upgrade PlanetLinux98.Weatherspell
```

Either way, your locations and settings stay as they were.

## Reading the forecast

The main window has three parts: the Location box at the top, the Forecast
box below it, and the status bar along the bottom. The Location box shows
which of your locations you're looking at, the Forecast box holds the
forecast text, and the status bar gives the time of the last update and
where it came from.

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
press Ctrl+PageDown. Previous Section (Ctrl+PageUp) moves back. To go
straight to the alerts, choose Alerts from the View menu, or press
Ctrl+Shift+A.

### Where the forecast comes from

In Canada and the United States, the forecast text and current conditions
come from each country's own weather service: Environment and Climate
Change Canada, and the National Weather Service. Right now usually names
the weather station that reported the conditions, and when. Everywhere else,
Weatherspell gets the forecast from Open-Meteo data, and parses it to
write in clear, plain text. Additional official weather sources may be
added in the future.

### Units and times

Weatherspell uses metric or imperial units according to your Windows region
settings, with one system for each location's whole forecast. Forecasts for
Canada are always metric, since Environment and Climate Change Canada
publishes them that way.

Times are given in the location's own time. When your computer is in a
different time zone, your time follows in brackets, for example "6:40 am
(9:40 am your time)".

### Copying the forecast

You can select and copy the forecast text as you would any other text, or
use Copy and Select All in the right-click menu (Shift+F10 or the
Applications key opens it from the keyboard).

### Keeping the forecast up to date

Weatherspell refreshes the forecast every 30 minutes by default, or as
often as you choose in [Settings](#changing-settings), and your place
in the text stays where it was. If you have text selected when a refresh
comes in, the new forecast waits until you clear the selection, for up to
5 minutes, so as to avoid interfering with your reason for selecting the text.

To refresh now, choose Refresh from the File menu, or press F5.

Once the forecast on screen is 30 minutes old, a line under Right now says
so, such as "Showing the forecast from 45 minutes ago."

If Weatherspell can't reach the weather service, it keeps showing the last
forecast it has, and that line gives the reason, such as "Showing the
forecast from 2 hours ago; couldn't reach the weather service." This also happens when you
open Weatherspell without an internet connection, using the forecast it
saved the last time.

## Adding a location

To add a location, choose Add Location from the Locations menu, or press
Ctrl+Shift+L. It's also the Add... button in Manage Locations, and the Add
Location dialog opens on its own the first time you open Weatherspell.

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
the one you're viewing, and Ctrl+1 to Ctrl+9 switch straight to them.

Weatherspell opens on the location you viewed last.

### Using Manage Locations

To change the order of your locations, give them nicknames, remove them or
choose which ones notify you about alerts, choose Manage Locations from the
Locations menu, or press Ctrl+L.

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

Choose OK to keep your changes, or Cancel to undo all of them. Since Cancel
undoes everything, nothing asks you to confirm along the way.

## Weather alerts

Weather alerts are available for Canada and the United States at this time,
with more regions planned. They come from Environment and Climate Change
Canada and the National Weather Service. For other locations, the Alerts
section says "Alerts are not available for this region."

The Alerts section, at the top of the forecast text, has a line for each
alert in effect, most severe first. Each line gives the alert, when it ends
and who issued it, such as "Flood warning until 8:00 am Tuesday, from the
National Weather Service. Press Enter for details." With no alerts in
effect, it says "No alerts in effect."

To go straight to the alerts, choose Alerts from the View menu, or press
Ctrl+Shift+A.

### Reading an alert's details

To read an alert in full, press Enter on its line, or double-click it. The
details window gives the alert's headline, its timing, the area it covers,
its full description and any instructions. Official page opens the
issuer's own page for the alert in your web browser, and Close returns you
to the forecast.

### Alert announcements

When a new alert is issued for one of your locations, your screen reader
announces it, starting with the location's name. When you open Weatherspell
or switch locations, the announcement that the forecast is ready also names
any alerts already in effect.

Weatherspell checks for alerts every 10 minutes by default, for every
location with "Notify me about alerts for this location" turned on, not
only the one you're viewing. In [Settings](#changing-settings), you can
change how often it checks, and choose whether to announce all new alerts,
only severe and extreme ones, or none.

Alerts are announced while Weatherspell is open, and some screen readers
may only announce them while Weatherspell is the active window.

If an alert check doesn't get through, the Alerts section says so and shows
the alerts from the last check that did, with how long ago that was.

## Changing settings

To change settings, choose Settings from the Settings menu. There are two
groups:

- **Forecast**: "Refresh the forecast every" 15 minutes, 30 minutes (the
  default), 1 hour or 2 hours.
- **Alerts and announcements**: "Check for alerts every" 5, 10 (the
  default), 15 or 30 minutes, and "Announce new alerts": All new alerts (the
  default), Severe and extreme only, or Off.

Choose OK or Apply to save your changes; Cancel closes Settings without
saving them.

### Text size and the window

Weatherspell follows your Windows text size and display scale settings, so
its text and windows grow with them.

Weatherspell also follows the light or dark mode chosen in Windows' colour
settings, and changes with it. With a contrast theme turned on, it uses the
theme's colours. If you turn one on while Weatherspell is open in dark
mode, please restart Weatherspell to see them.

The main window opens where you last left it, at the same size, and
maximized if it was. To put it back where and how it first opened, choose
Reset Window Size and Position from the View menu.

## Keyboard shortcuts

Every command is also in the menus, with its shortcut shown beside it.

### Main window

| Action | Keys |
| --- | --- |
| Refresh the forecast | F5 |
| Manage Locations | Ctrl+L |
| Add Location | Ctrl+Shift+L |
| Switch to one of your first nine locations | Ctrl+1 to Ctrl+9 |
| Move to the Location box | Alt+O |
| Next section | Ctrl+PageDown |
| Previous section | Ctrl+PageUp |
| Go to the alerts | Ctrl+Shift+A |
| Open this guide | F1 |
| Exit | Alt+F4 |

### Forecast text

| Action | Keys |
| --- | --- |
| Open an alert's details, from its line | Enter |
| Select all | Ctrl+A |
| Copy | Ctrl+C |
| Open the right-click menu | Shift+F10 or the Applications key |

### Manage Locations

| Action | Keys |
| --- | --- |
| Move Up | Alt+U |
| Move Down | Alt+D |
| Add | Alt+A |
| Edit | Alt+E or F2 |
| Remove | Alt+R or Delete |

These keys work from the Saved locations list, and you stay in the list.

## Where your settings are kept

Weatherspell keeps your locations, your settings and the last forecast for
each location in `%APPDATA%\Weatherspell`. To open that folder, type
`%APPDATA%\Weatherspell` into File Explorer's address bar or the Run dialog
(Windows+R).

Everything there stays when the executable is moved, replaced or updated.
It doesn't travel with the executable, though, so a copy on another
computer starts fresh.

If Weatherspell ever can't read its settings, it tells you and starts
without your saved locations. The file it couldn't read is kept as
`settings.json.bad` in the same folder, so nothing is lost.

### Removing Weatherspell

To remove Weatherspell, delete `Weatherspell.exe`, or run this if you
installed it with winget:

```
winget uninstall PlanetLinux98.Weatherspell
```

To remove your locations and settings as well, delete the
`%APPDATA%\Weatherspell` folder.

## Getting help

To open this guide, choose User Guide from the Help menu, or press F1. It
opens in your web browser, or default HTML viewer.

About Weatherspell, in the Help menu, gives the version you're running and
short credits, with buttons for the full credits and licences and for
Weatherspell's website.

If something doesn't work as expected, or reads or behaves badly with your
screen reader, magnifier or other assistive technology, please
[report it on GitHub](https://github.com/PlanetLinux98/weatherspell/issues/new/choose).
Ideas for Weatherspell are welcome there too. Reporting needs a free GitHub
account, and including the version from About helps.

## Credits and licences

Weatherspell is free and open source, released under the MIT licence
below. It is built on openly licensed data and software, credited here as
each one asks. The Sources section at the end of every forecast names the
sources used for that location.

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
libraries that come with it, and libraries written in Rust. Their
licences, and the credit each one asks for, are in the
[third-party notices](THIRD-PARTY-NOTICES.md).
