# Design decisions

Why Weatherspell is built the way it is, including the roads not taken, so
they are not re-proposed without new information.

## Rust and wxWidgets

After 0.1.0, Weatherspell is being rewritten in Rust with wxWidgets
(through the wxDragon bindings), Windows first, so that the same code can
later run on macOS and Linux. The released C# app gets fixes only until
the new one replaces it (#24).

wxWidgets drives each system's own controls: Win32 on Windows, AppKit on
macOS, GTK on Linux. Those are the controls screen readers know best, and
most of what the C# app's testing found (see "The C# app") came from a
toolkit layer standing between the screen reader and them. Rust compiles
to one native program with wxWidgets and the C runtime linked in, so the
single exe survives without .NET Framework, and the Mac and Linux
versions need nothing installed either. A first test build on Windows
came to 7.5 MB, using only Windows' own DLLs.

Rejected: modern .NET with a native app for each system (C# and AppKit on
the Mac), and Eto.Forms (C# over each system's controls), which keep C#
but carry .NET inside the Mac and Linux apps or depend on compiling it
ahead of time. Also rejected: toolkits that draw their own controls
(Avalonia, Flutter, Qt Quick, the Rust toolkits built on AccessKit),
whose accessibility is only what the toolkit implements, and where
reading a long read-only text line by line, this app's main control, is
the least proven part; and a web page in a window (Tauri, Electron),
where a screen reader's browse mode and a text that refreshes itself are
an awkward pair.

wxDragon is young. Where it falls short, it gets patched or worked
around; the fallback is the same Rust code with each system's own UI
written directly.

Before anything was ported, a test window checked wxWidgets against the
four problems the C# app's testing found (see "The C# app"), with NVDA
and Narrator, and none of them came back. The menu bar is Windows' own,
with each item's shortcut. The Location box is a Windows combo box, which
NVDA reads while it is arrowed closed. The forecast is a Windows edit
control, which NVDA reads by line and, under the mouse, a line at a time,
and which Narrator reads by line. wxWidgets puts no accessibility layer
of its own over these controls, and that layer is where 0.1's problems
came from. Its dialogs are Windows dialogs too, so About is read on
opening without the role the C# app had to give it.

The forecast is the plain edit control rather than the rich edit control
0.1 moved to for Narrator (#23): with WinForms out of the way, Narrator
reads either one by line, and the plain one avoids a rich edit quirk with
Ctrl+A after Shift+F10.

Two things to avoid, found the same way. wxDragon's accessibility setters
(name, role and the like) replace a window's accessible object, and the
replacement loses the names of the controls inside it, so they are kept
for a control that would otherwise read wrongly, and never used on a
window with controls in it. And announcements go through UI Automation
from the main window without the window presenting itself to UI
Automation: when it did, NVDA switched to reading it through UI
Automation instead of its usual way, which read worse in testing.

Until the Rust app replaces 0.1, it keeps a folder of its own,
`%APPDATA%\Weatherspell Preview`, which its first run fills with a copy
of 0.1's settings and cache. A preview can then never change what 0.1
relies on, and the two can run side by side. The files themselves are
the same: each app reads what the other writes, which the port's tests
check in both directions, so at the switch the Rust app simply moves
into 0.1's folder.

A location's own settings, its nickname and whether its new alerts are
spoken, are in one Edit Location dialog that names the place. In 0.1 the
alert switch sat under the Manage Locations list and acted on whichever
location was selected, which nothing said.

After a section key (Ctrl+PageDown, Ctrl+PageUp, Ctrl+Shift+A), the app
speaks the heading it moved to, since NVDA and JAWS say nothing after a
key they do not know. Narrator reads the new line itself, so with
Narrator as the only screen reader running the app leaves the heading to
it; 0.1 had Narrator say it twice.

## One exe

The whole program is a single `Weatherspell.exe` that runs the moment it is
downloaded, with nothing installed first and nothing beside it. The C# app
gets there with .NET Framework 4.8, which Windows already has (see "The C#
app"); the Rust app by linking everything into the exe.

## Settings live in `%APPDATA%\Weatherspell`

Not beside the exe. Storing next to the exe breaks under `Program Files` and,
under winget, whose upgrade and uninstall delete the whole package folder. A per-user
 AppData folder survives both and works when the exe sits somewhere read-only.
The only potential cost is that settings do not travel with the exe on a USB
stick; a portable-marker mode can be added later if
anyone asks.

## Windows are built in code

Controls are created in code rather than in designer files (`.Designer.cs`
and `.resx` pairs in the C# app). Every accessible name, tab index and
label association is then visible in one readable file and reviewable in a
diff, which matters more here than drag-and-drop layout. The windows are
simple enough that this costs little.

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

## Updates are checked on request only

No automatic checking at this time. Help > Check for Updates fetches the
latest GitHub Release, shows the newest version and its release notes, and
offers to install: it verifies the downloaded exe against the `SHA256SUMS`
asset and replaces itself. A copy installed by winget will not: winget
owns that folder, so the app should detect the winget install location and
point the user at `winget upgrade` instead.

## Versioning and releases

The version comes from the nearest `v*` tag (MinVer in the C# app); nothing
is hand-edited. Pushing a tag makes CI build the exe, write `SHA256SUMS`,
and draft a GitHub Release with the matching `CHANGELOG.md` section as
notes. Publishing the draft triggers the winget submission. Releases are
therefore reproducible from a tag with no local build step.

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

In the US the sentences are the ones the weather.gov forecast page shows,
read from that page's data (`forecast.weather.gov/MapClick.php`, JSON)
rather than the API's `detailedForecast`. Both are written from the same
forecast, but the API has its own sentence generator, which reads worse and
at times says something else: "East wind around 0 mph" where the page says
"Calm wind", "Northeast wind 0 to 5 mph" for "Calm wind becoming north
around 5 mph in the afternoon", and one night's showers "after 8pm" where
the page had "a 50 percent chance of showers after 2am". The page's data is
not a documented API, and the NWS points developers to api.weather.gov, so
the API's text stands in whenever the page's cannot be had or read: the
worst case is the API's wording, never a missing forecast. Rejected:
rewording the API's text in this app, which would fix the zeros but not
the disagreements, and would put this app's words in the service's mouth.

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

## What goes where in the text

Alerts, Right now, Rest of today, each coming day, and the Sources line.
Right now holds everything measured now: the sky, temperature and wind on
one line, then humidity, dew point, pressure, visibility and cloud cover on
another that a listener can move past. The sun and UV close Rest of today
because they are today's; in a section of their own after the coming days
they were read out of time. After sunset the line gives the next sunrise
("The sun set at 7:01 pm and rises at 7:07 am tomorrow"), since that is
what a listener is waiting for then. The UV index is the day's highest, so
it is left out after sunset, and when Environment Canada's own text for
today already gives it.

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

## Settings are a few combo boxes

The intervals are short lists in combo boxes rather than number fields: a
native combo box is the control screen readers read best, there is nothing
to mistype, and a spin box is a composite (in WinForms, an unnamed inner
edit plus buttons) whose reading would need its own testing. A value outside
the list, edited by hand into settings.json, is kept and listed in its
place, so opening the dialog never silently changes it. The dialog has no
accelerator: Windows has no conventional one for a settings dialog (Ctrl+comma
is macOS's), and Alt+S, S is two keys.

## Windows scale by font

Every window uses the system message font and scales by font rather than by
DPI alone. Display scale changes the font's pixel size, and the Windows Text
size accessibility setting enlarges the system font without changing the
display scale; scaling by font follows both, so a low-vision user who turns
Text size up gets a window that grows with its text instead of clipping it.
Layouts are auto-sizing wherever possible so they follow the font too, and a
window that would scale past the screen is clamped to the working area.

The main window reopens where it was closed, and its saved size is kept
with the font's average character size at the time, so after a change of
display scale or Text size it grows or shrinks as the text did, across and
down separately (a font does not grow in proportion: Segoe UI 9 pt is 7 by
15 pixels at 100 percent and 10 by 25 at 150). A saved place where no
screen shows enough of the title bar to take hold of opens centred instead,
and a window overhanging its screen by more than the invisible resize
border is moved back onto it.

The C# app is aware of the main display's scale only: moving its window to
a display with a different scale gets it bitmap-scaled by Windows, correct
but soft (see "The C# app"). The Rust app has no such limit.

## The C# app (0.1)

0.1 is WinForms on .NET Framework 4.8, chosen because 4.8 is part of Windows
10 (1903+) and 11, so the exe was tens of kilobytes and needed nothing
installed. A .NET 10 self-contained single file carried the runtime inside
(about 65 MB and a pause on first launch), and a framework-dependent one
asked users to download .NET. The same constraint ruled out a
`Weatherspell.exe.config`: so no binding redirects and no runtime NuGet
packages, JSON and HTTP from the in-box libraries, and system rather than
per-monitor DPI awareness. It also ruled out a separate Core library, so
the tests reference the exe directly, which keeps the logic in plain
classes and the forms thin. Rejected then: merging a class library into
the exe with ILRepack.

Its screen reader testing found four problems, each a WinForms layer
between the screen reader and Windows' own control, and each a check for
the rewrite:

- **Menus.** WinForms' `MenuStrip` draws its own menus, and on 4.8 its
  accessibility gave screen readers each item's mnemonic letter but never
  its shortcut. The menu bar is the native Win32 one (`MainMenu`), whose
  items read as "Refresh F5" and "Next Section Ctrl+PageDown".
- **A combo box arrowed while collapsed was silent in NVDA.** WinForms'
  objects also speak UI Automation, and NVDA drops MSAA events from a
  window that advertises a UI Automation provider unless it knows the
  window's class. `NativeComboBox` doesn't answer the UI Automation
  request, so it is a plain Win32 combo box to every screen reader.
- **A nudge of the mouse over the forecast read the whole text.** NVDA
  reads the text under the pointer only when the object there identifies
  itself (`IAccIdentity`), which WinForms' objects don't. The forecast box
  hands that request to the control itself.
- **Narrator couldn't read the forecast by line (#23).** A 4.8 TextBox
  reaches UI Automation only as an edit with a value and no text pattern.
  The forecast is a read-only RichEdit (RICHEDIT50W, part of Windows)
  holding plain text, built with "\n" line breaks so its offsets match the
  section keys'. Rejected: writing a text pattern for the TextBox.
