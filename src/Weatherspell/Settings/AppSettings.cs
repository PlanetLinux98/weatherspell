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

    public void Normalize()
    {
        Locations ??= [];
        Locations.RemoveAll(l => l is null || string.IsNullOrWhiteSpace(l.Name));
        if (LastLocation < 0 || LastLocation >= Locations.Count) LastLocation = 0;
    }
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
