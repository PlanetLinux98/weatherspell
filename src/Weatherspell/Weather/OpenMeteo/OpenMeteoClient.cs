using System.Globalization;

namespace Weatherspell.Weather.OpenMeteo;

// Forecast and geocoding from Open-Meteo (no key; non-commercial fair use;
// CC BY 4.0). Values are requested in the user's units so nothing is
// converted here; times come back in the location's own zone.
internal sealed class OpenMeteoClient
{
    public const string SourceName = "Open-Meteo";
    public const string SourceNote = "Open-Meteo (open-meteo.com), licensed CC BY 4.0";

    private const string ForecastEndpoint = "https://api.open-meteo.com/v1/forecast";
    private const string GeocodingEndpoint = "https://geocoding-api.open-meteo.com/v1/search";

    private const string CurrentFields =
        "temperature_2m,relative_humidity_2m,apparent_temperature,is_day,precipitation,weather_code,cloud_cover,wind_speed_10m,wind_direction_10m,wind_gusts_10m,pressure_msl";
    private const string HourlyFields =
        "temperature_2m,precipitation_probability,precipitation,weather_code,wind_speed_10m,wind_direction_10m,wind_gusts_10m,visibility,dew_point_2m,relative_humidity_2m,is_day";
    private const string DailyFields =
        "weather_code,temperature_2m_max,temperature_2m_min,apparent_temperature_max,apparent_temperature_min,sunrise,sunset,daylight_duration,uv_index_max,precipitation_sum,precipitation_probability_max,snowfall_sum,wind_speed_10m_max,wind_gusts_10m_max,wind_direction_10m_dominant";

    public static string ForecastUrl(Location location, UnitSystem units, int days)
    {
        var inv = CultureInfo.InvariantCulture;
        return ForecastEndpoint
            + "?latitude=" + location.Latitude.ToString("0.####", inv)
            + "&longitude=" + location.Longitude.ToString("0.####", inv)
            + "&current=" + CurrentFields
            + "&hourly=" + HourlyFields
            + "&daily=" + DailyFields
            + "&forecast_days=" + days.ToString(inv)
            + "&timezone=auto"
            + "&temperature_unit=" + Units.TemperatureParameter(units)
            + "&wind_speed_unit=" + Units.WindParameter(units)
            + "&precipitation_unit=" + Units.PrecipitationParameter(units);
    }

    public async Task<Forecast> GetForecastAsync(Location location, UnitSystem units, int days, CancellationToken cancellationToken)
    {
        var json = await Http.GetStringAsync(ForecastUrl(location, units, days), cancellationToken).ConfigureAwait(false);
        return Parse(json, location, units, DateTimeOffset.UtcNow);
    }

    public async Task<IReadOnlyList<Location>> SearchAsync(string query, CancellationToken cancellationToken)
    {
        var url = GeocodingEndpoint + "?name=" + Uri.EscapeDataString(query.Trim()) + "&count=20&language=en&format=json";
        var json = await Http.GetStringAsync(url, cancellationToken).ConfigureAwait(false);
        return ParseSearch(json);
    }

    public static IReadOnlyList<Location> ParseSearch(string json)
    {
        var response = Json.Read<GeocodingResponse>(json);
        var list = new List<Location>();
        foreach (var r in response.Results ?? [])
        {
            if (string.IsNullOrWhiteSpace(r.Name)) continue;
            list.Add(new Location(r.Name!, r.Admin1, r.Country, r.Latitude, r.Longitude, r.Timezone, Population: r.Population));
        }
        return list;
    }

