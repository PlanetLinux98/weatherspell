using System.Globalization;
using System.Text.RegularExpressions;
using Weatherspell.Weather.Alerts;

namespace Weatherspell.Weather.Nws;

// Active NWS alerts for a point (api.weather.gov/alerts/active?point=),
// GeoJSON with the CAP fields in "properties". The same product often comes
// back twice, once for the forecast zone and once for the county, so alerts
// are told apart by their VTEC event (office, phenomenon, significance and
// event number), which also stays the same through the NWS's updates of one
// event; a product with no VTEC (a special weather statement) is known by
// its message id and is announced again when reissued.
internal sealed class NwsAlertsClient
{
    // As spoken mid-sentence ("from the National Weather Service").
    public const string SpokenSource = "the " + NwsClient.SourceName;
    public const string Attribution = "National Weather Service (weather.gov)";
    private const string Endpoint = "https://api.weather.gov/alerts/active";

    public async Task<IReadOnlyList<WeatherAlert>> GetAsync(Location location, CancellationToken cancellationToken)
    {
        var inv = CultureInfo.InvariantCulture;
        var url = $"{Endpoint}?point={location.Latitude.ToString("0.####", inv)},{location.Longitude.ToString("0.####", inv)}&status=actual";
        return Parse(await Http.GetStringAsync(url, cancellationToken).ConfigureAwait(false));
    }

    public static IReadOnlyList<WeatherAlert> Parse(string json)
    {
        var list = new List<WeatherAlert>();
        var seen = new HashSet<string>(StringComparer.Ordinal);
        foreach (var feature in Json.Read<NwsAlertsResponse>(json).Features ?? [])
        {
            var a = feature.Properties;
            if (a is null || string.IsNullOrWhiteSpace(a.Event) || string.IsNullOrEmpty(a.Id) || string.IsNullOrEmpty(a.Sent)) continue;
            if (a.Status is not null && a.Status != "Actual") continue;
            var sent = DateTimeOffset.Parse(a.Sent, CultureInfo.InvariantCulture, DateTimeStyles.None);
            var id = Identity(a, sent);
            if (!seen.Add(id)) continue;
            list.Add(new WeatherAlert(
                Id: id,
                Event: OfficialText.SentenceCase(a.Event!),
                Severity: Severity(a.Severity),
                Issued: sent,
                Onset: Time(a.Onset),
                Ends: Time(a.Ends) ?? Time(a.Expires),
                Source: SpokenSource,
                Sender: string.IsNullOrWhiteSpace(a.SenderName) ? NwsClient.SourceName : a.SenderName!.Trim(),
                Area: string.IsNullOrWhiteSpace(a.AreaDesc) ? "" : a.AreaDesc!.Trim(),
                Level: null,
                Description: a.Description ?? "",
                Instruction: string.IsNullOrWhiteSpace(a.Instruction) ? null : a.Instruction,
                Url: ProductUrl(a)));
        }
        return list;
    }

    // "/O.NEW.KOTX.FF.A.0001.260913T0900Z-260913T2300Z/": the four middle
    // fields name the event; the number restarts each year.
    private static string Identity(NwsAlert a, DateTimeOffset sent)
    {
        var vtec = First(a.Parameters, "VTEC");
        if (vtec is not null)
        {
            var m = Regex.Match(vtec, @"^/[A-Z]\.[A-Z]{3}\.([A-Z]{4}\.[A-Z]{2}\.[A-Z]\.\d{4})\.");
            if (m.Success) return $"nws:{m.Groups[1].Value}.{sent.Year}";
        }
        return "nws:" + a.Id;
    }

    private static AlertSeverity Severity(string? word) => word switch
    {
        "Extreme" => AlertSeverity.Extreme,
        "Severe" => AlertSeverity.Severe,
        "Moderate" => AlertSeverity.Moderate,
        "Minor" => AlertSeverity.Minor,
        _ => AlertSeverity.Unknown,
    };

    // The office's own text page for the product: the AWIPS identifier ends
    // in the office code ("FFAOTX" is Spokane's flash flood watch).
    private static string? ProductUrl(NwsAlert a)
    {
        var awips = First(a.Parameters, "AWIPSidentifier");
        if (awips is null || awips.Length < 5) return null;
        var office = awips.Substring(awips.Length - 3);
        return $"https://forecast.weather.gov/wwamap/wwatxtget.php?cwa={office}&wwa={Uri.EscapeDataString(a.Event!.ToLowerInvariant())}";
    }

    private static string? First(Dictionary<string, string[]>? parameters, string key) =>
        parameters is not null && parameters.TryGetValue(key, out var values) && values is { Length: > 0 } ? values[0] : null;

    private static DateTimeOffset? Time(string? text) =>
        string.IsNullOrEmpty(text) ? null : DateTimeOffset.Parse(text, CultureInfo.InvariantCulture, DateTimeStyles.None);
}
