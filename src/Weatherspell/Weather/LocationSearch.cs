using Weatherspell.Weather.OpenMeteo;

namespace Weatherspell.Weather;

// What the Find Location dialog searches: the embedded postal code table and
// the Open-Meteo geocoder (place names everywhere, and the postal codes it
// does index), merged so each country has one source for its codes.
internal sealed class LocationSearch
{
    private readonly OpenMeteoClient _client;

    public LocationSearch(OpenMeteoClient client)
    {
        _client = client;
    }

    public async Task<IReadOnlyList<Location>> SearchAsync(string query, CancellationToken cancellationToken)
    {
        var table = PostalCodes.Find(query);
        IReadOnlyList<Location> geocoder;
        try
        {
            geocoder = await _client.SearchAsync(query, cancellationToken).ConfigureAwait(false);
        }
        catch (Exception ex) when (table.Count > 0 && IsNetworkFailure(ex, cancellationToken))
        {
            // The table answered, so a dead network is no reason to show an
            // error: the geocoder could only have added other countries.
            return table;
        }
        return Merge(table, geocoder);
    }

    // The failures the dialog would report as "Couldn't search"; an HttpClient
    // timeout arrives as a cancellation the user did not ask for.
    private static bool IsNetworkFailure(Exception ex, CancellationToken cancellationToken) =>
        ex is System.Net.Http.HttpRequestException or IOException or InvalidDataException or System.Runtime.Serialization.SerializationException
        || (ex is OperationCanceledException && !cancellationToken.IsCancellationRequested);

    // Table entries first (the query was their code exactly), then the
    // geocoder's, minus its answers for countries the table covers: a code
    // the table knows in Canada must not also surface the geocoder's guess.
    public static IReadOnlyList<Location> Merge(IReadOnlyList<Location> table, IReadOnlyList<Location> geocoder)
    {
        if (table.Count == 0) return geocoder;
        var merged = new List<Location>(table);
        foreach (var location in geocoder)
        {
            if (location.Country is null || Array.IndexOf(PostalCodes.Countries, location.Country) < 0) merged.Add(location);
        }
        return merged;
    }
}
