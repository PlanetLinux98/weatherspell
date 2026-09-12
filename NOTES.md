# Design decisions

Why Weatherspell is built the way it is, including the roads not taken, so
they are not re-proposed without new information.

## One exe, and what that rules out

The whole program is a single `Weatherspell.exe` that runs the moment it is
downloaded. Everything below follows from that.

### .NET Framework 4.8 over .NET 10

.NET Framework 4.8 is part of Windows 10 (1903+) and Windows 11, so the exe is
tens of kilobytes and needs nothing installed. A .NET 10 self-contained
single-file build is also "one exe", but it carries the runtime inside: about
65 MB, a visible decompression pause on first launch, and WinForms cannot be
trimmed to shrink it. A framework-dependent .NET 10 build would be small but
pops a "download .NET" prompt on machines without the runtime.

What is given up: the BCL is frozen, and WinForms on 4.8 has the 2018-era UI
Automation fixes but not those that landed in .NET 5 to 10. For the control
set this app uses (labels, text boxes, lists, buttons, menus) those controls
were already solid. Modern C# syntax still works: PolySharp generates the
compiler-support types at build time. Runtime-backed language features
(default interface members, static abstract members, ref fields) do not.

Revisit if the app ever needs a control whose 4.8 accessibility is genuinely
broken, or if the size of a .NET 10 build stops mattering.

### No `Weatherspell.exe.config`

The SDK would normally generate one beside the exe. It is switched off
(`GenerateSupportedRuntime=false`) so the exe is provably self-sufficient.
Consequences:

- **No binding redirects**, so no runtime NuGet dependencies. Build-time-only
  packages (MinVer, PolySharp) are fine because nothing of theirs ships.
- **System DPI awareness, not per-monitor.** WinForms 4.8 only rescales on a
  DPI change when told to through the config file. The manifest declares the
  app system-DPI-aware instead: crisp on the primary display at any scale,
  bitmap-scaled by Windows on a display with a different scale. Per-monitor
  v2 is a two-line change if the config file is ever accepted.
- **JSON and HTTP come from the in-box BCL.** Settings and API responses use
  what 4.8 ships; no System.Text.Json (it would be a DLL).

Rejected: merging a class library into the exe with ILRepack. It works, but
adds a build tool to maintain for a boundary that convention keeps just as
well.

## Settings live in `%APPDATA%\Weatherspell`

Not beside the exe. Storing next to the exe breaks under `Program Files` and,
under winget, whose upgrade and uninstall delete the whole package folder. A per-user
 AppData folder survives both and works when the exe sits somewhere read-only.
The only potential cost is that settings do not travel with the exe on a USB
stick; a portable-marker mode can be added later if
anyone asks.

## Tests reference the exe directly

The xUnit project takes a `ProjectReference` to the app; a .NET Framework exe
is an ordinary assembly, and `InternalsVisibleTo` opens the internals. No
separate Core library (a second DLL) and no linked-source tricks. The
discipline it imposes is the one wanted anyway: logic in plain classes, forms
kept thin.

The `AccessibilityLint` test constructs the forms on an STA thread without
creating window handles, so it runs on a headless CI runner.

## Hand-coded forms, no designer files

Controls are created in code rather than in `.Designer.cs` + `.resx` pairs.
Every accessible name, tab index and label association is then visible in one
readable file and reviewable in a diff, which matters more here than
drag-and-drop layout. The forms are simple enough that this costs little.

## Accessible name is the visible text

Screen readers announce a control by the text it shows. Text-less inputs take
their visible label's text as `AccessibleName`, verbatim. Not a reworded or
longer alternative unless necessary: users who see the screen and hear it should get
the same words (WCAG 2.5.3). Extra detail usually belongs in a tooltip or help text.

## Weather data: keyless sources only

Forecasts from federal government sources (assuming free access, no API key or account
 needed), or [Open-Meteo](https://open-meteo.com/) (free, global, no API
key); urgent alerts from government feeds (the US National Weather Service,
Environment Canada, MeteoAlarm for Europe, others as they are found).
Nothing to sign up for, so the exe works for everyone on first run.

Rejected: a keyed provider such as OpenWeatherMap One Call, which would cover
forecast and global alerts in one API but require every user to create an
account and paste a key into Settings.

## Postal codes come from a table in the exe

Open-Meteo's geocoder finds places by name everywhere, but its postal code
search is dependable only for the US, France, Spain, the Netherlands and
Belgium; it is patchy across the rest of Europe and has nothing for Canada,
the UK, Australia, New Zealand or Ireland (every GeoNames country was probed
in September 2026). So the exe carries the GeoNames postal code table (CC BY
4.0) for those five, the English-speaking countries whose users are likely
to type a code: about half a megabyte of text, embedded, no network needed.
The Find Location dialog merges both: the table answers first for its
countries, the geocoder for everything else.

GeoNames has only the first part of Canadian and Irish codes and the UK
outward code (the full codes are proprietary), so a full code resolves to
its area. A code shared by many places (162 in one Scottish district)
becomes a single entry named after its most populous place, so the results
list stays short; the forecast is the same across a code anyway.

