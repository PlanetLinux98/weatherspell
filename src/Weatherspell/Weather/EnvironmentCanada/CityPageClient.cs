using System.Globalization;
using System.Net.Http;
using System.Text.RegularExpressions;
using System.Xml.Linq;

namespace Weatherspell.Weather.EnvironmentCanada;

// Environment Canada's city page XML on the MSC Datamart: the forecast text
// per period, the current conditions and (for #4) the warnings for a site.
// Files land in dd.weather.gc.ca/today/citypage_weather/{PROV}/{HH}/ by UTC
// hour of emission, hourly at a minimum, and only today's tree is kept, so
// the latest file is found by listing the current hour and walking back.
internal sealed class CityPageClient
{
    public const string SourceName = "Environment Canada";
    private const string Base = "https://dd.weather.gc.ca/today/citypage_weather";

    // A site further than this is not this location's forecast.
    public const double MaxSiteDistanceKm = 200;

    private IReadOnlyList<Site>? _sites;
    private readonly SemaphoreSlim _sitesLock = new(1, 1);

    public async Task<OfficialForecast> GetAsync(Location location, CancellationToken cancellationToken)
    {
        var sites = await SitesAsync(cancellationToken).ConfigureAwait(false);
        var (site, distance) = SiteList.Nearest(sites, location.Latitude, location.Longitude);
        if (distance > MaxSiteDistanceKm)
        {
            throw new InvalidDataException($"no Environment Canada forecast site within {MaxSiteDistanceKm:0} kilometres");
        }
        var xml = await LatestCityPageAsync(site, DateTimeOffset.UtcNow, cancellationToken).ConfigureAwait(false);
        return Parse(xml);
    }

