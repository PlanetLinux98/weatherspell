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

// Official sentences are shown as written, in the service's own units, with
// one typographic pass so they read aloud the way the service says them on
// the radio: symbols become words.
internal static class OfficialText
{
    public static string Spoken(string text)
    {
        var s = text.Trim();
        s = System.Text.RegularExpressions.Regex.Replace(s, @"(\d)\s*%", "$1 percent");
        s = System.Text.RegularExpressions.Regex.Replace(s, @"\bkm/h\b", "kilometres per hour");
        s = System.Text.RegularExpressions.Regex.Replace(s, @"\bmph\b", "miles per hour");
        return s;
    }
}
