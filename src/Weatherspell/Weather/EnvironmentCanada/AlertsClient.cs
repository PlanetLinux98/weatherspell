using System.Globalization;
using Weatherspell.Weather.Alerts;

namespace Weatherspell.Weather.EnvironmentCanada;

// Environment Canada's alerts in effect at a point, from the weather-alerts
// collection of MSC GeoMet-OGC-API: one request gives each alert with its
// full English text, colour level, area and times, which the city page's
// warnings block (a headline and a link) and the CAP files on the datamart
// (one file per message, findable only by walking every office's hourly
// folder) could not do between them. The feature id is "{alert}_{area}":
// the alert part stays the same through EC's updates of one alert, so it
// is the identity.
internal sealed class AlertsClient
{
    public const string SourceName = CityPageClient.SourceName;
    public const string Attribution = "Environment Canada (weather.gc.ca)";
    private const string Endpoint = "https://api.weather.gc.ca/collections/weather-alerts/items";

    public async Task<IReadOnlyList<WeatherAlert>> GetAsync(Location location, DateTimeOffset now, CancellationToken cancellationToken)
    {
        var inv = CultureInfo.InvariantCulture;
        var lat = location.Latitude.ToString("0.####", inv);
        var lon = location.Longitude.ToString("0.####", inv);
        // A point-sized bbox: the polygons are forecast regions, and the
        // geometry itself (megabytes for a province-wide alert) is not needed.
        var url = $"{Endpoint}?f=json&limit=100&skipGeometry=true&bbox={lon},{lat},{lon},{lat}";
        return Parse(await Http.GetStringAsync(url, cancellationToken).ConfigureAwait(false), now, LocationUrl(location));
    }

    // The location page on weather.gc.ca lists the same alerts with links to
    // each one's full report.
    private static string LocationUrl(Location location)
    {
        var inv = CultureInfo.InvariantCulture;
        return $"https://weather.gc.ca/en/location/index.html?coords={location.Latitude.ToString("0.###", inv)},{location.Longitude.ToString("0.###", inv)}";
    }

    public static IReadOnlyList<WeatherAlert> Parse(string json, DateTimeOffset now, string? url)
    {
        var list = new List<WeatherAlert>();
        var seen = new HashSet<string>(StringComparer.Ordinal);
        foreach (var feature in Json.Read<EcAlertsResponse>(json).Features ?? [])
        {
            var a = feature.Properties;
            if (a is null || string.IsNullOrWhiteSpace(a.Name) || string.IsNullOrEmpty(feature.Id) || string.IsNullOrEmpty(a.Published)) continue;
            // The collection is meant to hold only alerts in effect; an
            // ended or stale one is dropped in case it lingers.
            if (a.Status == "ended") continue;
            var expires = Time(a.Expires);
            if (expires is not null && expires < now) continue;
            var id = "ec:" + feature.Id!.Split('_')[0];
            if (!seen.Add(id)) continue;
            list.Add(new WeatherAlert(
                Id: id,
                Event: OfficialText.SentenceCase(a.Name!),
                Severity: Severity(a.Colour, a.AlertType),
                Issued: Time(a.Published)!.Value,
                Onset: Time(a.Valid),
                Ends: Time(a.EventEnd) ?? expires,
                Source: SourceName,
                Sender: SourceName,
                Area: string.IsNullOrWhiteSpace(a.Area) ? "" : a.Area!.Trim(),
                Level: Level(a),
                Description: a.Text ?? "",
                Instruction: null,
                Url: url));
        }
        return list;
    }

    // EC's colour levels (yellow: be aware; orange: be prepared; red: take
    // action) set the severity, and the alert type is a floor under them:
    // a warning is at least Severe whatever its colour, so that the "severe
    // and extreme only" announcement setting means the same on both sides
    // of the border, where every NWS warning is Severe or Extreme.
    public static AlertSeverity Severity(string? colour, string? type)
    {
        var byColour = colour?.ToLowerInvariant() switch
        {
            "red" => AlertSeverity.Extreme,
            "orange" => AlertSeverity.Severe,
            "yellow" => AlertSeverity.Moderate,
            _ => AlertSeverity.Unknown,
        };
        var byType = type?.ToLowerInvariant() switch
        {
            "warning" => AlertSeverity.Severe,
            "watch" => AlertSeverity.Moderate,
            "advisory" => AlertSeverity.Minor,
            "statement" => AlertSeverity.Minor,
            _ => AlertSeverity.Unknown,
        };
        return byColour > byType ? byColour : byType;
    }

    // "Yellow level, moderate impact, high confidence": EC's tiered ranking
    // as the public alert pages state it, when the alert carries one.
    private static string? Level(EcAlert a)
    {
        if (string.IsNullOrWhiteSpace(a.Colour)) return null;
        var s = $"{OfficialText.SentenceCase(a.Colour!)} level";
        if (!string.IsNullOrWhiteSpace(a.Impact)) s += $", {a.Impact!.Trim().ToLowerInvariant()} impact";
        if (!string.IsNullOrWhiteSpace(a.Confidence)) s += $", {a.Confidence!.Trim().ToLowerInvariant()} confidence";
        return s;
    }

    private static DateTimeOffset? Time(string? text) =>
        string.IsNullOrEmpty(text) ? null : DateTimeOffset.Parse(text, CultureInfo.InvariantCulture, DateTimeStyles.AssumeUniversal);
}
