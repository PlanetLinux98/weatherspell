using System.Globalization;
using System.Runtime.Serialization;
using System.Runtime.Serialization.Json;
using System.Text;
using Weatherspell;
using Weatherspell.Cache;
using Weatherspell.Settings;
using Weatherspell.Weather;
using Weatherspell.Weather.Alerts;
using Weatherspell.Weather.EnvironmentCanada;
using Weatherspell.Weather.Nws;
using Weatherspell.Weather.OpenMeteo;

// Writes what the C# app (0.1) says for each case in the Rust port's
// reference list, so the port's tests compare against 0.1's text for the
// same data. Run from the repository root after changing a case or the
// C# writer:
//
//     dotnet run --project tools\ReferenceText
//
// Every difference the Rust tests then report is a decision for the
// port, not an accident.

var root = args.Length > 0 ? args[0] : Directory.GetCurrentDirectory();
var referenceDir = Path.Combine(root, "crates", "weatherspell-core", "tests", "reference");
var fixtures = Path.Combine(root, "tests", "Weatherspell.Tests", "Fixtures");

Case[] cases;
using (var stream = File.OpenRead(Path.Combine(referenceDir, "cases.json")))
{
    cases = (Case[])new DataContractJsonSerializer(typeof(Case[])).ReadObject(stream);
}

// Settings and cache files go both ways: 0.1's own, for the port to read,
// and 0.1's reading of the port's (written by the port's tests with
// WEATHERSPELL_BLESS set), so moving between the two loses nothing.
var filesDir = Path.Combine(referenceDir, "files");
Directory.CreateDirectory(filesDir);
var scratch = Path.Combine(Path.GetTempPath(), "weatherspell-reference-" + Guid.NewGuid().ToString("N"));
string[] cacheCases = ["ec-peterborough-evening", "alerts-spokane", "alerts-gander"];

foreach (var c in cases)
{
    var p = c.Place!;
    var location = new Location(p.Name!, p.Region, p.Country, p.Latitude, p.Longitude, null);
    var units = c.Units == "imperial" ? UnitSystem.Imperial : UnitSystem.Metric;
    var fetched = DateTimeOffset.Parse(c.Fetched!, CultureInfo.InvariantCulture);
    var now = DateTimeOffset.Parse(c.Now!, CultureInfo.InvariantCulture);
    var forecast = OpenMeteoClient.Parse(File.ReadAllText(Path.Combine(fixtures, c.Fixture!)), location, units, fetched);
    if (c.Official is Official o)
    {
        forecast = ForecastService.Compose(forecast, Read(o));
    }

    var report = c.Alerts is AlertsCase a ? ReadAlerts(a, now) : null;
    var text = Text(c, forecast, report);
    File.WriteAllText(Path.Combine(referenceDir, c.Name + ".txt"), text, new UTF8Encoding(false));
    Console.WriteLine($"{c.Name}: {text.Length} characters");
    if (cacheCases.Contains(c.Name)) CacheBothWays(c, location, forecast, report);
}

SettingsBothWays();
CacheFileNames();
Directory.Delete(scratch, recursive: true);

// The case's text, and for a case with alerts what a reader hears about
// them beyond it: the details dialog, the announcement of new ones, and
// what a switch says.
string Text(Case c, Forecast forecast, AlertReport? report)
{
    var now = DateTimeOffset.Parse(c.Now!, CultureInfo.InvariantCulture);
    var zone = TimeZoneInfo.CreateCustomTimeZone("case", TimeSpan.FromMinutes(c.PcOffsetMinutes), "case", "case");
    var culture = (CultureInfo)CultureInfo.InvariantCulture.Clone();
    culture.DateTimeFormat.AMDesignator = c.Am ?? "";
    culture.DateTimeFormat.PMDesignator = c.Pm ?? "";
    var options = new WriterOptions(now, zone, c.TimePattern!, culture, c.RefreshProblem);

    var text = SectionLayout.Build(ForecastWriter.Write(forecast, options, report)).Text;
    if (report is not null)
    {
        var clock = new Clock(forecast.UtcOffset, zone, c.TimePattern!, culture);
        var nowLocal = now.ToOffset(forecast.UtcOffset).DateTime;
        var gap = new string((char)10, 2);
        foreach (var alert in report.Alerts)
        {
            text += $"Details: {alert.Event}{gap}" + string.Join("", AlertWriter.Details(alert, clock, nowLocal).Select(d => d + gap));
        }
        if (report.Alerts.Count > 0) text += $"Announcement: {AlertWriter.Announcement(c.Place!.Name!, report.Alerts, clock, nowLocal)}{gap}";
        text += $"In effect: {AlertWriter.InEffect(report) ?? "nothing"}{gap}";
    }
    return text;
}

