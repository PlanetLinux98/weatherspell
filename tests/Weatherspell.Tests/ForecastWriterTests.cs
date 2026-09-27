using System.Globalization;
using Weatherspell.Weather;
using Weatherspell.Weather.OpenMeteo;
using Xunit;

namespace Weatherspell.Tests;

public class ForecastWriterTests
{
    private static readonly Location Toronto = new("Toronto", "Ontario", "Canada", 43.65, -79.38, "America/Toronto");
    private static readonly TimeSpan Eastern = TimeSpan.FromHours(-4);

    // 2:45 pm local on Thursday 11 September 2026, PC clock in the same zone.
    private static readonly DateTimeOffset Now = new(2026, 9, 11, 14, 45, 0, Eastern);

    private static WriterOptions Options(TimeZoneInfo? pcZone = null) =>
        new(Now, pcZone ?? TimeZoneInfo.CreateCustomTimeZone("test-eastern", Eastern, "Test Eastern", "Test Eastern"), "h:mm tt", CultureInfo.InvariantCulture);

    private static Forecast Sample(UnitSystem units = UnitSystem.Metric, int currentCode = 1)
    {
        var current = new CurrentConditions(
            LocalTime: new DateTime(2026, 9, 11, 14, 30, 0),
            Temperature: 21.4, FeelsLike: 19.6, Humidity: 52, WeatherCode: currentCode, IsDay: true,
            WindSpeed: 12, WindDirection: 225, WindGusts: 30, Precipitation: 0, CloudCover: 40, PressureHpa: 1016.8);

        var hours = new List<HourPoint>();
        // Today (the 11th): warms to 24.5 at 15:00, then cools a degree an
        // hour; one shower hour at 20:00 inside a partly cloudy evening.
        for (var h = 0; h < 24; h++)
        {
            var t = new DateTime(2026, 9, 11, 0, 0, 0).AddHours(h);
            var temp = h < 15 ? 20 + h * 0.3 : 24.5 - (h - 15);
            var code = h == 20 ? 80 : 2;
            var prob = h == 20 ? 60 : (h is 19 or 21 ? 30 : 0);
            hours.Add(new HourPoint(t, temp, prob, h == 20 ? 0.6 : 0, code, 8 + (h % 5) * 3, 225, 20, 24100, 11.2, 55, h is > 6 and < 20));
        }
        // Saturday the 12th: a cool clear night, partly cloudy day with showers
        // at 13:00 and 14:00, clearing evening. Sunday has no hourly data, so
        // it falls back to the daily aggregates.
        double[] saturday = [13, 12.5, 12, 11.5, 11.5, 11, 11, 12, 14, 16, 18, 20, 21.5, 22.5, 23, 23.2, 22.8, 22, 21, 19.5, 18, 17, 16, 15];
        for (var h = 0; h < 24; h++)
        {
            var t = new DateTime(2026, 9, 12, 0, 0, 0).AddHours(h);
            var code = h <= 6 || h >= 19 ? 1 : (h is 13 or 14 ? 80 : 2);
            var prob = h is 13 ? 55 : (h is 14 ? 60 : (h is 12 or 15 ? 30 : 0));
            var wind = 10 + (h % 6) * 3;
            hours.Add(new HourPoint(t, saturday[h], prob, h is 13 or 14 ? 2 : 0, code, wind, 225, wind * 1.8, 20000, 10, 60, h is > 6 and < 20));
        }

        var days = new List<DayForecast>
        {
            new(new DateTime(2026, 9, 11), 2, 24.6, 14.2, 25.0, 13.0, new DateTime(2026, 9, 11, 6, 52, 0), new DateTime(2026, 9, 11, 19, 34, 0), 45720, 5.75, 1.2, 60, 0, 18, 32, 225),
            new(new DateTime(2026, 9, 12), 61, 23.2, 11.0, 24.0, 11.0, new DateTime(2026, 9, 12, 6, 53, 0), new DateTime(2026, 9, 12, 19, 32, 0), 45600, 3.1, 8.4, 80, 0, 22, 41, 45),
            new(new DateTime(2026, 9, 13), 0, 22.0, 9.5, 21.0, 8.0, new DateTime(2026, 9, 13, 6, 54, 0), new DateTime(2026, 9, 13, 19, 30, 0), 45480, 6.4, 0, 0, 0, 9, 14, 315),
        };

        return new Forecast(Toronto, Now.AddMinutes(-3), Eastern, units, current, hours, days, [], "Open-Meteo", ["Forecast and current conditions: Open-Meteo (open-meteo.com), licensed CC BY 4.0."]);
    }

