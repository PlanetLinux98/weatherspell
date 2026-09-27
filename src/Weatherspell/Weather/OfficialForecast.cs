namespace Weatherspell.Weather;

// What a national weather service adds on top of the Open-Meteo base: its
// written forecast periods and its station observation. Observed values are
// SI (Celsius, kilometres an hour, hectopascals, metres) whatever the user's
// units; ForecastService converts once, when it lays them over the base.
internal sealed record OfficialForecast(
    string SourceName,
    string Attribution,
    IReadOnlyList<OfficialPeriod> Periods,
    Observation? Observation);

// Nulls are values the station did not report this time (the NWS marks
// them as such; Environment Canada leaves the element empty).
internal sealed record Observation(
    string Station,
    DateTimeOffset Time,
    string? Description,
    double? TemperatureC,
    double? FeelsLikeC,
    int? Humidity,
    double? WindKmh,
    int? WindDirection,
    double? WindGustKmh,
    double? PressureHpa,
    double? DewPointC,
    double? VisibilityMetres);

// A service that has no forecast text for a place at all: Environment
// Canada's observation-only Arctic sites (Alert, Eureka), an NWS point with
// no forecast grid (American Samoa). Not a failure another try would fix,
// so ForecastService words it as a fact, not "could not be fetched this
// time" (#21).
internal sealed class NoForecastTextException(string message) : IOException(message);

// Official sentences are shown as written, in the service's own units, with
// one typographic pass so they read aloud the way the service says them on
// the radio: symbols become words.
internal static class OfficialText
{
    public static string Spoken(string text)
    {
        // The weather.gov page's text has two spaces after a full stop.
        var s = System.Text.RegularExpressions.Regex.Replace(text.Trim(), @"\s{2,}", " ");
        s = System.Text.RegularExpressions.Regex.Replace(s, @"(\d)\s*%", "$1 percent");
        s = System.Text.RegularExpressions.Regex.Replace(s, @"\bkm/h\b", "kilometres per hour");
        s = System.Text.RegularExpressions.Regex.Replace(s, @"\bmph\b", "miles per hour");
        return s;
    }

    // A station as the services name it, read as words: "Gander Int'l
    // Airport" spelled out, and the NWS's all-capital names ("VISITORS
    // CENTER AT FURNACE CREEK DEATH VALLEY") in title case (#21).
    public static string StationName(string name)
    {
        var s = name.Trim();
        if (s == s.ToUpperInvariant() && s.Any(char.IsLetter))
        {
            s = System.Globalization.CultureInfo.InvariantCulture.TextInfo.ToTitleCase(s.ToLowerInvariant());
            s = System.Text.RegularExpressions.Regex.Replace(s, @"(?<=\S )(At|Of|The|And|On|In)\b", m => m.Value.ToLowerInvariant());
        }
        return System.Text.RegularExpressions.Regex.Replace(s, @"\bInt'l(?=\s|$)", "International");
    }

    // "Flash Flood Watch" as the NWS titles it, or "frost advisory" as
    // Environment Canada names it, becomes "Flash flood watch": the
    // sentence case this app's own text uses.
    public static string SentenceCase(string name)
    {
        var s = name.Trim();
        return s.Length == 0 ? s : char.ToUpperInvariant(s[0]) + s.Substring(1).ToLowerInvariant();
    }
}