// A case's forecast and alerts through a cache file each way; the port's
// file is read back into the case's own text, which must be unchanged.
void CacheBothWays(Case c, Location location, Forecast forecast, AlertReport? report)
{
    var ours = new ForecastCache(Path.Combine(scratch, c.Name!));
    ours.Save(location, forecast, report);
    File.Copy(Path.Combine(ours.Directory, ForecastCache.FileName(location)), Path.Combine(filesDir, $"cache-{c.Name}-0.1.json"), overwrite: true);

    var port = Path.Combine(filesDir, $"cache-{c.Name}-port.json");
    if (!File.Exists(port))
    {
        Console.WriteLine($"{c.Name}: no cache file from the port yet");
        return;
    }
    var theirs = new ForecastCache(Path.Combine(scratch, c.Name + "-port"));
    Directory.CreateDirectory(theirs.Directory);
    File.Copy(port, Path.Combine(theirs.Directory, ForecastCache.FileName(location)));
    var cached = theirs.Load(location) ?? throw new InvalidDataException($"0.1 could not read {port}");
    File.WriteAllText(Path.Combine(filesDir, $"cache-{c.Name}-port-0.1.txt"), Text(c, cached.Forecast, cached.Alerts), new UTF8Encoding(false));
    Console.WriteLine($"{c.Name}: cache files both ways");
}

// 0.1's settings.json for the sample, and the port's as 0.1 reads and
// saves it again.
void SettingsBothWays()
{
    var ours = new SettingsStore(Path.Combine(scratch, "settings", "settings.json"));
    ours.Save(SampleSettings());
    File.Copy(ours.Path, Path.Combine(filesDir, "settings-0.1.json"), overwrite: true);

    var port = Path.Combine(filesDir, "settings-port.json");
    if (!File.Exists(port))
    {
        Console.WriteLine("settings: no file from the port yet");
        return;
    }
    var theirs = new SettingsStore(Path.Combine(scratch, "port", "settings.json"));
    Directory.CreateDirectory(Path.GetDirectoryName(theirs.Path)!);
    File.Copy(port, theirs.Path);
    var read = theirs.Load();
    if (theirs.LoadProblem is not null) throw new InvalidDataException($"0.1 could not read {port}: {theirs.LoadError}");
    var again = new SettingsStore(Path.Combine(scratch, "again", "settings.json"));
    again.Save(read);
    File.Copy(again.Path, Path.Combine(filesDir, "settings-port-0.1.json"), overwrite: true);
    Console.WriteLine("settings: both ways");
}

// The same settings as the port's reference test builds: a nickname, seen
// alerts, a muted location, a name beyond ASCII, a point named by its
// coordinates, and a window.
AppSettings SampleSettings()
{
    var home = SavedLocation.From(new Location("Peterborough", "Ontario", "Canada", 44.30012, -78.31623, "America/Toronto", "Home"));
    home.SeenAlertIds.Add("ec:64919237566271632202609120507");
    home.SeenAlertIds.Add("nws:KALB.FL.W.0012");
    home.UtcOffsetSeconds = -14400;
    var paris = SavedLocation.From(new Location("Paris", (char)0xCE + "le-de-France", "France", 48.85341, 2.3488, "Europe/Paris"));
    paris.NotifyAlerts = false;
    paris.UtcOffsetSeconds = 7200;
    var point = SavedLocation.From(new Location("44.54 north, 78.54 west", null, null, 44.54, -78.54, null));
    var settings = new AppSettings
    {
        LastLocation = 1,
        ForecastRefreshMinutes = 60,
        AlertCheckMinutes = 5,
        AlertAnnouncements = "severe",
        Window = new SavedWindow { Left = -11, Top = 0, Width = 982, Height = 1019, Maximized = true, CharWidth = 9.92, CharHeight = 25 },
    };
    settings.Locations.AddRange([home, paris, point]);
    return settings;
}

// Cache file names for many points, as .NET Framework's "F4" gives them:
// coordinates whose fifth decimal is a 5 are the ones its rounding
// decides, so most points have one.
void CacheFileNames()
{
    var random = new Random(24);
    string Coordinate(int whole) =>
        (random.Next(2) == 0 ? "-" : "") + random.Next(0, whole) + "." + random.Next(0, 10000).ToString("D4", CultureInfo.InvariantCulture)
        + (random.Next(4) == 0 ? random.Next(0, 100).ToString(CultureInfo.InvariantCulture) : "5");
    var points = new List<(string, string)> { ("-0.00004", "-0.00005"), ("0.00005", "-0.000049999"), ("90", "-180") };
    for (var i = 0; i < 2000; i++) points.Add((Coordinate(90), Coordinate(180)));
    var lines = new StringBuilder();
    foreach (var (lat, lon) in points)
    {
        var l = new Location("x", null, null, double.Parse(lat, CultureInfo.InvariantCulture), double.Parse(lon, CultureInfo.InvariantCulture), null);
        lines.Append(lat).Append(' ').Append(lon).Append(' ').Append(ForecastCache.FileName(l)).Append((char)10);
    }
    File.WriteAllText(Path.Combine(filesDir, "cache-names-0.1.txt"), lines.ToString(), new UTF8Encoding(false));
    Console.WriteLine($"cache names: {points.Count} points");
}

