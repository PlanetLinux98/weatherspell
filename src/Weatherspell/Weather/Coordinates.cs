using System.Globalization;
using System.Text;
using System.Text.RegularExpressions;

namespace Weatherspell.Weather;

// Coordinates typed or pasted into the Add Location search field. A reading
// is either a point or, when the text was plainly meant as coordinates but
// cannot be one, the reason to tell the user.
internal sealed record CoordinateReading(double Latitude, double Longitude, string? Problem = null)
{
    public static CoordinateReading Fail(string problem) => new(0, 0, problem);
}

// Reads coordinates in as many of the ways people write and paste them as
// can be told apart from a place name or postal code: decimals with signs
// or compass letters (English, French and Spanish), degrees with minutes
// and seconds, degrees and decimal minutes as GPS units write them, decimal
// commas, either order when letters or labels say which is which, and the
// map links people copy. Anything that is not wholly coordinates returns
// null and goes to the place search, so a ZIP code, "SW1A 1AA" or a
// Japanese "060-0001" is never mistaken for a point.
internal static class Coordinates
{
    private const string Example = "44.54, -78.54";
    private static readonly CultureInfo Inv = CultureInfo.InvariantCulture;

    public static CoordinateReading? Read(string text)
    {
        var s = Normalize(text);
        if (s.Length == 0) return null;
        if (IsLink(s)) return ReadLink(s);

        var tokens = Tokenize(DecimalCommas(s));
        if (tokens is null) return null;
        // Only text that can only be coordinates earns an explanation when
        // it fails. Whole numbers with nothing else count only when a comma
        // or semicolon parts them: "114 55" and "110 00" are Swedish and
        // Czech postal codes.
        var evident = tokens.Any(t => t.Kind is Kind.Hemisphere or Kind.Label or Kind.Degrees or Kind.Minutes or Kind.Seconds
            || (t.Kind == Kind.Number && t.HasDecimals));
        if (!evident && Indexes(tokens, Kind.Separator).Count != 1) return null;
        var unreadable = evident ? CoordinateReading.Fail($"Couldn't read those coordinates. Type a latitude and then a longitude, such as {Example}.") : null;

        var groups = Split(tokens);
        if (groups is null) return unreadable;
        var first = Component(groups.Value.First);
        var second = Component(groups.Value.Second);
        if (first is null || second is null) return unreadable;
        if (first.Problem is not null) return evident ? CoordinateReading.Fail(first.Problem) : null;
        if (second.Problem is not null) return evident ? CoordinateReading.Fail(second.Problem) : null;

        Part lat, lon;
        if (first.Axis is Axis a && second.Axis is Axis b)
        {
            if (a == b)
            {
                return evident ? CoordinateReading.Fail(a == Axis.Latitude
                    ? $"Couldn't read those coordinates: both are latitudes. Type a latitude and then a longitude, such as {Example}."
                    : $"Couldn't read those coordinates: both are longitudes. Type a latitude and then a longitude, such as {Example}.") : null;
            }
            (lat, lon) = a == Axis.Latitude ? (first, second) : (second, first);
        }
        else if (first.Axis is Axis only)
        {
            (lat, lon) = only == Axis.Latitude ? (first, second) : (second, first);
        }
        else if (second.Axis is Axis other)
        {
            (lat, lon) = other == Axis.Longitude ? (first, second) : (second, first);
        }
        else if (Math.Abs(first.Value) > 90 && Math.Abs(first.Value) <= 180 && Math.Abs(second.Value) <= 90)
        {
            // Longitude first, as GeoJSON and some GIS tools write it: the
            // first number cannot be a latitude and the second can.
            (lat, lon) = (second, first);
        }
        else
        {
            (lat, lon) = (first, second);
        }

        if (Math.Abs(lat.Value) > 90)
        {
            return evident ? CoordinateReading.Fail($"Couldn't use {Number(lat.Value)} as a latitude: it must be from 90 south to 90 north.") : null;
        }
        if (Math.Abs(lon.Value) > 180)
        {
            return evident ? CoordinateReading.Fail($"Couldn't use {Number(lon.Value)} as a longitude: it must be from 180 west to 180 east.") : null;
        }
        return new CoordinateReading(lat.Value, lon.Value);
    }

    // "44.54 north, 78.54 west": how a point is said in results and used as
    // the name of a point nothing nearby names. Four places is about ten
    // metres, finer than any forecast.
    public static string Words(double latitude, double longitude) =>
        $"{Word(latitude, "north", "south")}, {Word(longitude, "east", "west")}";

