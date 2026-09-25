namespace Weatherspell.Weather;

// WMO weather interpretation codes as Open-Meteo reports them, turned into
// phrases that sit mid-sentence ("it's 21 degrees and mostly clear").
internal static class WeatherCodes
{
    public static string Describe(int code, bool isDay = true) => code switch
    {
        0 => "clear",
        1 => "mostly clear",
        2 => "partly cloudy",
        3 => "overcast",
        45 => "foggy",
        48 => "freezing fog",
        51 => "light drizzle",
        53 => "drizzle",
        55 => "heavy drizzle",
        56 => "light freezing drizzle",
        57 => "freezing drizzle",
        61 => "light rain",
        63 => "rain",
        65 => "heavy rain",
        66 => "light freezing rain",
        67 => "freezing rain",
        71 => "light snow",
        73 => "snow",
        75 => "heavy snow",
        77 => "snow grains",
        80 => "light rain showers",
        81 => "rain showers",
        82 => "heavy rain showers",
        85 => "light snow showers",
        86 => "heavy snow showers",
        95 => "thunderstorms",
        96 => "thunderstorms with hail",
        99 => "thunderstorms with heavy hail",
        _ => "unknown conditions",
    };

    // Capitalized for the start of a sentence ("Partly cloudy.").
    public static string DescribeSentence(int code, bool isDay = true)
    {
        var s = Describe(code, isDay);
        return char.ToUpperInvariant(s[0]) + s.Substring(1);
    }

    public static bool IsPrecipitation(int code) => code >= 50;

    public static bool IsSnow(int code) => code is >= 71 and <= 77 or 85 or 86;

    public static bool IsThunder(int code) => code >= 95;

    // The phrases that are nouns ("drizzle", "freezing fog", "thunderstorms"),
    // which follow "with" rather than "and" in "it's 13 degrees with drizzle".
    public static bool IsNoun(int code) => IsPrecipitation(code) || code == 48;

    // The same weather at different strengths ("light snow", "snow", "heavy
    // snow") shares a kind, so a heavier hour reads "heavier at times".
    public static string Kind(int code) => code switch
    {
        51 or 53 or 55 => "drizzle",
        56 or 57 => "freezing drizzle",
        61 or 63 or 65 => "rain",
        66 or 67 => "freezing rain",
        71 or 73 or 75 => "snow",
        80 or 81 or 82 => "rain showers",
        85 or 86 => "snow showers",
        95 or 96 or 99 => "thunderstorms",
        _ => code.ToString(System.Globalization.CultureInfo.InvariantCulture),
    };

    // Higher means more worth mentioning: a thunderstorm in one hour matters
    // more than three hours of overcast.
    public static int Severity(int code) => code switch
    {
        >= 95 => 9,
        66 or 67 => 8,
        56 or 57 => 8,
        65 or 82 or 75 or 86 => 7,
        63 or 81 or 73 => 6,
        61 or 80 or 71 or 85 or 77 => 5,
        55 => 4,
        51 or 53 => 3,
        45 or 48 => 2,
        3 => 1,
        _ => 0,
    };

    // The word for what falls: chance of rain / snow / precipitation.
    public static string PrecipitationWord(int code) => IsSnow(code) ? "snow" : (IsPrecipitation(code) ? "rain" : "precipitation");
}
