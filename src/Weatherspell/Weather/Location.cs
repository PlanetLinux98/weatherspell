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

    // Search-result line: the population tells namesakes apart.
    public string SearchResultText =>
        Population is long p && p > 0
            ? $"{FullName} (population {p.ToString("N0", System.Globalization.CultureInfo.CurrentCulture)})"
            : FullName;
}