Rejected: an online postal code service (Zippopotam, Nominatim, GeoNames'
own web service), which would add a second network dependency, a key or a
usage policy, and could not tell which country a bare "2000" meant; and
embedding every country, which would mean a worldwide geocoder several
megabytes in size for a benefit place-name search already gives. Adding a
country is one line in `tools\Update-PostalCodes.ps1` when someone asks and
GeoNames has its codes at town precision.

## Updates: self-update, deferring to winget

On request, the app fetches the latest GitHub Release, verifies the
downloaded exe against the `SHA256SUMS` asset, and replaces itself. A copy installed by winget will not:
winget owns that folder, so the app should detect the winget install location
and point the user at `winget upgrade` instead.

## Versioning and releases

MinVer stamps the version from the nearest `v*` tag; nothing is hand-edited.
Pushing a tag makes CI build the exe, write `SHA256SUMS`, and draft a GitHub
Release with the matching `CHANGELOG.md` section as notes. Publishing the
draft triggers the winget submission. Releases are therefore reproducible
from a tag with no local build step.

## Official forecast text where it exists, generated text elsewhere

The US National Weather Service and Environment Canada both publish
human-written forecast sentences and real station observations, keyless.
For those countries Weatherspell shows the official text; everywhere else it
writes its own sentences from Open-Meteo's numbers. Two writing styles is the
price of authority: a Canadian reader gets the same words Environment Canada
put on the radio, humidex and all.

The official text is laid over the Open-Meteo forecast rather than replacing
it: Open-Meteo still supplies the hourly data, sunrise and sunset, UV and the
unit conversion, and stands in for anything the service leaves out (a station
that reports no humidity, a page with no current conditions). The official
sentences are shown as written, in the service's own units (the NWS speaks
Fahrenheit whatever the units setting), with one typographic pass so they
read aloud the way the service says them on air: "km/h" and "mph" become
words and "90%" becomes "90 percent". The Sources line at the end names what
was used, and when the official fetch fails the location gets generated
sentences for that refresh with the reason stated there; Open-Meteo failing
still fails the refresh.

Coverage is decided by the geocoded country and then by the service: the NWS
rejects points outside the US, and a Canadian location further than 200 km
from any Environment Canada site gets generated text. Environment Canada's
files live on the MSC Datamart under `today/citypage_weather/{PROV}/{HH}/`
by UTC hour of emission, with only the current day kept, so the latest page
is found by listing the current hour and walking back; in the first minutes
after 00:00 UTC there may be none yet, which is one refresh of generated
text. The site list is fetched once per session, the NWS grid and station
once per location.

Candidates for later: Australia (Bureau of Meteorology open data) and Ireland
(Met Eireann open data), both English. The UK Met Office needs an API key, so
it stays out. Norway, Germany, Japan and others publish only in their respective
languages, worth considering alongside translation efforts.

## Alerts are regional by necessity

No keyless service covers alerts worldwide. Coverage is built one source at
a time: Environment Canada and the NWS first, MeteoAlarm (Europe) next.
A location outside every supported region gets an explicit "alerts are not
available for this region" line, never silence, so nobody mistakes a gap in
coverage for a quiet day.

Rejected: a keyed aggregator (OpenWeatherMap One Call carries alerts
globally) for the reason above: every user would need an account and a key.

## Words, not symbols

"21 degrees", "wind from the southwest at 20 kilometres an hour", "70 percent
chance of rain". Symbols and abbreviations ("21 C", "SW 20 km/h") depend on
how a particular screen reader and its symbol dictionary happen to read
them; words read the same everywhere. Whole degrees only.

Times are the location's own, with the PC's time in brackets when the two
zones differ, so a faraway sunrise reads correctly and a local one is not
cluttered.

## Section jumps are Ctrl+PageDown / Ctrl+PageUp

Not Ctrl+Up / Ctrl+Down: NVDA 2024.1 and later handle those keys itself in
editable text (paragraph navigation, a user setting) and JAWS users expect
the same pair to move by paragraph. Ctrl+PageUp/Down is unbound in edit
controls and in both screen readers.

## Alerts poll every saved location

Polling only the location on screen would miss a warning for home while the
user reads a forecast for somewhere else. Polling all of them costs one small
request per location every ten minutes and a per-location list of alert ids
already seen. Each saved location has its own "notify me" switch.

New alerts are announced through UI Automation notifications while the app
is open (all alerts by default; the threshold is a setting). Tray icon,
Windows toasts and start-with-Windows are deferred features: they add a background
mode whose behaviour deserves its own design pass.

## Updates are checked on request only

No automatic checking at this time. Help > Check for Updates shows the newest
version and its release notes and offers to install.

## Windows scale by font

Every form uses the system message font and scales by font rather than by
DPI alone. Display scale changes the font's pixel size, and the Windows Text
size accessibility setting enlarges the system font without changing the
display scale; scaling by font follows both, so a low-vision user who turns
Text size up gets a window that grows with its text instead of clipping it.
Layouts are auto-sizing wherever possible so they follow the font too, and a
window that would scale past the screen is clamped to the working area.

Per-monitor DPI stays out (see the single-exe section): moving the window to
a display with a different scale gets it bitmap-scaled by Windows, correct
but soft. Revisit if a user with mixed-scale monitors asks for other solutions.
