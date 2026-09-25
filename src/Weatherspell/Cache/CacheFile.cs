using System.Globalization;
using System.Runtime.Serialization;
using Weatherspell.Weather;
using Weatherspell.Weather.Alerts;

namespace Weatherspell.Cache;

// The shape of one cache file: mirrors of the forecast and alert records
// that the in-box serializer can build (it needs settable fields and a
// parameterless constructor, which positional records lack). The location
// is not stored; the file is keyed by coordinates and read back for the
// saved location that asked. Times are ISO 8601 text: the serializer's own
// date form goes through the PC's time zone, which would shift a
// location's wall-clock times across a DST change and read them back with
// the wrong Kind. A member the file lacks reads as null and fails To(), so
// a truncated or foreign file is treated as no cache.
[DataContract]
internal sealed class CacheFile
{
    public const int CurrentVersion = 1;

    [DataMember(Name = "version")] public int Version = CurrentVersion;
    [DataMember(Name = "forecast")] public ForecastData? Forecast;
    // The last alert check that succeeded, or the NotAvailable report for a
    // region no source covers; never a failed check.
    [DataMember(Name = "alerts")] public AlertsData? Alerts;
}

internal static class Iso
{
    private const string Wall = "yyyy-MM-dd'T'HH:mm:ss";

    // A location's wall-clock time, Kind Unspecified both ways.
    public static string Write(DateTime t) => t.ToString(Wall, CultureInfo.InvariantCulture);
    public static string? Write(DateTime? t) => t is DateTime v ? Write(v) : null;
    public static DateTime ReadWall(string? s) => DateTime.ParseExact(Required(s, "time"), Wall, CultureInfo.InvariantCulture, DateTimeStyles.None);
    public static DateTime? ReadWallOrNull(string? s) => s is null ? null : ReadWall(s);

    // An instant with its offset, kept exactly.
    public static string Write(DateTimeOffset t) => t.ToString("o", CultureInfo.InvariantCulture);
    public static string? Write(DateTimeOffset? t) => t is DateTimeOffset v ? Write(v) : null;
    public static DateTimeOffset ReadStamp(string? s) => DateTimeOffset.ParseExact(Required(s, "time"), "o", CultureInfo.InvariantCulture, DateTimeStyles.None);
    public static DateTimeOffset? ReadStampOrNull(string? s) => s is null ? null : ReadStamp(s);

    public static T Required<T>(T? value, string name) where T : class =>
        value ?? throw new InvalidDataException($"The cache file has no {name}.");
}

[DataContract]
internal sealed class ForecastData
{
    [DataMember(Name = "fetchedAt")] public string? FetchedAt;
    [DataMember(Name = "utcOffsetSeconds")] public int UtcOffsetSeconds;
    [DataMember(Name = "units")] public string? Units;
    [DataMember(Name = "current")] public CurrentData? Current;
    [DataMember(Name = "hours")] public List<HourData>? Hours;
    [DataMember(Name = "days")] public List<DayData>? Days;
    [DataMember(Name = "periods")] public List<PeriodData>? Periods;
    [DataMember(Name = "sourceName")] public string? SourceName;
    [DataMember(Name = "sources")] public List<string>? Sources;

    public static ForecastData From(Forecast f) => new()
    {
        FetchedAt = Iso.Write(f.FetchedAt),
        UtcOffsetSeconds = (int)f.UtcOffset.TotalSeconds,
        Units = f.Units == UnitSystem.Imperial ? "imperial" : "metric",
        Current = CurrentData.From(f.Current),
        Hours = f.Hours.Select(HourData.From).ToList(),
        Days = f.Days.Select(DayData.From).ToList(),
        Periods = f.Periods.Select(PeriodData.From).ToList(),
        SourceName = f.SourceName,
        Sources = f.Sources.ToList(),
    };

    public Forecast To(Location location) => new(
        location,
        Iso.ReadStamp(FetchedAt),
        // Beyond 14 hours no DateTimeOffset can hold it, and the writer
        // would fail on it later, outside anything that catches (#22).
        Math.Abs(UtcOffsetSeconds) <= 14 * 3600 ? TimeSpan.FromSeconds(UtcOffsetSeconds) : throw new InvalidDataException("The cache file's UTC offset is out of range."),
        Units == "imperial" ? UnitSystem.Imperial : UnitSystem.Metric,
        Iso.Required(Current, "current conditions").To(),
        Iso.Required(Hours, "hours").Select(h => h.To()).ToList(),
        Iso.Required(Days, "days").Select(d => d.To()).ToList(),
        Iso.Required(Periods, "periods").Select(p => p.To()).ToList(),
        Iso.Required(SourceName, "source name"),
        Iso.Required(Sources, "sources"));
}

