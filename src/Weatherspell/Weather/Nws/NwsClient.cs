using System.Globalization;
using System.Net.Http;

namespace Weatherspell.Weather.Nws;

// The US National Weather Service API (api.weather.gov): no key, but a
// User-Agent naming the app and a contact (Http sets it). points/{lat},{lon}
// resolves a location to a forecast grid and its nearby stations, and 404s
// outside the US, which is how coverage is decided; the grid's forecast
// carries the written periods, the nearest station the latest observation.
internal sealed class NwsClient
{
    public const string SourceName = "National Weather Service";
    private const string Endpoint = "https://api.weather.gov";

    // Grid and station per location for the session: the points lookup never
    // changes, and the NWS asks that it not be repeated needlessly.
    private readonly Dictionary<string, NwsGrid> _grids = new(StringComparer.Ordinal);
    private readonly SemaphoreSlim _gridLock = new(1, 1);

    public async Task<OfficialForecast> GetAsync(Location location, UnitSystem units, CancellationToken cancellationToken)
    {
        var grid = await GridAsync(location, cancellationToken).ConfigureAwait(false);
        var forecastTask = Http.GetStringAsync(ForecastUrl(grid.ForecastUrl, units), cancellationToken);
        var observationTask = LatestObservationAsync(grid.StationId, cancellationToken);
        var periods = ParsePeriods(await forecastTask.ConfigureAwait(false));
        var observationJson = await observationTask.ConfigureAwait(false);
        var observation = observationJson is null ? null : ParseObservation(observationJson, grid.StationName);
        return new OfficialForecast(SourceName, grid.Attribution, periods, observation);
    }

    private async Task<NwsGrid> GridAsync(Location location, CancellationToken cancellationToken)
    {
        var inv = CultureInfo.InvariantCulture;
        var key = location.Latitude.ToString("0.####", inv) + "," + location.Longitude.ToString("0.####", inv);
        await _gridLock.WaitAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            if (_grids.TryGetValue(key, out var known)) return known;
            var points = ParsePoints(await Http.GetStringAsync($"{Endpoint}/points/{key}", cancellationToken).ConfigureAwait(false));
            var stations = ParseStations(await Http.GetStringAsync(points.StationsUrl, cancellationToken).ConfigureAwait(false));
            var grid = new NwsGrid(points.ForecastUrl, points.Attribution, stations.Id, stations.Name);
            _grids[key] = grid;
            return grid;
        }
        finally
        {
            _gridLock.Release();
        }
    }

    // A station that is down should not cost the forecast text: the base
    // forecast's current conditions stand in, and the Sources line says so.
    private static async Task<string?> LatestObservationAsync(string? stationId, CancellationToken cancellationToken)
    {
        if (stationId is null) return null;
        try
        {
            return await Http.GetStringAsync($"{Endpoint}/stations/{Uri.EscapeDataString(stationId)}/observations/latest", cancellationToken).ConfigureAwait(false);
        }
        catch (Exception ex) when (ex is HttpRequestException || (ex is OperationCanceledException && !cancellationToken.IsCancellationRequested))
        {
            return null;
        }
    }

    // The grid's forecast text is written in US units unless asked for SI
    // ("High near 16. Southwest wind 7 to 13 km/h."), which keeps a metric
    // reader's whole text in one system.
    public static string ForecastUrl(string gridForecastUrl, UnitSystem units) =>
        units == UnitSystem.Metric ? gridForecastUrl + "?units=si" : gridForecastUrl;

    internal sealed record NwsGrid(string ForecastUrl, string Attribution, string? StationId, string? StationName);

    public static (string ForecastUrl, string StationsUrl, string Attribution) ParsePoints(string json)
    {
        var p = Json.Read<NwsPointsResponse>(json).Properties ?? throw new InvalidDataException("NWS points response has no properties.");
        if (string.IsNullOrEmpty(p.Forecast) || string.IsNullOrEmpty(p.ObservationStations))
        {
            throw new InvalidDataException("NWS points response names no forecast grid.");
        }
        var near = p.RelativeLocation?.Properties;
        var attribution = $"{SourceName} (weather.gov)";
        if (near is not null && !string.IsNullOrEmpty(near.City))
        {
            attribution += string.IsNullOrEmpty(near.State) ? $", forecast for {near.City}" : $", forecast for {near.City}, {near.State}";
        }
        return (p.Forecast!, p.ObservationStations!, attribution);
    }

    // The list is nearest first; the first station with an identifier is used.
    public static (string? Id, string? Name) ParseStations(string json)
    {
        foreach (var f in Json.Read<NwsStationsResponse>(json).Features ?? [])
        {
            if (!string.IsNullOrEmpty(f.Properties?.StationIdentifier))
            {
                return (f.Properties!.StationIdentifier, f.Properties.Name);
            }
        }
        return (null, null);
    }

    public static IReadOnlyList<OfficialPeriod> ParsePeriods(string json)
    {
        var p = Json.Read<NwsForecastResponse>(json).Properties ?? throw new InvalidDataException("NWS forecast has no properties.");
        var list = new List<OfficialPeriod>();
        foreach (var period in p.Periods ?? [])
        {
            if (string.IsNullOrWhiteSpace(period.Name) || string.IsNullOrWhiteSpace(period.DetailedForecast) || string.IsNullOrEmpty(period.StartTime)) continue;
            // The offset in startTime is the location's own, so its date is
            // the local date; a night period starts in the evening of its day.
            var start = DateTimeOffset.Parse(period.StartTime, CultureInfo.InvariantCulture, DateTimeStyles.None);
            list.Add(new OfficialPeriod(PeriodName(period.Name!), start.Date, OfficialText.Spoken(period.DetailedForecast!)));
        }
        if (list.Count == 0) throw new InvalidDataException("NWS forecast has no periods.");
        return list;
    }

    // "Saturday Night" and "This Afternoon" as Environment Canada and this
    // app's own headings case them.
    private static string PeriodName(string name)
    {
        var n = name.Trim();
        if (n.EndsWith(" Night", StringComparison.Ordinal)) n = n.Substring(0, n.Length - 6) + " night";
        if (n == "This Afternoon") n = "This afternoon";
        return n;
    }

    public static Observation ParseObservation(string json, string? stationName)
    {
        var o = Json.Read<NwsObservationResponse>(json).Properties ?? throw new InvalidDataException("NWS observation has no properties.");
        if (string.IsNullOrEmpty(o.Timestamp)) throw new InvalidDataException("NWS observation has no timestamp.");
        var time = DateTimeOffset.Parse(o.Timestamp, CultureInfo.InvariantCulture, DateTimeStyles.None);
        var pressurePa = o.SeaLevelPressure?.Value ?? o.BarometricPressure?.Value;
        return new Observation(
            Station: stationName ?? o.StationName ?? "the nearest station",
            Time: time,
            Description: string.IsNullOrWhiteSpace(o.TextDescription) ? null : o.TextDescription!.Trim(),
            TemperatureC: o.Temperature?.Value,
            FeelsLikeC: o.HeatIndex?.Value ?? o.WindChill?.Value,
            Humidity: o.RelativeHumidity?.Value is double h ? (int)Math.Round(h) : null,
            WindKmh: o.WindSpeed?.Value,
            WindDirection: o.WindDirection?.Value is double d ? (int)Math.Round(d) : null,
            WindGustKmh: o.WindGust?.Value,
            PressureHpa: pressurePa / 100,
            DewPointC: o.Dewpoint?.Value,
            VisibilityMetres: o.Visibility?.Value);
    }
}