    private static Section Find(IReadOnlyList<Section> sections, string heading) =>
        Assert.Single(sections, s => s.Heading == heading);

    [Fact]
    public void Sections_come_in_the_agreed_order()
    {
        var sections = ForecastWriter.Write(Sample(), Options());

        Assert.Equal(
            ["Alerts", "Right now", "Rest of today", "Saturday, September 12", "Sunday, September 13", "Sources"],
            sections.Select(s => s.Heading).ToArray());
    }

    [Fact]
    public void Right_now_reads_as_a_sentence_in_words()
    {
        var section = Find(ForecastWriter.Write(Sample(), Options()), "Right now");

        Assert.Equal(
            "As of 2:30 pm, it's 21 degrees and mostly clear, feeling like 20. Wind from the southwest at 12 kilometres an hour, gusting to 30.",
            section.Paragraphs[0]);
        Assert.Equal("Humidity 52 percent, dew point 11 degrees, pressure 1017 hectopascals, visibility 24 kilometres, cloud cover 40 percent.", section.Paragraphs[1]);
        Assert.Equal(2, section.Paragraphs.Count);
    }

    // Fresh text carries no age; from 30 minutes, or after a failed refresh,
    // the first line under Right now says how old it is and why.
    [Fact]
    public void Stale_or_unrefreshed_text_is_dated_on_its_first_line()
    {
        var stale = Sample() with { FetchedAt = Now.AddMinutes(-45) };
        var section = Find(ForecastWriter.Write(stale, Options()), "Right now");
        Assert.Equal("Showing the forecast from 45 minutes ago.", section.Paragraphs[0]);
        Assert.StartsWith("As of 2:30 pm, ", section.Paragraphs[1]);

        var failed = Find(ForecastWriter.Write(Sample(), Options() with { RefreshProblem = "couldn't reach the weather service" }), "Right now");
        Assert.Equal("Showing the forecast from 3 minutes ago; couldn't reach the weather service.", failed.Paragraphs[0]);

        Assert.Null(ForecastWriter.Freshness(TimeSpan.FromMinutes(29), null));
        Assert.Equal("Showing the forecast from 1 hour ago.", ForecastWriter.Freshness(TimeSpan.FromMinutes(75), null));
        Assert.Equal("Showing the forecast from less than a minute ago; couldn't reach the weather service.", ForecastWriter.Freshness(TimeSpan.FromSeconds(20), "couldn't reach the weather service"));
    }

    [Fact]
    public void Feels_like_is_omitted_when_it_rounds_to_the_temperature()
    {
        var f = Sample() with { Current = Sample().Current with { FeelsLike = 21.2 } };
        var section = Find(ForecastWriter.Write(f, Options()), "Right now");

        Assert.StartsWith("As of 2:30 pm, it's 21 degrees and mostly clear. ", section.Paragraphs[0]);
    }

    [Fact]
    public void Calm_air_and_minor_gusts_are_worded_simply()
    {
        var calm = Sample() with { Current = Sample().Current with { WindSpeed = 1, WindGusts = 3 } };
        Assert.Contains("The air is calm.", Find(ForecastWriter.Write(calm, Options()), "Right now").Paragraphs[0]);

        var steady = Sample() with { Current = Sample().Current with { WindSpeed = 12, WindGusts = 15 } };
        Assert.EndsWith("Wind from the southwest at 12 kilometres an hour.", Find(ForecastWriter.Write(steady, Options()), "Right now").Paragraphs[0]);
    }

