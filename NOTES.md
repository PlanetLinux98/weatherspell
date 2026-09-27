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
set this app uses (labels, lists, buttons, menus) those controls were
already solid. The exception was the text box, which has no UI Automation
text pattern on 4.8, so Narrator could not read it by line; the forecast
uses Windows' own RichEdit instead (see "Two controls with Windows' own
accessibility"). Modern C# syntax still works: PolySharp generates the
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

## Menus are the native Windows menu bar

The menu bar is `MainMenu`, the Win32 menu bar, not the WinForms `MenuStrip`
the designer offers. `MenuStrip` draws its own menus and supplies its own
accessibility objects, and on .NET Framework 4.8 those hand a screen reader
each item's mnemonic letter and never its shortcut, so the menus could not
tell anyone about F5 or Ctrl+PageDown; the popup was even named after the
internal control ("ViewDropDown"). The native menu bar is what every other
Windows program's menus are: `menu bar`, popups named for their menu, items
read as "Refresh F5" and "Next Section Ctrl+PageDown", and the system menu
font, which the Windows Text size setting scales.

Two things follow. `MainMenu`'s `Shortcut` enum has no PageUp/PageDown, so
the section keys are written into the item text after a tab (the native
accelerator column, exposed like any other) and handled by the form itself.
And `MainMenu` exists only on .NET Framework: if the app ever moves to a
newer runtime, where `MenuStrip` has proper UI Automation support, the menu
goes back to `MenuStrip`; it is one block of code in the main form.

Rejected: keeping `MenuStrip` and substituting a custom accessible object
that adds the shortcut. It would still be a managed imitation of a menu,
with its own keyboard and announcement quirks, patched from the outside.

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
The Add Location dialog merges both: the table answers first for its
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

## Coordinates are named by OpenStreetMap

The Add Location field also takes coordinates, in the forms people copy
them: decimals with signs or compass letters, degrees with minutes and
seconds, degrees and decimal minutes, decimal commas, and map links
(`Weather/Coordinates.cs` lists them; its tests hold the examples). There is
no separate coordinates mode: one field, one Enter, one list, as for a
name. Text counts as coordinates only when it can be nothing else, so
postal codes such as "060-0001", "114 55" or "N1" still go to the search.

The point is kept exactly as typed, since the reason to type coordinates
is usually a place that is not the town centre, and it is named after the
place it falls in: "Near Bobcaygeon, Ontario, Canada (44.54 north, 78.54
west)". Open-Meteo's geocoder only goes from names to coordinates, so the
name comes from OpenStreetMap's Nominatim, asked once per search: no key,
and its usage policy allows lookups a user asks for, with a User-Agent
naming the app and at most one request a second. The name matters beyond
the list: its country is what turns on official text and alerts for the
US and Canada, so a lookup that fails is reported as a failure rather than
saved as a point with no country. Where nothing has an address (open sea),
the point is offered under its coordinates, with generated text and no
alerts.

Rejected: a Coordinates button switching the dialog to latitude and
longitude fields, the first plan (a second mode to learn, and a point with
no name or country); naming from the weather services themselves (the NWS
names only US points, and Environment Canada's 800 or so forecast sites are
too sparse to name a cottage); BigDataCloud's free reverse geocoder, which
is only for a device's own location; and GeoNames' web service, which needs
an account.

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
sentences are shown as written, in the location's unit system (see "One
unit system per location"), with one typographic pass so they read aloud
the way the service says them on air: "km/h" and "mph" become
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
coverage for a quiet day. A source that cannot be reached is stated just as
plainly ("Alerts couldn't be checked this time"), for the same reason.

