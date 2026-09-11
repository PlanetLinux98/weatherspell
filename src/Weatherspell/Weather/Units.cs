using System.Globalization;

namespace Weatherspell.Weather;

internal enum UnitSystem
{
    Metric,
    Imperial,
}

// Unit choice and the words for each quantity. Sentences say "kilometres an
// hour", never "km/h": words read the same in every screen reader.
internal static class Units
{
    public static UnitSystem FromWindowsRegion() =>
        RegionInfo.CurrentRegion.IsMetric ? UnitSystem.Metric : UnitSystem.Imperial;

    // Open-Meteo request parameters, so values arrive already converted.
    public static string TemperatureParameter(UnitSystem u) => u == UnitSystem.Metric ? "celsius" : "fahrenheit";
    public static string WindParameter(UnitSystem u) => u == UnitSystem.Metric ? "kmh" : "mph";
    public static string PrecipitationParameter(UnitSystem u) => u == UnitSystem.Metric ? "mm" : "inch";

    public static string Degrees(double value)
    {
        var rounded = (int)Math.Round(value, MidpointRounding.AwayFromZero);
        // "minus 5 degrees" rather than a hyphen the synth may skip.
        return rounded < 0 ? $"minus {-rounded} degrees" : $"{rounded} degrees";
    }

    // Bare number for "high 26, low 15": the unit word was said once already.
    public static string DegreesBare(double value)
    {
        var rounded = (int)Math.Round(value, MidpointRounding.AwayFromZero);
        return rounded < 0 ? $"minus {-rounded}" : rounded.ToString(CultureInfo.InvariantCulture);
    }

    public static string Speed(double value, UnitSystem u)
    {
        var rounded = (int)Math.Round(value, MidpointRounding.AwayFromZero);
        return u == UnitSystem.Metric ? $"{rounded} kilometres an hour" : $"{rounded} miles an hour";
    }

    public static string SpeedBare(double value) =>
        ((int)Math.Round(value, MidpointRounding.AwayFromZero)).ToString(CultureInfo.InvariantCulture);

    public static string Distance(double metres, UnitSystem u)
    {
        if (u == UnitSystem.Metric)
        {
            var km = metres / 1000.0;
            return km < 1 ? $"{(int)Math.Round(metres)} metres" : $"{(int)Math.Round(km)} kilometres";
        }
        var miles = metres / 1609.344;
        return miles < 1 ? "under a mile" : $"{(int)Math.Round(miles)} miles";
    }

    public static string Pressure(double hpa, UnitSystem u) =>
        u == UnitSystem.Metric
            ? $"{(int)Math.Round(hpa)} hectopascals"
            : $"{(hpa * 0.0295299830714).ToString("0.00", CultureInfo.InvariantCulture)} inches of mercury";

    public static string PrecipitationAmount(double amount, UnitSystem u)
    {
        if (u == UnitSystem.Metric)
        {
            var mm = Math.Round(amount);
            if (mm < 1) return "less than a millimetre";
            return mm == 1 ? "1 millimetre" : $"{(int)mm} millimetres";
        }
        var inches = Math.Round(amount, 1);
        if (inches < 0.1) return "less than a tenth of an inch";
        return $"{inches.ToString("0.#", CultureInfo.InvariantCulture)} {(inches == 1 ? "inch" : "inches")}";
    }
}
