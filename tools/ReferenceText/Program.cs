using System.Globalization;
using System.Runtime.Serialization;
using System.Runtime.Serialization.Json;
using System.Text;
using Weatherspell;
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

    var zone = TimeZoneInfo.CreateCustomTimeZone("case", TimeSpan.FromMinutes(c.PcOffsetMinutes), "case", "case");
    var culture = (CultureInfo)CultureInfo.InvariantCulture.Clone();
    culture.DateTimeFormat.AMDesignator = c.Am ?? "";
    culture.DateTimeFormat.PMDesignator = c.Pm ?? "";
    var options = new WriterOptions(now, zone, c.TimePattern!, culture, c.RefreshProblem);

    var report = c.Alerts is AlertsCase a ? ReadAlerts(a, now) : null;
    var text = SectionLayout.Build(ForecastWriter.Write(forecast, options, report)).Text;
    if (report is not null)
    {
        // What a reader hears about the alerts beyond the text: the details
        // dialog, the announcement of new ones, and what a switch says.
        var clock = new Clock(forecast.UtcOffset, zone, c.TimePattern!, culture);
        var nowLocal = now.ToOffset(forecast.UtcOffset).DateTime;
        var gap = new string((char)10, 2);
        foreach (var alert in report.Alerts)
        {
            text += $"Details: {alert.Event}{gap}" + string.Join("", AlertWriter.Details(alert, clock, nowLocal).Select(d => d + gap));
        }
        if (report.Alerts.Count > 0) text += $"Announcement: {AlertWriter.Announcement(p.Name!, report.Alerts, clock, nowLocal)}{gap}";
        text += $"In effect: {AlertWriter.InEffect(report) ?? "nothing"}{gap}";
    }
    File.WriteAllText(Path.Combine(referenceDir, c.Name + ".txt"), text, new UTF8Encoding(false));
    Console.WriteLine($"{c.Name}: {text.Length} characters");
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