Both sources answer a point query with everything the app shows: the NWS
through `api.weather.gov/alerts/active?point=` and Environment Canada
through the `weather-alerts` collection of MSC GeoMet-OGC-API
(`api.weather.gc.ca`), whose features carry the full English text, the
colour level, the area name, the times and a stable alert id. Alerts are
told apart by that identity rather than by message: the NWS reissues a
message for every update (a new id each time) but keeps the VTEC event
number, and Environment Canada's feature id keeps the alert's number
through its updates, so each event is announced once and expired ones
drop off the seen list on the next check. The NWS returns the same product
twice, once for the forecast zone and once for the county; it is shown once.
Environment Canada's colour levels (yellow, orange, red) set an alert's
severity, with its type as a floor (a warning is at least Severe), so that
"severe and extreme only" means the same on both sides of the border, where
every NWS warning is Severe or Extreme.

Rejected: a keyed aggregator (OpenWeatherMap One Call carries alerts
globally) for the reason above: every user would need an account and a key.
Also rejected, for Canada: reading the city page's `warnings` block and
fetching the matching CAP file from the datamart for the full text. The
city page gives only a headline and a link, the CAP files are filed by
issuing office and hour with nothing in the city page naming the file, so
finding one means listing every office's hourly folder and downloading
candidates; the GeoMet collection answers in one request.

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
is open (all alerts by default; the threshold is a setting), naming the
location and the time the alert ends in the location's own zone. An alert
found by showing a location from the top (launch, switching, adding) is
marked seen silently, since the reader is about to meet it on the first
line; one found by F5 or by the poll is spoken, because the caret may be
anywhere. The seen ids persist in settings, so a relaunch during a long
alert does not announce it again. Tray icon, Windows toasts and
start-with-Windows are deferred features: they add a background mode whose
behaviour deserves its own design pass.

## One unit system per location, and no units setting

The plan had per-quantity unit choices in Settings (Celsius or Fahrenheit,
km/h, mph, m/s or knots, and so on). Dropped before it was built: the
official sentences carry their own numbers, so a reader who chose
Fahrenheit for a Canadian city would get Environment Canada's Celsius in
one paragraph and this app's Fahrenheit in the next. A location's whole
text is therefore in one system, decided by its weather service: Canada is
metric, which is all Environment Canada writes; the United States follows
the Windows region, with the NWS forecast requested in the same system
(`?units=si` gives "High near 16. Southwest wind 7 to 13 km/h."); everywhere
else follows the Windows region, since only this app's own numbers are
involved. The one reader left out is an imperial-minded one looking at
Canada, which is what #14 (converting the official text) is parked for.

## The forecast refreshes itself, and says when it could not

The forecast on screen is fetched again on a timer (30 minutes by default,
a setting) and every saved location's alerts are checked on another (10
minutes). Neither moves focus or the caret: the text is replaced and the
caret put back the same distance into the section with the same heading,
so a reader in the middle of Saturday stays on the same words however much
the Alerts section above grew (`SectionLayout.MapCaret`). F5 and the
alerts rewrite use the same mapping.

A refresh that fails leaves the text on screen rather than replacing it
with an error, and dates it: the first line under Right now becomes
"Showing the forecast from 20 minutes ago; couldn't reach the weather
service". The same line, without a reason, appears on its own once the
text is 30 minutes old (a long interval, or the PC asleep), and there is
no age line at all while the text is fresh; the exact time is in the
status bar. A failure the user asked for (F5, switching) is also spoken
through a notification; the timer's failures are not, since the line is
there when they next read. A once-a-minute tick re-renders the text and
applies the result only when it differs (the age line, a day heading at
midnight) and never while the user has a selection, so a copy in progress
is not lost.

## The cache is shown only after a fetch has failed