    [Fact]
    public void Rest_of_today_covers_the_remaining_parts_of_the_day_then_the_sun_and_uv()
    {
        var section = Find(ForecastWriter.Write(Sample(), Options()), "Rest of today");

        Assert.Equal("Today's high 25, tonight's low 12.", section.Paragraphs[0]);
        Assert.Equal("This afternoon: partly cloudy, around 24 degrees, wind up to 20 kilometres an hour.", section.Paragraphs[1]);
        Assert.Equal("This evening: partly cloudy with light rain showers at times, cooling from 23 to 20 degrees, 60 percent chance of rain, wind up to 20 kilometres an hour.", section.Paragraphs[2]);
        Assert.Equal("Overnight: mostly clear, cooling from 19 to 12 degrees, 30 percent chance of rain, wind up to 22 kilometres an hour.", section.Paragraphs[3]);
        Assert.Equal("The sun rose at 6:52 am and sets at 7:34 pm, 12 hours and 42 minutes of daylight.", section.Paragraphs[4]);
        Assert.Equal("UV index 6, high.", section.Paragraphs[5]);
        Assert.Equal(6, section.Paragraphs.Count);
    }

    [Fact]
    public void Coming_days_split_into_day_and_night_from_the_hourly_data()
    {
        var saturday = Find(ForecastWriter.Write(Sample(), Options()), "Saturday, September 12");

        Assert.Equal(
            "Saturday: partly cloudy with light rain showers at times, high 23, 60 percent chance of rain in the afternoon, about 8 millimetres, wind from the southwest up to 25 kilometres an hour, gusting to 45.",
            saturday.Paragraphs[0]);
        Assert.Equal(
            "Saturday night: mostly clear, low 15, wind from the southwest up to 25 kilometres an hour, gusting to 45.",
            saturday.Paragraphs[1]);
        Assert.Equal(2, saturday.Paragraphs.Count);
    }

    [Fact]
    public void A_day_without_hourly_data_falls_back_to_one_paragraph()
    {
        var sunday = Find(ForecastWriter.Write(Sample(), Options()), "Sunday, September 13");

        Assert.Equal("Clear. High 22, low 10. Wind from the northwest up to 9 kilometres an hour.", Assert.Single(sunday.Paragraphs));
    }

    [Fact]
    public void Chances_round_to_tens_and_pick_rain_or_snow_by_temperature()
    {
        var f = Sample();
        var cold = f.Hours.Select(h => h.LocalTime.Date == new DateTime(2026, 9, 12) && h.LocalTime.Hour >= 19
            ? h with { Temperature = -4, PrecipitationProbability = 26, WeatherCode = 3 }
            : h).ToList();
        var saturday = Find(ForecastWriter.Write(f with { Hours = cold }, Options()), "Saturday, September 12");

        Assert.Equal("Saturday night: overcast, low minus 4, 30 percent chance of snow, wind from the southwest up to 25 kilometres an hour, gusting to 45.", saturday.Paragraphs[1]);

        var marginal = f.Hours.Select(h => h.LocalTime.Date == new DateTime(2026, 9, 12) && h.LocalTime.Hour >= 19
            ? h with { Temperature = 1, PrecipitationProbability = 14, WeatherCode = 3 }
            : h).ToList();
        var quiet = Find(ForecastWriter.Write(f with { Hours = marginal }, Options()), "Saturday, September 12");
        Assert.DoesNotContain("percent", quiet.Paragraphs[1]);

        var marginalWet = marginal.Select(h => h.LocalTime.Date == new DateTime(2026, 9, 12) && h.LocalTime.Hour >= 19 ? h with { PrecipitationProbability = 44 } : h).ToList();
        var mixed = Find(ForecastWriter.Write(f with { Hours = marginalWet }, Options()), "Saturday, September 12");
        Assert.Contains("40 percent chance of precipitation", mixed.Paragraphs[1]);
    }

    [Fact]
    public void Precipitation_timing_is_omitted_when_the_chance_spans_the_day()
    {
        var f = Sample();
        var allDay = f.Hours.Select(h => h.LocalTime.Date == new DateTime(2026, 9, 12) && h.LocalTime.Hour is >= 7 and <= 18
            ? h with { PrecipitationProbability = 50, WeatherCode = 61 }
            : h).ToList();
        var saturday = Find(ForecastWriter.Write(f with { Hours = allDay }, Options()), "Saturday, September 12");

        Assert.StartsWith("Saturday: light rain, high 23, 50 percent chance of rain, about 8 millimetres, ", saturday.Paragraphs[0]);
    }