    private static string Word(double value, string positive, string negative)
    {
        var rounded = Math.Round(value, 4);
        return $"{Math.Abs(rounded).ToString("0.####", Inv)} {(rounded < 0 ? negative : positive)}";
    }

    private static string Number(double value) => value.ToString("0.######", Inv);

    // Typographic marks as they arrive from web pages and word processors
    // fold into the plain ones: primes and curly quotes into minutes and
    // seconds, the masculine ordinal and ring into degrees, and the minus
    // sign and dashes into a hyphen-minus.
    private static string Normalize(string text)
    {
        var b = new StringBuilder(text.Length);
        foreach (var c in text.Trim())
        {
            b.Append(c switch
            {
                '\u2212' or '\u2010' or '\u2011' or '\u2012' or '\u2013' or '\u2014' or '\uFE63' or '\uFF0D' => '-',
                '\u2032' or '\u2018' or '\u2019' or '\u00B4' or '`' => '\'',
                '\u2033' or '\u201C' or '\u201D' => '"',
                '\u00BA' or '\u02DA' => '\u00B0',
                '\u00A0' or '\u202F' or '\t' => ' ',
                _ => char.ToLowerInvariant(c),
            });
        }
        return b.ToString().Replace("''", "\"").Trim();
    }

    // Decimal commas ("44,54 -78,54", as a French or German Windows writes
    // them) are commas between digits in text with no decimal point, and
    // only when something else separates the two numbers; "44,78" stays a
    // separator.
    private static string DecimalCommas(string s)
    {
        if (s.IndexOf('.') >= 0 || !Regex.IsMatch(s, @"\d,\d")) return s;
        var otherSeparator = s.IndexOf(';') >= 0 || s.Any(char.IsWhiteSpace) || Regex.IsMatch(s, @"(?<!\d),|,(?!\d)");
        return otherSeparator ? Regex.Replace(s, @"(?<=\d),(?=\d)", ".") : s;
    }

    private enum Kind { Number, Sign, Degrees, Minutes, Seconds, Hemisphere, Label, Separator }

    private enum Axis { Latitude, Longitude }

    private sealed record Token(Kind Kind, double Value = 0, bool HasDecimals = false, bool Negative = false, Axis? Axis = null);

    private static readonly Dictionary<string, Token> Vocabulary = new()
    {
        ["n"] = Hemi(Axis.Latitude, false), ["north"] = Hemi(Axis.Latitude, false), ["nord"] = Hemi(Axis.Latitude, false), ["norte"] = Hemi(Axis.Latitude, false),
        ["s"] = Hemi(Axis.Latitude, true), ["south"] = Hemi(Axis.Latitude, true), ["sud"] = Hemi(Axis.Latitude, true), ["sur"] = Hemi(Axis.Latitude, true),
        ["e"] = Hemi(Axis.Longitude, false), ["east"] = Hemi(Axis.Longitude, false), ["est"] = Hemi(Axis.Longitude, false), ["este"] = Hemi(Axis.Longitude, false),
        ["w"] = Hemi(Axis.Longitude, true), ["west"] = Hemi(Axis.Longitude, true), ["o"] = Hemi(Axis.Longitude, true), ["ouest"] = Hemi(Axis.Longitude, true), ["oeste"] = Hemi(Axis.Longitude, true),
        ["lat"] = new(Kind.Label, Axis: Axis.Latitude), ["latitude"] = new(Kind.Label, Axis: Axis.Latitude),
        ["lon"] = new(Kind.Label, Axis: Axis.Longitude), ["long"] = new(Kind.Label, Axis: Axis.Longitude), ["lng"] = new(Kind.Label, Axis: Axis.Longitude), ["longitude"] = new(Kind.Label, Axis: Axis.Longitude),
        // "s" is south, so seconds are only ever spelt out or marked.
        ["d"] = new(Kind.Degrees), ["deg"] = new(Kind.Degrees), ["degree"] = new(Kind.Degrees), ["degrees"] = new(Kind.Degrees),
        ["m"] = new(Kind.Minutes), ["min"] = new(Kind.Minutes), ["mins"] = new(Kind.Minutes), ["minute"] = new(Kind.Minutes), ["minutes"] = new(Kind.Minutes),
        ["sec"] = new(Kind.Seconds), ["secs"] = new(Kind.Seconds), ["second"] = new(Kind.Seconds), ["seconds"] = new(Kind.Seconds),
    };

    private static Token Hemi(Axis axis, bool negative) => new(Kind.Hemisphere, Negative: negative, Axis: axis);