    private async Task<IReadOnlyList<Site>> SitesAsync(CancellationToken cancellationToken)
    {
        await _sitesLock.WaitAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            return _sites ??= SiteList.Parse(await Http.GetStringAsync(SiteList.Url, cancellationToken).ConfigureAwait(false));
        }
        finally
        {
            _sitesLock.Release();
        }
    }

    private static async Task<string> LatestCityPageAsync(Site site, DateTimeOffset nowUtc, CancellationToken cancellationToken)
    {
        for (var hour = nowUtc.Hour; hour >= 0; hour--)
        {
            var folder = $"{Base}/{site.Province}/{hour:00}/";
            string listing;
            try
            {
                listing = await Http.GetStringAsync(folder, cancellationToken).ConfigureAwait(false);
            }
            catch (HttpRequestException)
            {
                continue; // the hour's folder is not there yet
            }
            var file = LatestFile(listing, site.Code);
            if (file is not null)
            {
                return await Http.GetStringAsync(folder + file, cancellationToken).ConfigureAwait(false);
            }
        }
        throw new InvalidDataException($"no Environment Canada page for {site.Name} published yet today");
    }

    // File names start with the emission time, so the greatest name is the
    // newest: 20260912T035744.540Z_MSC_CitypageWeather_s0000629_en.xml.
    public static string? LatestFile(string listingHtml, string siteCode)
    {
        string? latest = null;
        foreach (Match m in Regex.Matches(listingHtml, @"href=""(\d{8}T\d{6}(?:\.\d+)?Z_MSC_CitypageWeather_" + Regex.Escape(siteCode) + @"_en\.xml)"""))
        {
            var name = m.Groups[1].Value;
            if (latest is null || string.CompareOrdinal(name, latest) > 0) latest = name;
        }
        return latest;
    }

    public static OfficialForecast Parse(string xml)
    {
        var root = XDocument.Parse(xml).Root ?? throw new InvalidDataException("Empty city page.");
        var location = root.Element("location");
        var siteName = location?.Element("name")?.Value.Trim();
        var region = location?.Element("region")?.Value.Trim();
        var attribution = $"{SourceName} (weather.gc.ca)";
        if (!string.IsNullOrEmpty(region)) attribution += $", forecast for {region}";
        else if (!string.IsNullOrEmpty(siteName)) attribution += $", forecast for {siteName}";

        return new OfficialForecast(SourceName, attribution, ParsePeriods(root), ParseObservation(root));
    }

    private static IReadOnlyList<OfficialPeriod> ParsePeriods(XElement root)
    {
        var group = root.Element("forecastGroup") ?? throw new InvalidDataException("City page has no forecast.");
        var issued = LocalDate(group, "forecastIssue") ?? throw new InvalidDataException("City page forecast has no issue time.");

        // Periods are named, not dated: "Tonight" belongs to the issue date,
        // a weekday name to the first such day from there on, and the names
        // run forward, so each one moves the cursor.
        var list = new List<OfficialPeriod>();
        var cursor = issued;
        foreach (var forecast in group.Elements("forecast"))
        {
            var name = forecast.Element("period")?.Attribute("textForecastName")?.Value.Trim();
            var text = forecast.Element("textSummary")?.Value;
            if (string.IsNullOrEmpty(name) || string.IsNullOrWhiteSpace(text)) continue;
            var date = cursor;
            if (name != "Today" && name != "Tonight" && TryWeekday(name!, out var weekday))
            {
                while (date.DayOfWeek != weekday) date = date.AddDays(1);
                cursor = date;
            }
            list.Add(new OfficialPeriod(name!, date, OfficialText.Spoken(text!)));
        }
        if (list.Count == 0) throw new InvalidDataException("City page forecast has no periods.");
        return list;
    }

    private static bool TryWeekday(string periodName, out DayOfWeek weekday)
    {
        var first = periodName.Split(' ')[0];
        return Enum.TryParse(first, ignoreCase: true, out weekday) && Enum.IsDefined(typeof(DayOfWeek), weekday);
    }

    // The local (non-UTC) dateTime element of the given name, as a date.
    private static DateTime? LocalDate(XElement parent, string name)
    {
        foreach (var dt in parent.Elements("dateTime"))
        {
            if (dt.Attribute("name")?.Value != name || dt.Attribute("zone")?.Value == "UTC") continue;
            if (int.TryParse(dt.Element("year")?.Value, out var y) && int.TryParse(dt.Element("month")?.Value, out var m) && int.TryParse(dt.Element("day")?.Value, out var d))
            {
                return new DateTime(y, m, d);
            }
        }
        return null;
    }

    private static Observation? ParseObservation(XElement root)
    {
        var c = root.Element("currentConditions");
        if (c is null || !c.HasElements) return null;
        var station = c.Element("station")?.Value.Trim();
        var stamp = c.Elements("dateTime").FirstOrDefault(d => d.Attribute("zone")?.Value == "UTC")?.Element("timeStamp")?.Value;
        if (string.IsNullOrEmpty(station) || string.IsNullOrEmpty(stamp)) return null;
        var time = DateTime.ParseExact(stamp, "yyyyMMddHHmmss", CultureInfo.InvariantCulture, DateTimeStyles.AssumeUniversal | DateTimeStyles.AdjustToUniversal);

        var wind = c.Element("wind");
        var condition = c.Element("condition")?.Value.Trim();
        return new Observation(
            Station: station!,
            Time: new DateTimeOffset(time, TimeSpan.Zero),
            Description: string.IsNullOrEmpty(condition) ? null : condition,
            TemperatureC: Number(c.Element("temperature")),
            FeelsLikeC: Number(c.Element("humidex")) ?? Number(c.Element("windChill")),
            Humidity: Number(c.Element("relativeHumidity")) is double h ? (int)Math.Round(h) : null,
            WindKmh: Number(wind?.Element("speed")),
            WindDirection: Number(wind?.Element("bearing")) is double b ? (int)Math.Round(b) : null,
            WindGustKmh: Number(wind?.Element("gust")),
            PressureHpa: Number(c.Element("pressure")) * 10, // kPa
            DewPointC: Number(c.Element("dewpoint")),
            VisibilityMetres: Number(c.Element("visibility")) * 1000); // km
    }

    private static double? Number(XElement? element) =>
        element is not null && double.TryParse(element.Value.Trim(), NumberStyles.Float, CultureInfo.InvariantCulture, out var v) ? v : null;
}