    // After sunset the next sunrise is the news, and the UV index, the
    // day's highest, is nothing to act on.
    [Fact]
    public void After_sunset_the_next_sunrise_is_given_and_the_uv_index_left_out()
    {
        var evening = Options() with { Now = new DateTimeOffset(2026, 9, 11, 20, 15, 0, Eastern) };
        var section = Find(ForecastWriter.Write(Sample(), evening), "Rest of today");

        Assert.Equal("The sun set at 7:34 pm and rises at 6:53 am tomorrow.", section.Paragraphs[section.Paragraphs.Count - 1]);
        Assert.DoesNotContain(section.Paragraphs, p => p.StartsWith("UV index", StringComparison.Ordinal));

        var pacific = TimeZoneInfo.CreateCustomTimeZone("test-pacific", TimeSpan.FromHours(-7), "Test Pacific", "Test Pacific");
        var away = Find(ForecastWriter.Write(Sample(), Options(pacific) with { Now = evening.Now }), "Rest of today");
        Assert.Equal("The sun set at 7:34 pm (4:34 pm your time) and rises at 6:53 am tomorrow (3:53 am tomorrow your time).", away.Paragraphs[away.Paragraphs.Count - 1]);

        // Without the next day's sunrise, the day's own times as before.
        var lastDay = Sample() with { Days = [Sample().Days[0]] };
        var alone = Find(ForecastWriter.Write(lastDay, evening), "Rest of today");
        Assert.Equal("The sun rose at 6:52 am and set at 7:34 pm, 12 hours and 42 minutes of daylight.", alone.Paragraphs[alone.Paragraphs.Count - 1]);
    }

    [Fact]
    public void Sources_close_the_reading()
    {
        var sections = ForecastWriter.Write(Sample(), Options());

        Assert.Equal("Sources", sections[sections.Count - 1].Heading);
        Assert.Equal("Forecast and current conditions: Open-Meteo (open-meteo.com), licensed CC BY 4.0.", sections[sections.Count - 1].Paragraphs[0]);
    }

    [Fact]
    public void Times_carry_the_pc_time_in_brackets_when_zones_differ()
    {
        var pacific = TimeZoneInfo.CreateCustomTimeZone("test-pacific", TimeSpan.FromHours(-7), "Test Pacific", "Test Pacific");
        var sections = ForecastWriter.Write(Sample(), Options(pacific));

        Assert.StartsWith("As of 2:30 pm (11:30 am your time), ", Find(sections, "Right now").Paragraphs[0]);
        Assert.Contains("The sun rose at 6:52 am (3:52 am your time) and sets at 7:34 pm (4:34 pm your time), 12 hours and 42 minutes of daylight.", Find(sections, "Rest of today").Paragraphs);
    }

    [Fact]
    public void The_bracket_names_the_reader_s_own_day_when_it_is_not_all_today()
    {
        // A PC in Kathmandu's zone (UTC+5:45) reading Toronto at 2:45 pm
        // on the 11th is at 12:30 am on the 12th: Toronto's sunrise was the
        // reader's yesterday afternoon, and its afternoon and sunset fall in
        // the reader's today, which the bracket now says.
        var kathmandu = TimeZoneInfo.CreateCustomTimeZone("test-kathmandu", new TimeSpan(5, 45, 0), "Test Kathmandu", "Test Kathmandu");
        var sections = ForecastWriter.Write(Sample(), Options(kathmandu));

        Assert.StartsWith("As of 2:30 pm (12:15 am today your time), ", Find(sections, "Right now").Paragraphs[0]);
        Assert.Contains("The sun rose at 6:52 am (4:37 pm yesterday your time) and sets at 7:34 pm (5:19 am today your time), 12 hours and 42 minutes of daylight.", Find(sections, "Rest of today").Paragraphs);

        // A PC behind the location, as in Honolulu: Toronto's late evening is
        // the reader's afternoon, all on their today, so no day is named;
        // Toronto's early tomorrow is still the reader's today.
        var honolulu = TimeZoneInfo.CreateCustomTimeZone("test-honolulu", TimeSpan.FromHours(-10), "Test Honolulu", "Test Honolulu");
        var clock = new Clock(Eastern, honolulu, "h:mm tt", CultureInfo.InvariantCulture);
        var now = new DateTime(2026, 9, 11, 14, 45, 0);
        Assert.Equal("11:30 pm (5:30 pm your time)", clock.Time(new DateTime(2026, 9, 11, 23, 30, 0), now));
        Assert.Equal("1:00 am (7:00 pm today your time)", clock.Time(new DateTime(2026, 9, 12, 1, 0, 0), now));
    }