[DataContract]
internal sealed class CurrentData
{
    [DataMember(Name = "localTime")] public string? LocalTime;
    [DataMember(Name = "temperature")] public double Temperature;
    [DataMember(Name = "feelsLike")] public double FeelsLike;
    [DataMember(Name = "humidity")] public int Humidity;
    [DataMember(Name = "weatherCode")] public int WeatherCode;
    [DataMember(Name = "isDay")] public bool IsDay;
    [DataMember(Name = "windSpeed")] public double WindSpeed;
    [DataMember(Name = "windDirection")] public int WindDirection;
    [DataMember(Name = "windGusts")] public double WindGusts;
    [DataMember(Name = "precipitation")] public double Precipitation;
    [DataMember(Name = "cloudCover")] public int CloudCover;
    [DataMember(Name = "pressureHpa")] public double PressureHpa;
    [DataMember(Name = "station")] public string? Station;
    [DataMember(Name = "description")] public string? Description;
    [DataMember(Name = "dewPoint")] public double? DewPoint;
    [DataMember(Name = "visibilityMetres")] public double? VisibilityMetres;

    public static CurrentData From(CurrentConditions c) => new()
    {
        LocalTime = Iso.Write(c.LocalTime),
        Temperature = c.Temperature,
        FeelsLike = c.FeelsLike,
        Humidity = c.Humidity,
        WeatherCode = c.WeatherCode,
        IsDay = c.IsDay,
        WindSpeed = c.WindSpeed,
        WindDirection = c.WindDirection,
        WindGusts = c.WindGusts,
        Precipitation = c.Precipitation,
        CloudCover = c.CloudCover,
        PressureHpa = c.PressureHpa,
        Station = c.Station,
        Description = c.Description,
        DewPoint = c.DewPoint,
        VisibilityMetres = c.VisibilityMetres,
    };

    public CurrentConditions To() => new(
        Iso.ReadWall(LocalTime), Temperature, FeelsLike, Humidity, WeatherCode, IsDay,
        WindSpeed, WindDirection, WindGusts, Precipitation, CloudCover, PressureHpa,
        Station, Description, DewPoint, VisibilityMetres);
}

[DataContract]
internal sealed class HourData
{
    [DataMember(Name = "localTime")] public string? LocalTime;
    [DataMember(Name = "temperature")] public double Temperature;
    [DataMember(Name = "precipitationProbability")] public int? PrecipitationProbability;
    [DataMember(Name = "precipitation")] public double Precipitation;
    [DataMember(Name = "weatherCode")] public int WeatherCode;
    [DataMember(Name = "windSpeed")] public double WindSpeed;
    [DataMember(Name = "windDirection")] public int WindDirection;
    [DataMember(Name = "windGusts")] public double WindGusts;
    [DataMember(Name = "visibilityMetres")] public double? VisibilityMetres;
    [DataMember(Name = "dewPoint")] public double? DewPoint;
    [DataMember(Name = "humidity")] public int? Humidity;
    [DataMember(Name = "isDay")] public bool IsDay;

    public static HourData From(HourPoint h) => new()
    {
        LocalTime = Iso.Write(h.LocalTime),
        Temperature = h.Temperature,
        PrecipitationProbability = h.PrecipitationProbability,
        Precipitation = h.Precipitation,
        WeatherCode = h.WeatherCode,
        WindSpeed = h.WindSpeed,
        WindDirection = h.WindDirection,
        WindGusts = h.WindGusts,
        VisibilityMetres = h.VisibilityMetres,
        DewPoint = h.DewPoint,
        Humidity = h.Humidity,
        IsDay = h.IsDay,
    };

    public HourPoint To() => new(
        Iso.ReadWall(LocalTime), Temperature, PrecipitationProbability, Precipitation, WeatherCode,
        WindSpeed, WindDirection, WindGusts, VisibilityMetres, DewPoint, Humidity, IsDay);
}

[DataContract]
internal sealed class DayData
{
    [DataMember(Name = "date")] public string? Date;
    [DataMember(Name = "weatherCode")] public int WeatherCode;
    [DataMember(Name = "high")] public double High;
    [DataMember(Name = "low")] public double Low;
    [DataMember(Name = "feelsLikeHigh")] public double FeelsLikeHigh;
    [DataMember(Name = "feelsLikeLow")] public double FeelsLikeLow;
    [DataMember(Name = "sunrise")] public string? Sunrise;
    [DataMember(Name = "sunset")] public string? Sunset;
    [DataMember(Name = "daylightSeconds")] public double? DaylightSeconds;
    [DataMember(Name = "uvIndexMax")] public double? UvIndexMax;
    [DataMember(Name = "precipitationSum")] public double PrecipitationSum;
    [DataMember(Name = "precipitationProbabilityMax")] public int? PrecipitationProbabilityMax;
    [DataMember(Name = "snowfallSum")] public double SnowfallSum;
    [DataMember(Name = "windSpeedMax")] public double WindSpeedMax;
    [DataMember(Name = "windGustsMax")] public double WindGustsMax;
    [DataMember(Name = "windDirectionDominant")] public int WindDirectionDominant;

