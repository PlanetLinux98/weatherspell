using System.Runtime.Serialization;

namespace Weatherspell.Weather.Nominatim;

// reverse?format=jsonv2&addressdetails=1: the matched object and the
// address of where it is; {"error": "..."} when nothing is there.
[DataContract]
internal sealed class ReverseResponse
{
    [DataMember(Name = "error")] public string? Error;
    [DataMember(Name = "name")] public string? Name;
    [DataMember(Name = "address")] public ReverseAddress? Address;
}

[DataContract]
internal sealed class ReverseAddress
{
    [DataMember(Name = "hamlet")] public string? Hamlet;
    [DataMember(Name = "village")] public string? Village;
    [DataMember(Name = "town")] public string? Town;
    [DataMember(Name = "city")] public string? City;
    [DataMember(Name = "municipality")] public string? Municipality;
    [DataMember(Name = "county")] public string? County;
    [DataMember(Name = "state_district")] public string? StateDistrict;
    [DataMember(Name = "state")] public string? State;
    [DataMember(Name = "ISO3166-2-lvl4")] public string? Iso3166Level4;
    [DataMember(Name = "country")] public string? Country;
    [DataMember(Name = "country_code")] public string? CountryCode;
}
