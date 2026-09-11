using System.Runtime.Serialization;

namespace Weatherspell.Weather.OpenMeteo;

// Wire shapes for Open-Meteo's JSON, read with the in-box
// DataContractJsonSerializer (no System.Text.Json: it would ship a DLL).
// Member names match the API's snake_case; nullable arrays absorb JSON nulls.

[DataContract]
internal sealed class ForecastResponse
{
    [DataMember(Name = "latitude")] public double Latitude;
    [DataMember(Name = "longitude")] public double Longitude;
    [DataMember(Name = "timezone")] public string? Timezone;
    [DataMember(Name = "timezone_abbreviation")] public string? TimezoneAbbreviation;
    [DataMember(Name = "utc_offset_seconds")] public int UtcOffsetSeconds;
    [DataMember(Name = "current")] public CurrentBlock? Current;
    [DataMember(Name = "hourly")] public HourlyBlock? Hourly;
    [DataMember(Name = "daily")] public DailyBlock? Daily;
}

[DataContract]
internal sealed class CurrentBlock
{
    [DataMember(Name = "time")] public string? Time;
    [DataMember(Name = "temperature_2m")] public double? Temperature;
    [DataMember(Name = "relative_humidity_2m")] public double? Humidity;
    [DataMember(Name = "apparent_temperature")] public double? ApparentTemperature;
    [DataMember(Name = "is_day")] public int? IsDay;
    [DataMember(Name = "precipitation")] public double? Precipitation;
    [DataMember(Name = "weather_code")] public int? WeatherCode;
    [DataMember(Name = "cloud_cover")] public double? CloudCover;
    [DataMember(Name = "wind_speed_10m")] public double? WindSpeed;
    [DataMember(Name = "wind_direction_10m")] public double? WindDirection;
    [DataMember(Name = "wind_gusts_10m")] public double? WindGusts;
    [DataMember(Name = "pressure_msl")] public double? PressureMsl;
}

[DataContract]
internal sealed class HourlyBlock
{
    [DataMember(Name = "time")] public string[]? Time;
    [DataMember(Name = "temperature_2m")] public double?[]? Temperature;
    [DataMember(Name = "precipitation_probability")] public double?[]? PrecipitationProbability;
    [DataMember(Name = "precipitation")] public double?[]? Precipitation;
    [DataMember(Name = "weather_code")] public double?[]? WeatherCode;
    [DataMember(Name = "wind_speed_10m")] public double?[]? WindSpeed;
    [DataMember(Name = "wind_direction_10m")] public double?[]? WindDirection;
    [DataMember(Name = "wind_gusts_10m")] public double?[]? WindGusts;
    [DataMember(Name = "visibility")] public double?[]? Visibility;
    [DataMember(Name = "dew_point_2m")] public double?[]? DewPoint;
    [DataMember(Name = "relative_humidity_2m")] public double?[]? Humidity;
    [DataMember(Name = "is_day")] public double?[]? IsDay;
}

[DataContract]
internal sealed class DailyBlock
{
    [DataMember(Name = "time")] public string[]? Time;
    [DataMember(Name = "weather_code")] public double?[]? WeatherCode;
    [DataMember(Name = "temperature_2m_max")] public double?[]? TemperatureMax;
    [DataMember(Name = "temperature_2m_min")] public double?[]? TemperatureMin;
    [DataMember(Name = "apparent_temperature_max")] public double?[]? ApparentMax;
    [DataMember(Name = "apparent_temperature_min")] public double?[]? ApparentMin;
    [DataMember(Name = "sunrise")] public string?[]? Sunrise;
    [DataMember(Name = "sunset")] public string?[]? Sunset;
    [DataMember(Name = "daylight_duration")] public double?[]? DaylightDuration;
    [DataMember(Name = "uv_index_max")] public double?[]? UvIndexMax;
    [DataMember(Name = "precipitation_sum")] public double?[]? PrecipitationSum;
    [DataMember(Name = "precipitation_probability_max")] public double?[]? PrecipitationProbabilityMax;
    [DataMember(Name = "snowfall_sum")] public double?[]? SnowfallSum;
    [DataMember(Name = "wind_speed_10m_max")] public double?[]? WindSpeedMax;
    [DataMember(Name = "wind_gusts_10m_max")] public double?[]? WindGustsMax;
    [DataMember(Name = "wind_direction_10m_dominant")] public double?[]? WindDirectionDominant;
}

[DataContract]
internal sealed class GeocodingResponse
{
    [DataMember(Name = "results")] public GeocodingResult[]? Results;
}

[DataContract]
internal sealed class GeocodingResult
{
    [DataMember(Name = "id")] public long Id;
    [DataMember(Name = "name")] public string? Name;
    [DataMember(Name = "latitude")] public double Latitude;
    [DataMember(Name = "longitude")] public double Longitude;
    [DataMember(Name = "country")] public string? Country;
    [DataMember(Name = "country_code")] public string? CountryCode;
    [DataMember(Name = "admin1")] public string? Admin1;
    [DataMember(Name = "admin2")] public string? Admin2;
    [DataMember(Name = "timezone")] public string? Timezone;
    [DataMember(Name = "population")] public long? Population;
    [DataMember(Name = "feature_code")] public string? FeatureCode;
}
