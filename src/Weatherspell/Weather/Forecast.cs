namespace Weatherspell.Weather;

// Source-neutral forecast: what the prose is written from. Values are already
// in the requested unit system; times are the location's local time
// (DateTimeKind.Unspecified) unless named otherwise.
internal sealed record Forecast(
    Location Location,
    DateTimeOffset FetchedAt,
    TimeSpan UtcOffset,
    UnitSystem Units,
    CurrentConditions Current,
    IReadOnlyList<HourPoint> Hours,
    IReadOnlyList<DayForecast> Days,
    string SourceName,
    string SourceNote);

internal sealed record CurrentConditions(
    DateTime LocalTime,
    double Temperature,
    double FeelsLike,
    int Humidity,
    int WeatherCode,
    bool IsDay,
    double WindSpeed,
    int WindDirection,
    double WindGusts,
    double Precipitation,
    int CloudCover,
    double PressureHpa);

internal sealed record HourPoint(
    DateTime LocalTime,
    double Temperature,
    int? PrecipitationProbability,
    double Precipitation,
    int WeatherCode,
    double WindSpeed,
    int WindDirection,
    double WindGusts,
    double? VisibilityMetres,
    double? DewPoint,
    int? Humidity,
    bool IsDay);

internal sealed record DayForecast(
    DateTime Date,
    int WeatherCode,
    double High,
    double Low,
    double FeelsLikeHigh,
    double FeelsLikeLow,
    DateTime? Sunrise,
    DateTime? Sunset,
    double? DaylightSeconds,
    double? UvIndexMax,
    double PrecipitationSum,
    int? PrecipitationProbabilityMax,
    double SnowfallSum,
    double WindSpeedMax,
    double WindGustsMax,
    int WindDirectionDominant);
