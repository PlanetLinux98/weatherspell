using System.Globalization;
using System.Runtime.Serialization;
using System.Runtime.Serialization.Json;
using System.Text;
using Weatherspell;
using Weatherspell.Weather;
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

    var text = SectionLayout.Build(ForecastWriter.Write(forecast, options)).Text;
    File.WriteAllText(Path.Combine(referenceDir, c.Name + ".txt"), text, new UTF8Encoding(false));
    Console.WriteLine($"{c.Name}: {text.Length} characters");
}

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