    // Null when anything in the text is not part of a coordinate.
    private static List<Token>? Tokenize(string s)
    {
        var tokens = new List<Token>();
        var i = 0;
        while (i < s.Length)
        {
            var c = s[i];
            if (char.IsWhiteSpace(c) || c is ':' or '=' or '&' or '(' or ')' or '[' or ']')
            {
                // Colons also stand between degrees, minutes and seconds
                // ("44:32:24"), where the numbers' order is enough.
                i++;
            }
            else if (c is '-' or '+' && i > 0 && char.IsDigit(s[i - 1]))
            {
                // A sign against a digit is a postal code's hyphen:
                // "060-0001" (Japan), "01310-100" (Brazil), ZIP+4.
                return null;
            }
            else if (char.IsDigit(c) || (c == '.' && i + 1 < s.Length && char.IsDigit(s[i + 1])))
            {
                var start = i;
                while (i < s.Length && char.IsDigit(s[i])) i++;
                var decimals = false;
                if (i < s.Length && s[i] == '.')
                {
                    i++;
                    while (i < s.Length && char.IsDigit(s[i])) { i++; decimals = true; }
                }
                var digits = s.Substring(start, i - start);
                if (!double.TryParse(digits, NumberStyles.AllowDecimalPoint, Inv, out var value)) return null;
                tokens.Add(new Token(Kind.Number, value, decimals));
            }
            else if (c is '-' or '+')
            {
                tokens.Add(new Token(Kind.Sign, Negative: c == '-'));
                i++;
            }
            else if (c == '\u00B0') { tokens.Add(new Token(Kind.Degrees)); i++; }
            else if (c == '\'') { tokens.Add(new Token(Kind.Minutes)); i++; }
            else if (c == '"') { tokens.Add(new Token(Kind.Seconds)); i++; }
            else if (c is ',' or ';' or '/') { tokens.Add(new Token(Kind.Separator)); i++; }
            else if (c >= 'a' && c <= 'z')
            {
                var start = i;
                while (i < s.Length && s[i] >= 'a' && s[i] <= 'z') i++;
                if (!Vocabulary.TryGetValue(s.Substring(start, i - start), out var word)) return null;
                tokens.Add(word);
            }
            else
            {
                return null;
            }
        }
        // One number is never a point: "N1" and "M1" are postal districts.
        return tokens.Count(t => t.Kind == Kind.Number) >= 2 ? tokens : null;
    }

    // The two halves: at the one separator; else where the second label or
    // compass letter begins; else at the second degree mark or the sign
    // after the first; else plain numbers halved (2, 4 or 6 of them).
    private static (List<Token> First, List<Token> Second)? Split(List<Token> t)
    {
        var separators = Indexes(t, Kind.Separator);
        if (separators.Count > 1) return null;
        if (separators.Count == 1) return At(t, separators[0], skip: 1);

        var labels = Indexes(t, Kind.Label);
        if (labels.Count == 2) return labels[0] == 0 ? At(t, labels[1]) : null;

        var hemis = Indexes(t, Kind.Hemisphere);
        if (hemis.Count == 2)
        {
            if (hemis[0] == 0) return At(t, hemis[1]);
            if (hemis[1] == t.Count - 1) return At(t, hemis[0] + 1);
            return null;
        }
        if (hemis.Count > 2) return null;

        var prefixStyle = t[0].Kind is Kind.Hemisphere or Kind.Label;
        int? secondStart = null;
        var degrees = Indexes(t, Kind.Degrees);
        var signs = Indexes(t, Kind.Sign).Where(i => i > 0).ToList();
        if (degrees.Count == 2)
        {
            secondStart = degrees[1] - 1;
        }
        else if (signs.Count == 1)
        {
            secondStart = signs[0] + 1;
        }
        else if (signs.Count == 0)
        {
            var numbers = Indexes(t, Kind.Number);
            if (numbers.Count is 2 or 4 or 6) secondStart = numbers[numbers.Count / 2];
        }
        if (secondStart is not int start || start <= 0 || t[start].Kind != Kind.Number) return null;

        // Back over the sign, and over a compass letter or label that opens
        // the second half when the first half opened with one too.
        if (t[start - 1].Kind == Kind.Sign) start--;
        if (start > 0 && prefixStyle && t[start - 1].Kind is Kind.Hemisphere or Kind.Label) start--;
        return start > 0 ? At(t, start) : null;
    }

