namespace Weatherspell.Weather;

// A place the user can ask about. Name/Region/Country come from the geocoder
// and are kept verbatim; Nickname is the user's optional label for it.
internal sealed record Location(
    string Name,
    string? Region,
    string? Country,
    double Latitude,
    double Longitude,
    string? TimeZoneId,
    string? Nickname = null,
    long? Population = null)
{
    // "Peterborough, Ontario, Canada": what the combo box and headings show
    // when there is no nickname.
    public string FullName
    {
        get
        {
            var parts = new List<string> { Name };
            if (!string.IsNullOrWhiteSpace(Region) && Region != Name) parts.Add(Region!);
            if (!string.IsNullOrWhiteSpace(Country)) parts.Add(Country!);
            return string.Join(", ", parts);
        }
    }

    public string DisplayName => string.IsNullOrWhiteSpace(Nickname) ? FullName : Nickname!;

    // Where the National Weather Service writes the forecast and the alerts:
    // the states and the territories it has offices for (San Juan, Guam,
    // Pago Pago). Open-Meteo's geocoder gives the territories only a country
    // code, so OpenMeteoClient names them from this list (#19).
    public static readonly IReadOnlyDictionary<string, string> NwsTerritories = new Dictionary<string, string>
    {
        ["PR"] = "Puerto Rico",
        ["VI"] = "United States Virgin Islands",
        ["GU"] = "Guam",
        ["MP"] = "Northern Mariana Islands",
        ["AS"] = "American Samoa",
    };

    public bool IsNwsCovered => Country == "United States" || (Country is not null && NwsTerritories.Values.Contains(Country));

    // Search-result line: the population tells namesakes apart.
    public string SearchResultText =>
        Population is long p && p > 0
            ? $"{FullName} (population {p.ToString("N0", System.Globalization.CultureInfo.CurrentCulture)})"
            : FullName;
}
