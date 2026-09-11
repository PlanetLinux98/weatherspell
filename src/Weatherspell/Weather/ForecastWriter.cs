using System.Globalization;
using System.Text;

namespace Weatherspell.Weather;

internal sealed record Section(string Heading, IReadOnlyList<string> Paragraphs);

internal sealed record WriterOptions(
    DateTimeOffset Now,
    TimeZoneInfo PcZone,
    string TimePattern,
    CultureInfo Culture)
{
    public static WriterOptions Default() =>
        new(DateTimeOffset.UtcNow, TimeZoneInfo.Local, Clock.WindowsTimePattern(), CultureInfo.CurrentCulture);
}

// Turns a Forecast into sections of sentences meant to be listened to:
// words instead of symbols, whole degrees, one idea per sentence.
internal static class ForecastWriter
{
    public static IReadOnlyList<Section> Write(Forecast f, WriterOptions o)
    {
        var clock = new Clock(f.UtcOffset, o.PcZone, o.TimePattern, o.Culture);
        var nowLocal = o.Now.ToOffset(f.UtcOffset).DateTime;

        var sections = new List<Section>
        {
            new("Alerts", ["Alert checking is not part of this version yet."]),
            RightNow(f, o, clock),
        };

        var rest = RestOfToday(f, nowLocal);
        if (rest is not null) sections.Add(rest);

        foreach (var day in f.Days)
        {
            if (day.Date <= nowLocal.Date) continue;
            var following = f.Days.FirstOrDefault(d => d.Date == day.Date.AddDays(1));
            sections.Add(new Section(clock.DayHeading(day.Date), DayAndNight(day, following, f, o.Culture)));
        }

        var sun = SunAndUv(f, nowLocal, clock);
        if (sun is not null) sections.Add(sun);

        sections.Add(Details(f, nowLocal));
        sections.Add(new Section("Sources", [$"Forecast and current conditions: {f.SourceNote}."]));
        return sections;
    }

    private static Section RightNow(Forecast f, WriterOptions o, Clock clock)
    {
        var c = f.Current;
        var sb = new StringBuilder();
        sb.Append($"As of {clock.Time(c.LocalTime)}, it's {Units.Degrees(c.Temperature)} and {WeatherCodes.Describe(c.WeatherCode, c.IsDay)}");
        if (Round(c.FeelsLike) != Round(c.Temperature))
        {
            sb.Append($", feeling like {Units.DegreesBare(c.FeelsLike)}");
        }
        sb.Append(". ");
        sb.Append(WindSentence(c.WindSpeed, c.WindDirection, c.WindGusts, f.Units));
        sb.Append($" Humidity {c.Humidity} percent.");

        var age = o.Now - f.FetchedAt;
        return new Section("Right now", [sb.ToString(), $"Updated {Clock.Age(age)}."]);
    }

    private static string WindSentence(double speed, int direction, double gusts, UnitSystem units)
    {
        if (speed < 2) return "The air is calm.";
        var s = $"Wind from the {Compass.FromDegrees(direction)} at {Units.Speed(speed, units)}";
        if (gusts - speed >= GustMargin(units))
        {
            s += $", gusting to {Units.SpeedBare(gusts)}";
        }
        return s + ".";
    }

    // Gusts get a mention when they add something a listener would feel.
    private static double GustMargin(UnitSystem units) => units == UnitSystem.Metric ? 10 : 6;
    private static double BreezyThreshold(UnitSystem units) => units == UnitSystem.Metric ? 10 : 6;

    private sealed record Part(string Label, DateTime Start, DateTime End);