    [Fact]
    public void Precipitation_right_now_reads_with_rather_than_and()
    {
        var sections = ForecastWriter.Write(Sample(currentCode: 53), Options());
        Assert.StartsWith("As of 2:30 pm, it's 21 degrees with drizzle, feeling like 20.", Find(sections, "Right now").Paragraphs[0]);

        Assert.StartsWith("As of 2:30 pm, it's 21 degrees and foggy", Find(ForecastWriter.Write(Sample(currentCode: 45), Options()), "Right now").Paragraphs[0]);
    }

    [Fact]
    public void A_heavier_hour_of_the_same_weather_reads_heavier_at_times()
    {
        var f = Sample();
        // Saturday's daytime hours all light snow but one hour of snow.
        var hours = f.Hours.Select(h => h.LocalTime.Date == new DateTime(2026, 9, 12) && h.LocalTime.Hour is >= 7 and < 19
            ? h with { WeatherCode = h.LocalTime.Hour == 13 ? 73 : 71 }
            : h).ToList();

        var saturday = Find(ForecastWriter.Write(f with { Hours = hours }, Options()), "Saturday, September 12").Paragraphs[0];
        Assert.StartsWith("Saturday: light snow, heavier at times, high 23", saturday);
    }

    [Fact]
    public void Polar_day_and_night_are_said_outright()
    {
        var f = Sample();
        var day = f.Days[0] with { Sunrise = new DateTime(2026, 9, 11), Sunset = new DateTime(2026, 9, 12), DaylightSeconds = 86400 };
        var night = f.Days[0] with { Sunrise = new DateTime(2026, 9, 11), Sunset = new DateTime(2026, 9, 11), DaylightSeconds = 0 };

        var midsummer = Find(ForecastWriter.Write(f with { Days = [day, .. f.Days.Skip(1)] }, Options()), "Rest of today").Paragraphs;
        Assert.Equal(["The sun does not set today.", "UV index 6, high."], midsummer.Skip(midsummer.Count - 2).ToArray());
        var midwinter = Find(ForecastWriter.Write(f with { Days = [night, .. f.Days.Skip(1)] }, Options()), "Rest of today").Paragraphs;
        Assert.Equal("The sun does not rise today.", midwinter[midwinter.Count - 1]);
    }

    [Fact]
    public void Imperial_units_are_worded_in_miles_and_inches()
    {
        var f = Sample(UnitSystem.Imperial);
        var sections = ForecastWriter.Write(f, Options());

        Assert.Contains("Wind from the southwest at 12 miles an hour, gusting to 30.", Find(sections, "Right now").Paragraphs[0]);
        Assert.Contains("pressure 30.03 inches of mercury, visibility 15 miles", Find(sections, "Right now").Paragraphs[1]);
    }

    [Fact]
    public void Negative_temperatures_say_minus()
    {
        var f = Sample() with { Current = Sample().Current with { Temperature = -4.6, FeelsLike = -11.2 } };

        Assert.StartsWith("As of 2:30 pm, it's minus 5 degrees and mostly clear, feeling like minus 11.", Find(ForecastWriter.Write(f, Options()), "Right now").Paragraphs[0]);
    }

    [Fact]
    public void Writes_a_real_response_without_error()
    {
        var f = OpenMeteoClient.Parse(Fixtures.Read("open-meteo-toronto.json"), Toronto, UnitSystem.Metric, Now.AddMinutes(-1));
        var sections = ForecastWriter.Write(f, Options());

        Assert.Equal("Alerts", sections[0].Heading);
        Assert.Equal("Right now", sections[1].Heading);
        Assert.Equal("Sources", sections[sections.Count - 1].Heading);
        Assert.All(sections, s => Assert.NotEmpty(s.Paragraphs));
        Assert.Contains(sections, s => s.Heading == "Saturday, September 12");
    }
}
