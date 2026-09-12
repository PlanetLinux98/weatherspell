using System.Globalization;

namespace Weatherspell.Weather.EnvironmentCanada;

// Environment Canada publishes a forecast page per site (about 850 towns),
// listed with coordinates in site_list_en.csv on the Datamart. A location
// gets the nearest site's page.
internal sealed record Site(string Code, string Name, string Province, double Latitude, double Longitude);

internal static class SiteList
{
    public const string Url = "https://dd.weather.gc.ca/today/citypage_weather/docs/site_list_en.csv";

    // Two header lines ("Site Names" then the column names), then
    // s0000629,Peterborough,ON,44.30N,78.33W per line.
    public static IReadOnlyList<Site> Parse(string csv)
    {
        var sites = new List<Site>();
        foreach (var raw in csv.Split('\n'))
        {
            var line = raw.TrimEnd('\r');
            var f = line.Split(',');
            if (f.Length < 5 || !f[0].StartsWith("s", StringComparison.Ordinal) || f[0].Length != 8) continue;
            if (!TryCoordinate(f[3], 'N', 'S', out var lat) || !TryCoordinate(f[4], 'E', 'W', out var lon)) continue;
            sites.Add(new Site(f[0], f[1].Trim(), f[2].Trim().ToUpperInvariant(), lat, lon));
        }
        if (sites.Count == 0) throw new InvalidDataException("The Environment Canada site list is empty.");
        return sites;
    }

    private static bool TryCoordinate(string text, char positive, char negative, out double value)
    {
        value = 0;
        var t = text.Trim();
        if (t.Length < 2) return false;
        var hemisphere = char.ToUpperInvariant(t[t.Length - 1]);
        if (hemisphere != positive && hemisphere != negative) return false;
        if (!double.TryParse(t.Substring(0, t.Length - 1), NumberStyles.Float, CultureInfo.InvariantCulture, out value)) return false;
        if (hemisphere == negative) value = -value;
        return true;
    }

    public static (Site Site, double DistanceKm) Nearest(IReadOnlyList<Site> sites, double latitude, double longitude)
    {
        Site? best = null;
        var bestDistance = double.MaxValue;
        foreach (var site in sites)
        {
            var d = DistanceKm(latitude, longitude, site.Latitude, site.Longitude);
            if (d < bestDistance)
            {
                bestDistance = d;
                best = site;
            }
        }
        return (best ?? throw new InvalidDataException("No Environment Canada sites."), bestDistance);
    }

    // Haversine; the site list's two decimals make anything finer pointless.
    public static double DistanceKm(double lat1, double lon1, double lat2, double lon2)
    {
        const double R = 6371.0;
        var dLat = Radians(lat2 - lat1);
        var dLon = Radians(lon2 - lon1);
        var a = Math.Sin(dLat / 2) * Math.Sin(dLat / 2)
            + Math.Cos(Radians(lat1)) * Math.Cos(Radians(lat2)) * Math.Sin(dLon / 2) * Math.Sin(dLon / 2);
        return 2 * R * Math.Asin(Math.Sqrt(a));
    }

    private static double Radians(double degrees) => degrees * Math.PI / 180;
}