Every successful fetch writes the location's forecast and alerts to one
file under `%APPDATA%\Weatherspell\cache\`, named by its coordinates so a
nickname or a rename keeps it. There is one file per saved location,
overwritten each time and never appended to, so the cache is the size of
the saved locations and files for removed locations are pruned at startup.

The obvious use, showing the cached text the moment a location is chosen
and swapping the live text in when it arrives, was rejected: a screen
reader user who has started reading would have the words replaced under
them, on every launch, to save a second or two. So a launch or a switch
shows one line, "Fetching the forecast for Peterborough...", the live text
replaces it when it lands (with a spoken notification, since the
replacement itself makes no sound), and the cache is read only once the
fetch has failed, which is immediate with no network and bounded by the
HTTP timeout otherwise. What is then shown is the cached text with the
same age line a failed refresh produces, and the last known alerts, dated
("showing the alerts from 2 hours ago") and without any whose end has
passed, since "until" would read as still in effect. A failed alert check
on its own gets the same treatment, so an outage never leaves "No alerts
in effect" standing undated.

The live alert check is still awaited when the forecast fetch fails: the
two services are independent, and an alert service that answers gives
live alerts over cached text.

## Two controls with Windows' own accessibility

Since .NET Framework 4.7.3, WinForms answers the accessibility requests
for its controls itself, with objects that also speak UI Automation. NVDA
(the reference screen reader) reads Win32 controls through MSAA, and two of
those objects fall between the two APIs, found with NVDA's own event log on
2026-09-14:

- A combo box arrowed while collapsed was silent. WinForms fires the MSAA
  value change, but NVDA drops MSAA events from any window that advertises
  a UIA provider unless its class is on NVDA's Win32 list; the WinForms
  class normalizes to "COMBOBOX", which is not on it ("Edit" is, so text
  boxes were fine), and the matching UIA selection event is rejected on
  the other side. `NativeComboBox` does not answer the UIA root request, so
  the combo is a plain Win32 combo box to every screen reader.
- A nudge of the mouse over the forecast read the whole text. NVDA reads
  the text under the pointer through its edit-control support only when it
  can identify the object under the pointer as the window's client object
  (`IAccIdentity`), which WinForms' objects do not implement, so it fell
  back to the control's name plus its entire value. The forecast box hands
  the MSAA client request to the control itself, whose own object
  identifies itself; NVDA then reads the paragraph under the pointer. That
  object names the box from the static control just before it among its
  siblings, so each such box sits in a panel with its label first (the
  lint checks the order).

The text to be read, the forecast and an alert's details, is not a
TextBox but a read-only RichTextBox on msftedit's RICHEDIT50W holding
plain text (`ReadingBox`). Narrator reads through UI Automation, and on
this runtime a WinForms TextBox reaches UI Automation only through the MSAA
proxy, as an edit with a value and no text pattern: Narrator read the whole
forecast whenever the box took focus and nothing as the arrows moved
through it (2026-09-25, #23). WinForms' own RichTextBox here is RichEdit20W,
which is no better; RICHEDIT50W answers UI Automation itself, as a document
with a text pattern, and NVDA reads it exactly as it read the TextBox.
msftedit.dll is part of Windows, and RICHEDIT50W is what .NET 10's
RichTextBox uses by default. A text pattern written for the TextBox was the
alternative: ranges, units and bounding rectangles that RichEdit already
implements, and a UI Automation provider on the box that NVDA would then
prefer to the edit support it reads well. RichEdit counts a line break as
one character, so the text is built with "\n" and the offsets for the
section keys and alert lines match the box's own.

The combo box and the forecast box are each one message on one control,
plus the class for the box. Every combo box and every multiline text in the
app is one of these (the lint insists); the single-line search box keeps
WinForms' object, since its label is not a sibling and a short value under
the mouse is harmless.

## Settings are a few combo boxes

The intervals are short lists in combo boxes rather than number fields: a
native combo box is the control screen readers read best, there is nothing
to mistype, and WinForms' spin box is a composite (an unnamed inner edit
plus buttons) whose reading would need its own testing. A value outside
the list, edited by hand into settings.json, is kept and listed in its
place, so opening the dialog never silently changes it. The dialog has no
accelerator: Windows has no conventional one for a settings dialog (Ctrl+comma
is macOS's), and Alt+S, S is two keys.

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