// The coordinate reader over many forms of input (variants of its tests'
// own), one line each: the point, the problem it reports, or "none" when
// the text is left to the place search.
string[] inputs;
using (var stream = File.OpenRead(Path.Combine(referenceDir, "coordinates-input.json")))
{
    inputs = (string[])new DataContractJsonSerializer(typeof(string[])).ReadObject(stream);
}
var readings = new StringBuilder();
for (var i = 0; i < inputs.Length; i++)
{
    string reading;
    try
    {
        var r = Coordinates.Read(inputs[i]);
        reading = r is null ? "none"
            : r.Problem ?? r.Latitude.ToString("F6", CultureInfo.InvariantCulture) + " " + r.Longitude.ToString("F6", CultureInfo.InvariantCulture);
    }
    catch (Exception ex)
    {
        reading = "exception " + ex.GetType().Name;
    }
    readings.Append(i).Append(": ").Append(reading).Append((char)10);
}
File.WriteAllText(Path.Combine(referenceDir, "coordinates-0.1.txt"), readings.ToString(), new UTF8Encoding(false));
Console.WriteLine($"coordinates: {inputs.Length} inputs");

// A national weather service's text and observation, from captured
// responses, as ForecastService would have laid them over the base.
OfficialForecast Read(Official o)
{
    string Fixture(string name) => File.ReadAllText(Path.Combine(fixtures, name));
    if (o.Kind == "ec") return CityPageClient.Parse(Fixture(o.CityPage!));
    var points = NwsClient.ParsePoints(Fixture(o.Points!));
    var station = NwsClient.ParseStations(Fixture(o.Stations!));
    var periods = o.Page is string page ? NwsClient.ParsePagePeriods(Fixture(page)) : NwsClient.ParsePeriods(Fixture(o.Forecast!));
    var observation = o.Observation is string obs ? NwsClient.ParseObservation(Fixture(obs), station.Name) : null;
    return new OfficialForecast(NwsClient.SourceName, points.Attribution, periods, observation);
}

// A service's alerts from a captured response, as AlertService would have
// reported them, or a failed check carrying an earlier one's alerts.
AlertReport ReadAlerts(AlertsCase a, DateTimeOffset now)
{
    var json = File.ReadAllText(Path.Combine(fixtures, a.Fixture!));
    var parsedAt = a.LastChecked is string last ? DateTimeOffset.Parse(last, CultureInfo.InvariantCulture) : now;
    var (alerts, attribution) = a.Kind == "ec"
        ? (AlertsClient.Parse(json, parsedAt, null), AlertsClient.Attribution)
        : (NwsAlertsClient.Parse(json), NwsAlertsClient.Attribution);
    var ordered = AlertService.Order(alerts);
    if (a.Problem is null) return new AlertReport(ordered, attribution, null, now);
    var failed = new AlertReport([], attribution, a.Problem);
    return a.LastChecked is null ? failed : failed.OrLastKnown(new AlertReport(ordered, attribution, null, parsedAt));
}

// Filled by the JSON reader, which the compiler cannot see.
#pragma warning disable CS0649

[DataContract]
internal sealed class Case
{
    [DataMember(Name = "name")] public string? Name;
    [DataMember(Name = "fixture")] public string? Fixture;
    [DataMember(Name = "place")] public Place? Place;
    [DataMember(Name = "units")] public string? Units;
    [DataMember(Name = "fetched")] public string? Fetched;
    [DataMember(Name = "now")] public string? Now;
    [DataMember(Name = "pcOffsetMinutes")] public int PcOffsetMinutes;
    [DataMember(Name = "timePattern")] public string? TimePattern;
    [DataMember(Name = "am")] public string? Am;
    [DataMember(Name = "pm")] public string? Pm;
    [DataMember(Name = "refreshProblem")] public string? RefreshProblem;
    [DataMember(Name = "official")] public Official? Official;
    [DataMember(Name = "alerts")] public AlertsCase? Alerts;
}

[DataContract]
internal sealed class AlertsCase
{
    [DataMember(Name = "kind")] public string? Kind;
    [DataMember(Name = "fixture")] public string? Fixture;
    [DataMember(Name = "problem")] public string? Problem;
    [DataMember(Name = "lastChecked")] public string? LastChecked;
}

[DataContract]
internal sealed class Official
{
    [DataMember(Name = "kind")] public string? Kind;
    [DataMember(Name = "cityPage")] public string? CityPage;
    [DataMember(Name = "points")] public string? Points;
    [DataMember(Name = "stations")] public string? Stations;
    [DataMember(Name = "forecast")] public string? Forecast;
    [DataMember(Name = "page")] public string? Page;
    [DataMember(Name = "observation")] public string? Observation;
}

[DataContract]
internal sealed class Place
{
    [DataMember(Name = "name")] public string? Name;
    [DataMember(Name = "region")] public string? Region;
    [DataMember(Name = "country")] public string? Country;
    [DataMember(Name = "latitude")] public double Latitude;
    [DataMember(Name = "longitude")] public double Longitude;
}
