using System.Runtime.Serialization;
using Weatherspell.Weather;

namespace Weatherspell.Settings;

// What persists between runs. Read with the in-box DataContractJsonSerializer,
// which does not run constructors, so Normalize() fills anything a hand-edited
// or older file left out.
[DataContract]
internal sealed class AppSettings
{
    [DataMember(Name = "version")] public int Version = 1;
    [DataMember(Name = "locations")] public List<SavedLocation> Locations = [];
    [DataMember(Name = "lastLocation")] public int LastLocation;
    // Minutes between automatic refreshes of the forecast on screen and
    // between checks of every saved location's alerts. Any positive value
    // up to a day is honoured; the Settings dialog offers a short list.
    [DataMember(Name = "forecastRefreshMinutes")] public int ForecastRefreshMinutes = 30;
    [DataMember(Name = "alertCheckMinutes")] public int AlertCheckMinutes = 10;
    // Which new alerts are spoken: "all", "severe" (severe and extreme
    // only) or "off".
    [DataMember(Name = "alertAnnouncements")] public string AlertAnnouncements = "all";

    [OnDeserializing]
    private void Defaults(StreamingContext context)
    {
        ForecastRefreshMinutes = 30;
        AlertCheckMinutes = 10;
        AlertAnnouncements = "all";
    }

    public void Normalize()
    {
        Locations ??= [];
        Locations.RemoveAll(l => l is null || string.IsNullOrWhiteSpace(l.Name));
        foreach (var l in Locations) l.SeenAlertIds ??= [];
        if (LastLocation < 0 || LastLocation >= Locations.Count) LastLocation = 0;
        if (ForecastRefreshMinutes < 1 || ForecastRefreshMinutes > MaxMinutes) ForecastRefreshMinutes = 30;
        if (AlertCheckMinutes < 1 || AlertCheckMinutes > MaxMinutes) AlertCheckMinutes = 10;
        if (AlertAnnouncements is not ("all" or "severe" or "off")) AlertAnnouncements = "all";
    }

    // A day: a longer interval is a typo, and a Windows timer's interval is
    // an int of milliseconds.
    public const int MaxMinutes = 24 * 60;

    public bool Announces(Weather.Alerts.AlertSeverity severity) => AlertAnnouncements switch
    {
        "off" => false,
        "severe" => severity >= Weather.Alerts.AlertSeverity.Severe,
        _ => true,
    };
}

[DataContract]
internal sealed class SavedLocation
{
    [DataMember(Name = "name")] public string? Name;
    [DataMember(Name = "region")] public string? Region;
    [DataMember(Name = "country")] public string? Country;
    [DataMember(Name = "latitude")] public double Latitude;
    [DataMember(Name = "longitude")] public double Longitude;
    [DataMember(Name = "timeZoneId")] public string? TimeZoneId;
    [DataMember(Name = "nickname")] public string? Nickname;
    [DataMember(Name = "notifyAlerts")] public bool NotifyAlerts = true;
    // Ids of the alerts in effect at the last check, so each is announced
    // once, across runs (see AlertTracker).
    [DataMember(Name = "seenAlertIds")] public List<string> SeenAlertIds = [];
    // The location's UTC offset from its last forecast, so an alert for a
    // location not on screen can be announced in its own time.
    [DataMember(Name = "utcOffsetSeconds")] public int? UtcOffsetSeconds;

    // The serializer skips field initializers, so a file from before a
    // field existed would read it as false or null; defaults are set here,
    // just before the file's own values land.
    [OnDeserializing]
    private void Defaults(StreamingContext context)
    {
        NotifyAlerts = true;
        SeenAlertIds = [];
    }

    public static SavedLocation From(Location l) => new()
    {
        Name = l.Name,
        Region = l.Region,
        Country = l.Country,
        Latitude = l.Latitude,
        Longitude = l.Longitude,
        TimeZoneId = l.TimeZoneId,
        Nickname = l.Nickname,
    };

    public Location ToLocation() =>
        new(Name ?? "", Region, Country, Latitude, Longitude, TimeZoneId, Nickname);
}