    private static Section? RestOfToday(Forecast f, DateTime nowLocal)
    {
        var today = nowLocal.Date;
        var parts = new List<Part>();
        var five = today.AddHours(5);
        var noon = today.AddHours(12);
        var five_pm = today.AddHours(17);
        var nine_pm = today.AddHours(21);
        var five_tomorrow = today.AddDays(1).AddHours(5);

        // The hour in progress counts: its point is the nearest thing to "now".
        var hourFloor = new DateTime(nowLocal.Year, nowLocal.Month, nowLocal.Day, nowLocal.Hour, 0, 0);
        if (nowLocal < five) parts.Add(new Part("Early this morning", hourFloor, five));
        if (nowLocal < noon) parts.Add(new Part("This morning", Max(hourFloor, five), noon));
        if (nowLocal < five_pm) parts.Add(new Part("This afternoon", Max(hourFloor, noon), five_pm));
        if (nowLocal < nine_pm) parts.Add(new Part("This evening", Max(hourFloor, five_pm), nine_pm));
        parts.Add(new Part("Overnight", Max(hourFloor, nine_pm), five_tomorrow));

        var paragraphs = new List<string>();

        var todayDay = f.Days.FirstOrDefault(d => d.Date == today);
        var overnight = f.Hours.Where(h => h.LocalTime >= Max(hourFloor, nine_pm) && h.LocalTime < five_tomorrow).ToList();
        if (todayDay is not null && overnight.Count > 0)
        {
            paragraphs.Add($"Today's high {Units.DegreesBare(todayDay.High)}, tonight's low {Units.DegreesBare(overnight.Min(h => h.Temperature))}.");
        }

        foreach (var part in parts)
        {
            var hours = f.Hours.Where(h => h.LocalTime >= part.Start && h.LocalTime < part.End).ToList();
            if (hours.Count == 0) continue;
            paragraphs.Add(PartSentence(part.Label, hours, f.Units));
        }

        return paragraphs.Count == 0 ? null : new Section("Rest of today", paragraphs);
    }

    private static string PartSentence(string label, List<HourPoint> hours, UnitSystem units)
    {
        var pieces = new List<string> { ConditionPhrase(hours) };

        var first = hours[0].Temperature;
        var last = hours[hours.Count - 1].Temperature;
        if (Math.Abs(Round(last) - Round(first)) <= 2)
        {
            pieces.Add($"around {Units.Degrees(hours.Average(h => h.Temperature))}");
        }
        else if (last < first)
        {
            pieces.Add($"cooling from {Units.DegreesBare(first)} to {Units.Degrees(last)}");
        }
        else
        {
            pieces.Add($"warming from {Units.DegreesBare(first)} to {Units.Degrees(last)}");
        }

        var chance = hours.Max(h => h.PrecipitationProbability ?? 0);
        if (chance >= MinimumMentionedChance)
        {
            pieces.Add(ChancePhrase(hours, chance, units));
        }

        var wind = hours.Max(h => h.WindSpeed);
        pieces.Add(wind >= BreezyThreshold(units) ? $"wind up to {Units.Speed(wind, units)}" : "light wind");

        return $"{label}: {string.Join(", ", pieces)}.";
    }

    // The most frequent condition carries the sentence; a rarer but more
    // serious one (a shower in an otherwise cloudy afternoon) is added "at
    // times" rather than allowed to define the whole period.
    private static string ConditionPhrase(List<HourPoint> hours)
    {
        var common = hours.GroupBy(h => h.WeatherCode)
            .OrderByDescending(g => g.Count())
            .ThenByDescending(g => WeatherCodes.Severity(g.Key))
            .First().Key;
        var worst = hours.OrderByDescending(h => WeatherCodes.Severity(h.WeatherCode)).First().WeatherCode;
        if (worst != common && WeatherCodes.Severity(worst) > WeatherCodes.Severity(common) && WeatherCodes.IsPrecipitation(worst))
        {
            return $"{WeatherCodes.Describe(common)} with {WeatherCodes.Describe(worst)} at times";
        }
        return WeatherCodes.Describe(common);
    }

    // Twelve-hour periods, as the official services write them: "Saturday"
    // from sunrise to sunset, "Saturday night" until the next sunrise. Built
    // from the hourly data; the daily aggregates are the fallback when the
    // hours run out near the end of the horizon.
    private static IReadOnlyList<string> DayAndNight(DayForecast day, DayForecast? following, Forecast f, CultureInfo culture)
    {
        var dayStart = Floor(day.Sunrise ?? day.Date.AddHours(6));
        var dayEnd = Floor(day.Sunset ?? day.Date.AddHours(18));
        var nightEnd = Floor(following?.Sunrise ?? day.Date.AddDays(1).AddHours(6));
        var dayHours = f.Hours.Where(h => h.LocalTime >= dayStart && h.LocalTime < dayEnd).ToList();
        var nightHours = f.Hours.Where(h => h.LocalTime >= dayEnd && h.LocalTime < nightEnd).ToList();

        if (dayHours.Count < 3)
        {
            return [DayParagraph(day, f.Units)];
        }

        var weekday = day.Date.ToString("dddd", culture);
        var paragraphs = new List<string> { $"{weekday}: {DaySentence(dayHours, day, f.Units)}." };
        if (nightHours.Count >= 3)
        {
            paragraphs.Add($"{weekday} night: {NightSentence(nightHours, f.Units)}.");
        }
        return paragraphs;
    }