    private static List<int> Indexes(List<Token> t, Kind kind) =>
        Enumerable.Range(0, t.Count).Where(i => t[i].Kind == kind).ToList();

    private static (List<Token>, List<Token>)? At(List<Token> t, int index, int skip = 0) =>
        index <= 0 || index + skip >= t.Count ? null : (t.GetRange(0, index), t.GetRange(index + skip, t.Count - index - skip));

    private sealed record Part(double Value, Axis? Axis, string? Problem = null);

    // One half: [label] [letter] [sign] degrees [mark] [minutes [mark] [seconds [mark]]] [letter].
    private static Part? Component(List<Token> t)
    {
        var i = 0;
        Axis? axis = null;
        var negative = false;
        Token? Next(Kind kind) => i < t.Count && t[i].Kind == kind ? t[i++] : null;

        if (Next(Kind.Label) is Token label) axis = label.Axis;
        var prefix = Next(Kind.Hemisphere);
        if (Next(Kind.Sign) is Token sign) negative = sign.Negative;
        if (Next(Kind.Number) is not Token degrees) return null;
        Next(Kind.Degrees);
        var minutes = Next(Kind.Number);
        if (minutes is not null) Next(Kind.Minutes);
        var seconds = minutes is null ? null : Next(Kind.Number);
        if (seconds is not null) Next(Kind.Seconds);
        var suffix = prefix is null ? Next(Kind.Hemisphere) : null;
        if (i != t.Count) return null;

        var hemi = prefix ?? suffix;
        if (hemi is not null)
        {
            if (axis is not null && axis != hemi.Axis) return null;
            // "-78.54 W" says west twice; "-44 N" contradicts itself.
            if (negative && !hemi.Negative) return null;
            axis = hemi.Axis;
            negative = hemi.Negative;
        }

        var value = degrees.Value;
        if (minutes is not null)
        {
            // Only the last number may have decimals: "44.5 32" is not a reading.
            if (degrees.HasDecimals || (seconds is not null && minutes.HasDecimals)) return null;
            if (minutes.Value >= 60 || (seconds?.Value ?? 0) >= 60)
            {
                return new Part(0, axis, "Couldn't read those coordinates: minutes and seconds must be under 60.");
            }
            value += minutes.Value / 60 + (seconds?.Value ?? 0) / 3600;
        }
        return new Part(negative ? -value : value, axis);
    }

    // Map links: the dropped pin where the link has one, else the view's
    // centre. Short links (maps.app.goo.gl) carry no coordinates at all.
    private static bool IsLink(string s) =>
        s.StartsWith("geo:", StringComparison.Ordinal) || s.Contains("://") || s.StartsWith("www.", StringComparison.Ordinal)
        || Regex.IsMatch(s, @"^[a-z0-9-]+(\.[a-z0-9-]+)*\.[a-z]{2,}/");

    private const string N = @"([-+]?\d+(?:\.\d+)?)";
    private static readonly Regex[] LinkPatterns =
    [
        new(@"^geo:\s*" + N + @"\s*,\s*" + N),
        new(@"!3d" + N + "!4d" + N),
        new(@"[?&](?:q|query|ll|sll|daddr|destination|center|coordinate|where1)=[\s+]*(?:loc:)?[\s+]*" + N + @"[\s+]*,[\s+]*" + N),
        new(@"[?&]mlat=" + N + @"&mlon=" + N),
        new(@"[?&]lat=" + N + @"&(?:lon|lng)=" + N),
        new(@"#map=\d+(?:\.\d+)?/" + N + "/" + N),
        new(@"[?&]cp=" + N + "~" + N),
        new(@"@" + N + "," + N),
    ];

    private static CoordinateReading ReadLink(string s)
    {
        var url = Uri.UnescapeDataString(s);
        foreach (var pattern in LinkPatterns)
        {
            var m = pattern.Match(url);
            if (!m.Success) continue;
            var lat = double.Parse(m.Groups[1].Value, NumberStyles.Float, Inv);
            var lon = double.Parse(m.Groups[2].Value, NumberStyles.Float, Inv);
            if (Math.Abs(lat) > 90) return CoordinateReading.Fail($"Couldn't use {Number(lat)} as a latitude: it must be from 90 south to 90 north.");
            if (Math.Abs(lon) > 180) return CoordinateReading.Fail($"Couldn't use {Number(lon)} as a longitude: it must be from 180 west to 180 east.");
            return new CoordinateReading(lat, lon);
        }
        return CoordinateReading.Fail("That link has no coordinates in it. Copy the coordinates themselves, or search for the place by name.");
    }
}
