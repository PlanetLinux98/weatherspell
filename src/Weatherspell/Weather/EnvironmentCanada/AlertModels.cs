using System.Runtime.Serialization;

namespace Weatherspell.Weather.EnvironmentCanada;

// Wire shapes for the weather-alerts collection of MSC GeoMet-OGC-API
// (api.weather.gc.ca), GeoJSON features with the alert in "properties".
// Times are UTC ISO 8601; the French fields are not read.

[DataContract]
internal sealed class EcAlertsResponse
{
    [DataMember(Name = "features")] public EcAlertFeature[]? Features;
}

[DataContract]
internal sealed class EcAlertFeature
{
    [DataMember(Name = "id")] public string? Id;
    [DataMember(Name = "properties")] public EcAlert? Properties;
}

[DataContract]
internal sealed class EcAlert
{
    [DataMember(Name = "alert_type")] public string? AlertType;
    [DataMember(Name = "alert_name_en")] public string? Name;
    [DataMember(Name = "alert_text_en")] public string? Text;
    [DataMember(Name = "publication_datetime")] public string? Published;
    [DataMember(Name = "expiration_datetime")] public string? Expires;
    [DataMember(Name = "validity_datetime")] public string? Valid;
    [DataMember(Name = "event_end_datetime")] public string? EventEnd;
    [DataMember(Name = "risk_colour_en")] public string? Colour;
    [DataMember(Name = "impact_en")] public string? Impact;
    [DataMember(Name = "confidence_en")] public string? Confidence;
    [DataMember(Name = "feature_name_en")] public string? Area;
    [DataMember(Name = "status_en")] public string? Status;
}