    private static string DaySentence(List<HourPoint> hours, DayForecast day, UnitSystem units)
    {
        var pieces = new List<string> { ConditionPhrase(hours) };
        var high = hours.Max(h => h.Temperature);
        pieces.Add($"high {Units.DegreesBare(high)}");
        if (Math.Abs(Round(day.FeelsLikeHigh) - Round(high)) >= 3)
        {
            pieces.Add($"feeling like {Units.DegreesBare(day.FeelsLikeHigh)} at the warmest");
        }

        var chance = hours.Max(h => h.PrecipitationProbability ?? 0);
        if (chance >= MinimumMentionedChance)
        {
            var piece = ChancePhrase(hours, chance, units) + Timing(hours, chance);
            if (day.PrecipitationSum >= MinimumMentionedAmount(units))
            {
                piece += $", about {Units.PrecipitationAmount(day.PrecipitationSum, units)}";
            }
            pieces.Add(piece);
        }
        if (day.SnowfallSum >= (units == UnitSystem.Metric ? 1 : 0.5))
        {
            pieces.Add(units == UnitSystem.Metric
                ? $"snowfall around {(int)Math.Round(day.SnowfallSum)} centimetres"
                : $"snowfall around {day.SnowfallSum.ToString("0.#", CultureInfo.InvariantCulture)} inches");
        }
        pieces.Add(WindPhrase(hours, units));
        return string.Join(", ", pieces);
    }

    private static string NightSentence(List<HourPoint> hours, UnitSystem units)
    {
        var pieces = new List<string>
        {
            ConditionPhrase(hours),
            $"low {Units.DegreesBare(hours.Min(h => h.Temperature))}",
        };
        var chance = hours.Max(h => h.PrecipitationProbability ?? 0);
        if (chance >= MinimumMentionedChance)
        {
            pieces.Add(ChancePhrase(hours, chance, units));
        }
        pieces.Add(WindPhrase(hours, units));
        return string.Join(", ", pieces);
    }

    // "30 percent chance of rain": chances rounded to tens, as forecasts say
    // them, and the word chosen from the hour's conditions or, failing a
    // precipitation code, from how cold the period is.
    private static string ChancePhrase(List<HourPoint> hours, int chance, UnitSystem units)
    {
        var rounded = (int)Math.Round(chance / 10.0, MidpointRounding.AwayFromZero) * 10;
        var worst = hours.OrderByDescending(h => WeatherCodes.Severity(h.WeatherCode)).First().WeatherCode;
        var word = WeatherCodes.IsPrecipitation(worst) ? WeatherCodes.PrecipitationWord(worst) : PrecipitationWordByTemperature(hours, units);
        return $"{rounded} percent chance of {word}";
    }

    private static string PrecipitationWordByTemperature(List<HourPoint> hours, UnitSystem units)
    {
        var coldest = hours.Min(h => h.Temperature);
        var warmest = hours.Max(h => h.Temperature);
        var (rainAbove, snowBelow) = units == UnitSystem.Metric ? (3.0, -1.0) : (37.0, 30.0);
        if (coldest > rainAbove) return "rain";
        if (warmest < snowBelow) return "snow";
        return "precipitation";
    }

    // Mentioned from 15 percent, which rounds to "20 percent"; below that a
    // chance is noise.
    private const int MinimumMentionedChance = 15;

    // "in the afternoon" when the likely hours all sit in one part of the
    // day; nothing when the chance is spread across the day.
    private static string Timing(List<HourPoint> hours, int chance)
    {
        var likely = hours.Where(h => (h.PrecipitationProbability ?? 0) >= chance - 10).Select(h => PartOfDay(h.LocalTime.Hour)).Distinct().ToList();
        return likely.Count == 1 ? $" in the {likely[0]}" : "";
    }

    private static string PartOfDay(int hour) => hour < 12 ? "morning" : (hour < 17 ? "afternoon" : "evening");

    // Direction taken at the strongest hour, so "from the southwest up to 25"
    // describes one moment rather than an average of shifting winds.
    private static string WindPhrase(List<HourPoint> hours, UnitSystem units)
    {
        var strongest = hours.OrderByDescending(h => h.WindSpeed).First();
        if (strongest.WindSpeed < BreezyThreshold(units)) return "light wind";
        var s = $"wind from the {Compass.FromDegrees(strongest.WindDirection)} up to {Units.Speed(strongest.WindSpeed, units)}";
        var gust = hours.Max(h => h.WindGusts);
        if (gust - strongest.WindSpeed >= GustMargin(units))
        {
            s += $", gusting to {Units.SpeedBare(gust)}";
        }
        return s;
    }

    private static DateTime Floor(DateTime t) => new(t.Year, t.Month, t.Day, t.Hour, 0, 0);

    private static string DayParagraph(DayForecast d, UnitSystem units)
    {
        var sb = new StringBuilder();
        sb.Append($"{WeatherCodes.DescribeSentence(d.WeatherCode)}. High {Units.DegreesBare(d.High)}, low {Units.DegreesBare(d.Low)}.");
        if (Math.Abs(Round(d.FeelsLikeHigh) - Round(d.High)) >= 3)
        {
            sb.Append($" Feeling like {Units.DegreesBare(d.FeelsLikeHigh)} at the warmest.");
        }
        var chance = d.PrecipitationProbabilityMax ?? 0;
        if (chance >= MinimumMentionedChance)
        {
            var rounded = (int)Math.Round(chance / 10.0, MidpointRounding.AwayFromZero) * 10;
            sb.Append($" {rounded} percent chance of {WeatherCodes.PrecipitationWord(d.WeatherCode)}");
            if (d.PrecipitationSum >= MinimumMentionedAmount(units))
            {
                sb.Append($", about {Units.PrecipitationAmount(d.PrecipitationSum, units)}");
            }
            sb.Append('.');
        }
        if (d.SnowfallSum >= (units == UnitSystem.Metric ? 1 : 0.5))
        {
            sb.Append(units == UnitSystem.Metric
                ? $" Snowfall around {(int)Math.Round(d.SnowfallSum)} centimetres."
                : $" Snowfall around {d.SnowfallSum.ToString("0.#", CultureInfo.InvariantCulture)} inches.");
        }
        sb.Append($" Wind from the {Compass.FromDegrees(d.WindDirectionDominant)} up to {Units.Speed(d.WindSpeedMax, units)}");
        if (d.WindGustsMax - d.WindSpeedMax >= GustMargin(units))
        {
            sb.Append($", gusting to {Units.SpeedBare(d.WindGustsMax)}");
        }
        sb.Append('.');
        return sb.ToString();
    }

    private static double MinimumMentionedAmount(UnitSystem units) => units == UnitSystem.Metric ? 1 : 0.05;

    private static Section? SunAndUv(Forecast f, DateTime nowLocal, Clock clock)
    {
        var today = f.Days.FirstOrDefault(d => d.Date == nowLocal.Date);
        if (today is null) return null;
        var paragraphs = new List<string>();
        if (today.Sunrise is DateTime rise && today.Sunset is DateTime set)
        {
            var rises = nowLocal < rise ? "rises" : "rose";
            var sets = nowLocal < set ? "sets" : "set";
            var s = $"The sun {rises} at {clock.Time(rise)} and {sets} at {clock.Time(set)}";
            if (today.DaylightSeconds is double daylight)
            {
                s += $", {Clock.Duration(daylight)} of daylight";
            }
            paragraphs.Add(s + ".");
        }
        if (today.UvIndexMax is double uv)
        {
            var level = (int)Math.Round(uv);
            paragraphs.Add($"UV index {level}, {UvBand(level)}.");
        }
        return paragraphs.Count == 0 ? null : new Section("Sun and UV", paragraphs);
    }

    private static string UvBand(int uv) => uv switch
    {
        < 3 => "low",
        < 6 => "moderate",
        < 8 => "high",
        < 11 => "very high",
        _ => "extreme",
    };

    private static Section Details(Forecast f, DateTime nowLocal)
    {
        var c = f.Current;
        // Dew point and visibility are hourly-only; take the hour nearest now.
        var nearest = f.Hours.OrderBy(h => Math.Abs((h.LocalTime - nowLocal).Ticks)).FirstOrDefault();
        var pieces = new List<string> { $"Humidity {c.Humidity} percent" };
        if (nearest?.DewPoint is double dew) pieces.Add($"dew point {Units.Degrees(dew)}");
        if (c.PressureHpa > 0) pieces.Add($"pressure {Units.Pressure(c.PressureHpa, f.Units)}");
        if (nearest?.VisibilityMetres is double vis) pieces.Add($"visibility {Units.Distance(vis, f.Units)}");
        pieces.Add($"cloud cover {c.CloudCover} percent");
        return new Section("Details", [Capitalize(string.Join(", ", pieces)) + "."]);
    }

    private static string Capitalize(string s) => char.ToUpperInvariant(s[0]) + s.Substring(1);
    private static int Round(double v) => (int)Math.Round(v, MidpointRounding.AwayFromZero);
    private static DateTime Max(DateTime a, DateTime b) => a > b ? a : b;
}
