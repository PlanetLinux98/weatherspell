using System.Net.Http;
using System.Runtime.Serialization;
using System.Xml;
using Weatherspell.Weather.EnvironmentCanada;
using Weatherspell.Weather.Nws;
using Weatherspell.Weather.OpenMeteo;

namespace Weatherspell.Weather;

// One forecast for a location: the Open-Meteo base (hours, days, sun, UV,
// in the user's units) with a national weather service's written periods
// and station observation laid over it where one exists, in 0.1.0 for
// Canada and the United States. Coverage is decided by the geocoded
// country and then by the service accepting the point; when the official
// fetch fails the location gets generated sentences for that refresh and
// the Sources line says why. Open-Meteo failing still fails the refresh.
internal sealed class ForecastService
{
    private readonly OpenMeteoClient _openMeteo;
    private readonly NwsClient _nws = new();
    private readonly CityPageClient _canada = new();

    public ForecastService(OpenMeteoClient openMeteo)
    {
        _openMeteo = openMeteo;
    }

    public async Task<Forecast> GetAsync(Location location, UnitSystem units, int days, CancellationToken cancellationToken)
    {
        var baseTask = _openMeteo.GetForecastAsync(location, units, days, cancellationToken);
        var officialTask = OfficialAsync(location, units, cancellationToken);
        var baseForecast = await baseTask.ConfigureAwait(false);
        var (official, problem) = await officialTask.ConfigureAwait(false);
        return official is null ? WithProblem(baseForecast, problem) : Compose(baseForecast, official);
    }

    private async Task<(OfficialForecast? Official, string? Problem)> OfficialAsync(Location location, UnitSystem units, CancellationToken cancellationToken)
    {
        Task<OfficialForecast>? fetch = location.Country switch
        {
            "Canada" => _canada.GetAsync(location, cancellationToken),
            "United States" => _nws.GetAsync(location, units, cancellationToken),
            _ => null,
        };
        if (fetch is null) return (null, null);
        var name = location.Country == "Canada" ? CityPageClient.SourceName : NwsClient.SourceName;
        try
        {
            return (await fetch.ConfigureAwait(false), null);
        }
        catch (Exception ex) when (ex is HttpRequestException or IOException or InvalidDataException or SerializationException or FormatException or XmlException
                                   || (ex is OperationCanceledException && !cancellationToken.IsCancellationRequested))
        {
            // HttpClient reports its timeout as a cancellation; the token
            // says whether this one was the caller's.
            var reason = ex is OperationCanceledException ? "it took too long to answer" : ex.Message;
            return (null, $"{name}'s forecast text could not be fetched this time ({reason}), so these sentences are written from Open-Meteo data.");
        }
    }

    private static Forecast WithProblem(Forecast f, string? problem) =>
        problem is null ? f : f with { Sources = [problem, .. f.Sources] };

    // Pure, so tests can lay a fixture's official forecast over a base.
    public static Forecast Compose(Forecast f, OfficialForecast official)
    {
        var current = f.Current;
        var sources = new List<string>();
        if (official.Observation is { TemperatureC: not null } o)
        {
            current = Observed(f, o);
            sources.Add($"Forecast text and current conditions: {official.Attribution}, observed at {o.Station}.");
            sources.Add($"Hourly data, sun and UV: {OpenMeteoClient.SourceNote}.");
        }
        else
        {
            sources.Add($"Forecast text: {official.Attribution}.");
            sources.Add($"Current conditions, hourly data, sun and UV: {OpenMeteoClient.SourceNote}.");
        }
        return f with
        {
            Current = current,
            Periods = official.Periods,
            SourceName = $"{official.SourceName} and {OpenMeteoClient.SourceName}",
            Sources = sources,
        };
    }

    // The station's numbers in the user's units; what it did not report
    // (a broken humidity sensor, no gust) keeps the base forecast's value,
    // so the Right now sentence is always whole. Feels-like is only ever
    // the station's own humidex, heat index or wind chill: pairing an
    // observed temperature with a modelled apparent one would mislead.
    private static CurrentConditions Observed(Forecast f, Observation o)
    {
        var b = f.Current;
        var units = f.Units;
        var temperature = Units.FromCelsius(o.TemperatureC!.Value, units);
        var wind = o.WindKmh is double w ? Units.FromKmh(w, units) : b.WindSpeed;
        return new CurrentConditions(
            LocalTime: o.Time.ToOffset(f.UtcOffset).DateTime,
            Temperature: temperature,
            FeelsLike: o.FeelsLikeC is double feels ? Units.FromCelsius(feels, units) : temperature,
            Humidity: o.Humidity ?? b.Humidity,
            WeatherCode: b.WeatherCode,
            IsDay: b.IsDay,
            WindSpeed: wind,
            WindDirection: o.WindDirection ?? b.WindDirection,
            WindGusts: o.WindGustKmh is double g ? Units.FromKmh(g, units) : wind,
            Precipitation: b.Precipitation,
            CloudCover: b.CloudCover,
            PressureHpa: o.PressureHpa ?? b.PressureHpa,
            Station: o.Station,
            Description: o.Description,
            DewPoint: o.DewPointC is double dew ? Units.FromCelsius(dew, units) : null,
            VisibilityMetres: o.VisibilityMetres);
    }
}
