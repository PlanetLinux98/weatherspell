using System.Globalization;
using System.Text.RegularExpressions;

namespace Weatherspell.Weather;

// Postal codes for the countries Open-Meteo's geocoder does not index. Its
// postcode search is dependable for the US, France, Spain, the Netherlands
// and Belgium, patchy elsewhere in Europe and absent for Canada, the UK,
// Australia, New Zealand and Ireland (probed 2026-09-12), so the exe carries
// the GeoNames table for those five: one entry per code, named after its
// most populous place. tools\Update-PostalCodes.ps1 regenerates it and
// explains the choices. GeoNames has only the first part of Canadian and
// Irish codes and the UK outward code, so a full code resolves to its area.
internal static class PostalCodes
{
    public const string SourceNote = "GeoNames (geonames.org), licensed CC BY 4.0";

    // Countries whose codes come from the table; the geocoder's answers for
    // them are dropped when the table has any, so each country has one source.
    public static readonly string[] Countries =
        ["Canada", "United Kingdom", "Australia", "New Zealand", "Ireland"];

    // A UK code with the space left out: outward code, then digit and two letters.
    private static readonly Regex UkFull = new(@"^[A-Z]{1,2}\d[A-Z\d]?\d[A-Z]{2}$", RegexOptions.CultureInvariant);
    // Canadian (letter digit letter digit letter digit) and Irish (letter, two
    // digits, four more) codes with the space left out: the first three count.
    private static readonly Regex CanadianOrIrishFull = new(@"^(?:[A-Z]\d[A-Z]\d[A-Z]\d|[A-Z]\d\d[A-Z\d]{4})$", RegexOptions.CultureInvariant);
    private static readonly Regex CodeLike = new(@"^[A-Z\d]{2,8}$", RegexOptions.CultureInvariant);

    private static readonly Lazy<Dictionary<string, List<Location>>> Table = new(Load);

    // Every place the query names as a postal code, in table order (country,
    // then code); empty when it names none. A code can exist in several
    // countries (2000 is Sydney and 1010 both Sydney and Auckland), and all
    // are listed with their country, as the geocoder does for its own.
    public static IReadOnlyList<Location> Find(string query)
    {
        var list = new List<Location>();
        foreach (var key in Keys(query))
        {
            if (Table.Value.TryGetValue(key, out var found)) list.AddRange(found);
        }
        return list;
    }

    // The table keys a query could mean: the whole thing, the part before a
    // space ("K9J 7B8", "SW1A 1AA", "D02 X285"), and the same cuts when the
    // space was left out.
    public static IReadOnlyList<string> Keys(string query)
    {
        var keys = new List<string>();
        var text = query.Trim().ToUpperInvariant();
        var whole = Regex.Replace(text, @"\s+", "");
        Add(keys, whole);
        var space = text.IndexOf(' ');
        if (space > 0) Add(keys, text.Substring(0, space));
        if (UkFull.IsMatch(whole)) Add(keys, whole.Substring(0, whole.Length - 3));
        if (CanadianOrIrishFull.IsMatch(whole)) Add(keys, whole.Substring(0, 3));
        return keys;
    }

    private static void Add(List<string> keys, string key)
    {
        if (CodeLike.IsMatch(key) && !keys.Contains(key)) keys.Add(key);
    }

    private static Dictionary<string, List<Location>> Load()
    {
        var table = new Dictionary<string, List<Location>>(StringComparer.Ordinal);
        using var stream = typeof(PostalCodes).Assembly.GetManifestResourceStream("PostalCodes.tsv")
            ?? throw new InvalidOperationException("The postal code table is not embedded.");
        using var reader = new StreamReader(stream);
        while (reader.ReadLine() is { } line)
        {
            if (line.Length == 0 || line[0] == '#') continue;
            var f = line.Split('\t');
            if (f.Length < 6) continue;
            var location = new Location(
                f[1], f[2].Length == 0 ? null : f[2], f[3],
                double.Parse(f[4], CultureInfo.InvariantCulture),
                double.Parse(f[5], CultureInfo.InvariantCulture),
                TimeZoneId: null);
            if (!table.TryGetValue(f[0], out var list)) table[f[0]] = list = [];
            list.Add(location);
        }
        return table;
    }
}