    // Pure: fixtures feed this directly in tests.
    public static Forecast Parse(string json, Location location, UnitSystem units, DateTimeOffset fetchedAt)
    {
        var r = Json.Read<ForecastResponse>(json);
        var c = r.Current ?? throw new InvalidDataException("Open-Meteo response has no current block.");
        var h = r.Hourly ?? throw new InvalidDataException("Open-Meteo response has no hourly block.");
        var d = r.Daily ?? throw new InvalidDataException("Open-Meteo response has no daily block.");

        var current = new CurrentConditions(
            ParseTime(c.Time),
            c.Temperature ?? 0,
            c.ApparentTemperature ?? c.Temperature ?? 0,
            (int)Math.Round(c.Humidity ?? 0),
            c.WeatherCode ?? 0,
            (c.IsDay ?? 1) == 1,
            c.WindSpeed ?? 0,
            (int)Math.Round(c.WindDirection ?? 0),
            c.WindGusts ?? c.WindSpeed ?? 0,
            c.Precipitation ?? 0,
            (int)Math.Round(c.CloudCover ?? 0),
            c.PressureMsl ?? 0);

        var hours = new List<HourPoint>();
        var times = h.Time ?? [];
        for (var i = 0; i < times.Length; i++)
        {
            var temp = At(h.Temperature, i);
            if (temp is null) continue; // past the model's horizon
            hours.Add(new HourPoint(
                ParseTime(times[i]),
                temp.Value,
                AtInt(h.PrecipitationProbability, i),
                At(h.Precipitation, i) ?? 0,
                AtInt(h.WeatherCode, i) ?? 0,
                At(h.WindSpeed, i) ?? 0,
                AtInt(h.WindDirection, i) ?? 0,
                At(h.WindGusts, i) ?? At(h.WindSpeed, i) ?? 0,
                At(h.Visibility, i),
                At(h.DewPoint, i),
                AtInt(h.Humidity, i),
                (AtInt(h.IsDay, i) ?? 1) == 1));
        }

        var days = new List<DayForecast>();
        var dates = d.Time ?? [];
        for (var i = 0; i < dates.Length; i++)
        {
            var high = At(d.TemperatureMax, i);
            var low = At(d.TemperatureMin, i);
            if (high is null || low is null) continue;
            days.Add(new DayForecast(
                ParseDate(dates[i]),
                AtInt(d.WeatherCode, i) ?? 0,
                high.Value,
                low.Value,
                At(d.ApparentMax, i) ?? high.Value,
                At(d.ApparentMin, i) ?? low.Value,
                ParseOptionalTime(AtString(d.Sunrise, i)),
                ParseOptionalTime(AtString(d.Sunset, i)),
                At(d.DaylightDuration, i),
                At(d.UvIndexMax, i),
                At(d.PrecipitationSum, i) ?? 0,
                AtInt(d.PrecipitationProbabilityMax, i),
                At(d.SnowfallSum, i) ?? 0,
                At(d.WindSpeedMax, i) ?? 0,
                At(d.WindGustsMax, i) ?? 0,
                AtInt(d.WindDirectionDominant, i) ?? 0));
        }

        return new Forecast(
            location,
            fetchedAt,
            TimeSpan.FromSeconds(r.UtcOffsetSeconds),
            units,
            current,
            hours,
            days,
            Periods: [],
            SourceName,
            [$"Forecast and current conditions: {SourceNote}."]);
    }

    private static double? At(double?[]? values, int i) =>
        values is not null && i < values.Length ? values[i] : null;

    private static int? AtInt(double?[]? values, int i)
    {
        var v = At(values, i);
        return v is null ? null : (int)Math.Round(v.Value);
    }

    private static string? AtString(string?[]? values, int i) =>
        values is not null && i < values.Length ? values[i] : null;

    private static DateTime ParseTime(string? value) =>
        ParseOptionalTime(value) ?? throw new InvalidDataException("Open-Meteo time missing.");

    private static DateTime? ParseOptionalTime(string? value)
    {
        if (string.IsNullOrEmpty(value)) return null;
        return DateTime.ParseExact(value, "yyyy-MM-dd'T'HH:mm", CultureInfo.InvariantCulture, DateTimeStyles.None);
    }

    private static DateTime ParseDate(string value) =>
        DateTime.ParseExact(value, "yyyy-MM-dd", CultureInfo.InvariantCulture, DateTimeStyles.None);
}
