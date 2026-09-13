using System.Runtime.Serialization;

namespace Weatherspell.Weather.Nws;

// Wire shapes for the parts of api.weather.gov's GeoJSON that are read.
// Everything sits under "properties"; measured values are objects with a
// nullable "value" (null with a qualityControl flag when not reported).

[DataContract]
internal sealed class NwsPointsResponse
{
    [DataMember(Name = "properties")] public NwsPoints? Properties;
}

[DataContract]
internal sealed class NwsPoints
{
    [DataMember(Name = "forecast")] public string? Forecast;
    [DataMember(Name = "observationStations")] public string? ObservationStations;
    [DataMember(Name = "timeZone")] public string? TimeZone;
    [DataMember(Name = "relativeLocation")] public NwsRelativeLocation? RelativeLocation;
}

[DataContract]
internal sealed class NwsRelativeLocation
{
    [DataMember(Name = "properties")] public NwsRelativeLocationProperties? Properties;
}

[DataContract]
internal sealed class NwsRelativeLocationProperties
{
    [DataMember(Name = "city")] public string? City;
    [DataMember(Name = "state")] public string? State;
}

[DataContract]
internal sealed class NwsForecastResponse
{
    [DataMember(Name = "properties")] public NwsForecastProperties? Properties;
}

[DataContract]
internal sealed class NwsForecastProperties
{
    [DataMember(Name = "updateTime")] public string? UpdateTime;
    [DataMember(Name = "periods")] public NwsPeriod[]? Periods;
}

[DataContract]
internal sealed class NwsPeriod
{
    [DataMember(Name = "number")] public int Number;
    [DataMember(Name = "name")] public string? Name;
    [DataMember(Name = "startTime")] public string? StartTime;
    [DataMember(Name = "endTime")] public string? EndTime;
    [DataMember(Name = "isDaytime")] public bool IsDaytime;
    [DataMember(Name = "detailedForecast")] public string? DetailedForecast;
}

[DataContract]
internal sealed class NwsStationsResponse
{
    [DataMember(Name = "features")] public NwsStationFeature[]? Features;
}

[DataContract]
internal sealed class NwsStationFeature
{
    [DataMember(Name = "properties")] public NwsStation? Properties;
}

[DataContract]
internal sealed class NwsStation
{
    [DataMember(Name = "stationIdentifier")] public string? StationIdentifier;
    [DataMember(Name = "name")] public string? Name;
}

[DataContract]
internal sealed class NwsObservationResponse
{
    [DataMember(Name = "properties")] public NwsObservation? Properties;
}

[DataContract]
internal sealed class NwsObservation
{
    [DataMember(Name = "stationName")] public string? StationName;
    [DataMember(Name = "timestamp")] public string? Timestamp;
    [DataMember(Name = "textDescription")] public string? TextDescription;
    [DataMember(Name = "temperature")] public NwsValue? Temperature;
    [DataMember(Name = "dewpoint")] public NwsValue? Dewpoint;
    [DataMember(Name = "windDirection")] public NwsValue? WindDirection;
    [DataMember(Name = "windSpeed")] public NwsValue? WindSpeed;
    [DataMember(Name = "windGust")] public NwsValue? WindGust;
    [DataMember(Name = "barometricPressure")] public NwsValue? BarometricPressure;
    [DataMember(Name = "seaLevelPressure")] public NwsValue? SeaLevelPressure;
    [DataMember(Name = "visibility")] public NwsValue? Visibility;
    [DataMember(Name = "relativeHumidity")] public NwsValue? RelativeHumidity;
    [DataMember(Name = "windChill")] public NwsValue? WindChill;
    [DataMember(Name = "heatIndex")] public NwsValue? HeatIndex;
}

[DataContract]
internal sealed class NwsValue
{
    [DataMember(Name = "unitCode")] public string? UnitCode;
    [DataMember(Name = "value")] public double? Value;
}

// alerts/active?point=: one feature per alert message. Parameters is a map
// of string arrays (VTEC, AWIPSidentifier, NWSheadline and others), which the
// in-box serializer reads only in its simple dictionary format (see Json).

[DataContract]
internal sealed class NwsAlertsResponse
{
    [DataMember(Name = "features")] public NwsAlertFeature[]? Features;
}

[DataContract]
internal sealed class NwsAlertFeature
{
    [DataMember(Name = "properties")] public NwsAlert? Properties;
}

[DataContract]
internal sealed class NwsAlert
{
    [DataMember(Name = "id")] public string? Id;
    [DataMember(Name = "areaDesc")] public string? AreaDesc;
    [DataMember(Name = "sent")] public string? Sent;
    [DataMember(Name = "onset")] public string? Onset;
    [DataMember(Name = "expires")] public string? Expires;
    [DataMember(Name = "ends")] public string? Ends;
    [DataMember(Name = "status")] public string? Status;
    [DataMember(Name = "messageType")] public string? MessageType;
    [DataMember(Name = "severity")] public string? Severity;
    [DataMember(Name = "event")] public string? Event;
    [DataMember(Name = "senderName")] public string? SenderName;
    [DataMember(Name = "headline")] public string? Headline;
    [DataMember(Name = "description")] public string? Description;
    [DataMember(Name = "instruction")] public string? Instruction;
    [DataMember(Name = "parameters")] public Dictionary<string, string[]>? Parameters;
}
