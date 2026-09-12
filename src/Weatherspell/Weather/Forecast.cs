namespace Weatherspell.Weather;

// Source-neutral forecast: what the prose is written from. Values are already
// in the requested unit system; times are the location's local time
// (DateTimeKind.Unspecified) unless named otherwise. Periods is empty when
// there is no official text and the sentences are generated from the hours.
internal sealed record Forecast(
    Location Location,
    DateTimeOffset FetchedAt,
    TimeSpan UtcOffset,
    UnitSystem Units,
    CurrentConditions Current,
    IReadOnlyList<HourPoint> Hours,
    IReadOnlyList<DayForecast> Days,
    IReadOnlyList<OfficialPeriod> Periods,
    string SourceName,
    IReadOnlyList<string> Sources);

// Station and Description are set when a weather service's observation is
// shown: the service's own condition words ("Mist", "Patchy Fog") rather
// than a code; DewPoint and Visibility likewise when observed.
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
    double PressureHpa,
    string? Station = null,
    string? Description = null,
    double? DewPoint = null,
    double? VisibilityMetres = null);

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

// One period of a weather service's written forecast, as the service named
// it ("Tonight", "Saturday", "Saturday night") and worded it. Date is the
// local date the period belongs to: a night belongs to the day it follows.
internal sealed record OfficialPeriod(string Name, DateTime Date, string Text);
