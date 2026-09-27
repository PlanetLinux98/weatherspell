using System.Globalization;

namespace Weatherspell.Weather.Nominatim;

// Names for coordinates typed into Add Location, from OpenStreetMap's
// Nominatim (no key; data ODbL, credited in About). Open-Meteo's geocoder
// only goes from names to coordinates. The usage policy allows lookups the
// user asks for, with a User-Agent naming the app (Http sets one) and at
// most one request a second, which the gate below holds to even if Enter
// is pressed repeatedly.
internal sealed class NominatimClient
{
    public const string SourceNote = "OpenStreetMap (openstreetmap.org), \u00A9 OpenStreetMap contributors, licensed ODbL";

    private const string Endpoint = "https://nominatim.openstreetmap.org/reverse";
    private static readonly SemaphoreSlim Gate = new(1, 1);
    private static DateTime _lastRequestUtc = DateTime.MinValue;

    public static string ReverseUrl(double latitude, double longitude)
    {
        var inv = CultureInfo.InvariantCulture;
        // Zoom 14 finds the neighbourhood; its address still names the
        // settlement, which is what the name is taken from.
        return Endpoint
            + "?format=jsonv2&addressdetails=1&zoom=14&accept-language=en"
            + "&lat=" + latitude.ToString("0.######", inv)
            + "&lon=" + longitude.ToString("0.######", inv);
    }

    public async Task<Location?> ReverseAsync(double latitude, double longitude, CancellationToken cancellationToken)
    {
        await Gate.WaitAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            var wait = _lastRequestUtc.AddSeconds(1) - DateTime.UtcNow;
            if (wait > TimeSpan.Zero) await Task.Delay(wait, cancellationToken).ConfigureAwait(false);
            var json = await Http.GetStringAsync(ReverseUrl(latitude, longitude), cancellationToken).ConfigureAwait(false);
            return ParseReverse(json, latitude, longitude);
        }
        finally
        {
            _lastRequestUtc = DateTime.UtcNow;
            Gate.Release();
        }
    }

    // The place keeps the coordinates asked about, not those of the object
    // Nominatim matched: a cottage is not the town centre. Null when nothing
    // there has an address at all ("Unable to geocode": open sea).
    public static Location? ParseReverse(string json, double latitude, double longitude)
    {
        var response = Json.Read<ReverseResponse>(json);
        if (response.Error is not null || response.Address is not ReverseAddress a) return null;

        // The smallest settlement first: a village inside a US town, a town
        // inside an amalgamated city ("Bobcaygeon", not "Kawartha Lakes").
        // Then whatever area the point is in, for the countryside and water.
        var name = First(a.Hamlet, a.Village, a.Town, a.City, a.Municipality, a.County, a.StateDistrict, response.Name)
            ?? Coordinates.Words(latitude, longitude);
        var region = First(a.State);
        var country = First(a.Country);

        // Nominatim puts the US territories in the United States; the
        // geocoder, and so the rest of the app, calls them by name (#19).
        var code = a.Iso3166Level4;
        if (code is not null && code.StartsWith("US-", StringComparison.Ordinal)
            && Location.NwsTerritories.TryGetValue(code.Substring(3), out var territory))
        {
            country = territory;
            region = null;
        }
        return new Location(name, region, country, latitude, longitude, TimeZoneId: null);
    }

    private static string? First(params string?[] values) =>
        values.Select(v => v?.Trim()).FirstOrDefault(v => !string.IsNullOrEmpty(v));
}