    public static DayData From(DayForecast d) => new()
    {
        Date = Iso.Write(d.Date),
        WeatherCode = d.WeatherCode,
        High = d.High,
        Low = d.Low,
        FeelsLikeHigh = d.FeelsLikeHigh,
        FeelsLikeLow = d.FeelsLikeLow,
        Sunrise = Iso.Write(d.Sunrise),
        Sunset = Iso.Write(d.Sunset),
        DaylightSeconds = d.DaylightSeconds,
        UvIndexMax = d.UvIndexMax,
        PrecipitationSum = d.PrecipitationSum,
        PrecipitationProbabilityMax = d.PrecipitationProbabilityMax,
        SnowfallSum = d.SnowfallSum,
        WindSpeedMax = d.WindSpeedMax,
        WindGustsMax = d.WindGustsMax,
        WindDirectionDominant = d.WindDirectionDominant,
    };

    public DayForecast To() => new(
        Iso.ReadWall(Date), WeatherCode, High, Low, FeelsLikeHigh, FeelsLikeLow,
        Iso.ReadWallOrNull(Sunrise), Iso.ReadWallOrNull(Sunset), DaylightSeconds, UvIndexMax,
        PrecipitationSum, PrecipitationProbabilityMax, SnowfallSum, WindSpeedMax, WindGustsMax, WindDirectionDominant);
}

[DataContract]
internal sealed class PeriodData
{
    [DataMember(Name = "name")] public string? Name;
    [DataMember(Name = "date")] public string? Date;
    [DataMember(Name = "text")] public string? Text;

    public static PeriodData From(OfficialPeriod p) => new() { Name = p.Name, Date = Iso.Write(p.Date), Text = p.Text };

    public OfficialPeriod To() => new(Iso.Required(Name, "period name"), Iso.ReadWall(Date), Iso.Required(Text, "period text"));
}

[DataContract]
internal sealed class AlertsData
{
    [DataMember(Name = "attribution")] public string? Attribution;
    [DataMember(Name = "checkedAt")] public string? CheckedAt;
    [DataMember(Name = "alerts")] public List<AlertData>? Alerts;

    public static AlertsData From(AlertReport r) => new()
    {
        Attribution = r.Attribution,
        CheckedAt = Iso.Write(r.CheckedAt),
        Alerts = r.Alerts.Select(AlertData.From).ToList(),
    };

    public AlertReport To() =>
        Attribution is null
            ? AlertReport.NotAvailable
            : new AlertReport(Iso.Required(Alerts, "alerts").Select(a => a.To()).ToList(), Attribution, null, Iso.ReadStampOrNull(CheckedAt));
}

[DataContract]
internal sealed class AlertData
{
    [DataMember(Name = "id")] public string? Id;
    [DataMember(Name = "event")] public string? Event;
    [DataMember(Name = "severity")] public string? Severity;
    [DataMember(Name = "issued")] public string? Issued;
    [DataMember(Name = "onset")] public string? Onset;
    [DataMember(Name = "ends")] public string? Ends;
    [DataMember(Name = "source")] public string? Source;
    [DataMember(Name = "sender")] public string? Sender;
    [DataMember(Name = "area")] public string? Area;
    [DataMember(Name = "level")] public string? Level;
    [DataMember(Name = "description")] public string? Description;
    [DataMember(Name = "instruction")] public string? Instruction;
    [DataMember(Name = "url")] public string? Url;
    [DataMember(Name = "expires")] public string? Expires;

    public static AlertData From(WeatherAlert a) => new()
    {
        Id = a.Id,
        Event = a.Event,
        Severity = a.Severity.ToString(),
        Issued = Iso.Write(a.Issued),
        Onset = Iso.Write(a.Onset),
        Ends = Iso.Write(a.Ends),
        Source = a.Source,
        Sender = a.Sender,
        Area = a.Area,
        Level = a.Level,
        Description = a.Description,
        Instruction = a.Instruction,
        Url = a.Url,
        Expires = Iso.Write(a.Expires),
    };

    public WeatherAlert To() => new(
        Iso.Required(Id, "alert id"),
        Iso.Required(Event, "alert event"),
        Enum.TryParse<AlertSeverity>(Severity, out var severity) ? severity : AlertSeverity.Unknown,
        Iso.ReadStamp(Issued),
        Iso.ReadStampOrNull(Onset),
        Iso.ReadStampOrNull(Ends),
        Iso.Required(Source, "alert source"),
        Iso.Required(Sender, "alert sender"),
        Iso.Required(Area, "alert area"),
        Level,
        Iso.Required(Description, "alert description"),
        Instruction,
        Url,
        Iso.ReadStampOrNull(Expires));
}
