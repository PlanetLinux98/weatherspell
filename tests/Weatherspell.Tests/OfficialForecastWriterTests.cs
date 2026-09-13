using System.Globalization;
using Weatherspell.Weather;
using Weatherspell.Weather.EnvironmentCanada;
using Weatherspell.Weather.OpenMeteo;
using Xunit;

namespace Weatherspell.Tests;

// The official text laid over a base forecast: composition, then the
// reading structure it produces.
public class OfficialForecastWriterTests
{
    private static readonly Location Peterborough = new("Peterborough", "Ontario", "Canada", 44.30, -78.33, "America/Toronto");
    private static readonly TimeSpan Eastern = TimeSpan.FromHours(-4);

    // Friday 11 September 2026, 11:59 pm local, just after the fixture's
    // 11:57 pm observation.
    private static readonly DateTimeOffset Now = new(2026, 9, 11, 23, 59, 0, Eastern);

    private static WriterOptions Options(DateTimeOffset? now = null) =>
        new(now ?? Now, TimeZoneInfo.CreateCustomTimeZone("test-eastern", Eastern, "Test Eastern", "Test Eastern"), "h:mm tt", CultureInfo.InvariantCulture);

    internal static Forecast Base(UnitSystem units = UnitSystem.Metric)
    {
        var current = new CurrentConditions(
            LocalTime: new DateTime(2026, 9, 11, 23, 45, 0),
            Temperature: 12.4, FeelsLike: 11.0, Humidity: 88, WeatherCode: 45, IsDay: false,
            WindSpeed: 6, WindDirection: 200, WindGusts: 6, Precipitation: 0, CloudCover: 20, PressureHpa: 1017.2);
        var hours = new List<HourPoint>();
        for (var h = 0; h < 48; h++)
        {
            hours.Add(new HourPoint(new DateTime(2026, 9, 11, 0, 0, 0).AddHours(h), 12, 0, 0, 1, 5, 200, 8, 20000, 10.5, 85, h % 24 is > 6 and < 20));
        }
        var days = new List<DayForecast>
        {
            new(new DateTime(2026, 9, 11), 1, 21, 7, 21, 7, new DateTime(2026, 9, 11, 6, 47, 0), new DateTime(2026, 9, 11, 19, 32, 0), 45900, 5.0, 0, 0, 0, 12, 20, 200),
            new(new DateTime(2026, 9, 12), 1, 26, 18, 31, 18, new DateTime(2026, 9, 12, 6, 48, 0), new DateTime(2026, 9, 12, 19, 30, 0), 45720, 6.2, 0, 40, 0, 20, 35, 180),
        };
        return new Forecast(Peterborough, Now.AddMinutes(-1), Eastern, units, current, hours, days, [], "Open-Meteo", ["Forecast and current conditions: Open-Meteo (open-meteo.com), licensed CC BY 4.0."]);
    }

    private static OfficialForecast Official() => CityPageClient.Parse(Fixtures.Read("ec-citypage-peterborough.xml"));

    private static Section Find(IReadOnlyList<Section> sections, string heading) =>
        Assert.Single(sections, s => s.Heading == heading);

    [Fact]
    public void Composition_lays_the_observation_and_periods_over_the_base()
    {
        var f = ForecastService.Compose(Base(), Official());

        Assert.Equal("Environment Canada and Open-Meteo", f.SourceName);
        Assert.Equal(12, f.Periods.Count);
        var c = f.Current;
        Assert.Equal("Peterborough Municipal Airport", c.Station);
        Assert.Equal("Mist", c.Description);
        Assert.Equal(new DateTime(2026, 9, 11, 23, 57, 0), c.LocalTime);
        Assert.Equal(9.2, c.Temperature);
        Assert.Equal(9.2, c.FeelsLike); // no humidex or wind chill reported: never the model's apparent temperature
        Assert.Equal(100, c.Humidity);
        Assert.Equal(4, c.WindSpeed);
        Assert.Equal(4, c.WindGusts);
        Assert.Equal(260, c.WindDirection);
        Assert.Equal(1018, c.PressureHpa, 3);
        Assert.Equal(9.2, c.DewPoint);
        Assert.Equal(6400, c.VisibilityMetres);
        Assert.Equal(20, c.CloudCover); // the base's; stations do not report it
        Assert.Equal(
            ["Forecast text and current conditions: Environment Canada (weather.gc.ca), forecast for Peterborough City - Lakefield - Southern Peterborough County, observed at Peterborough Municipal Airport.",
             "Hourly data, sun and UV: Open-Meteo (open-meteo.com), licensed CC BY 4.0."],
            f.Sources);
    }

    [Fact]
    public void Observed_values_are_converted_to_imperial_units()
    {
        var c = ForecastService.Compose(Base(UnitSystem.Imperial), Official()).Current;

        Assert.Equal(48.56, c.Temperature, 2);
        Assert.Equal(2.49, c.WindSpeed, 2);
        Assert.Equal(48.56, c.DewPoint!.Value, 2);
    }

    [Fact]
    public void Without_an_observation_the_base_conditions_stay_and_the_sources_say_so()
    {
        var f = ForecastService.Compose(Base(), Official() with { Observation = null });

        Assert.Null(f.Current.Station);
        Assert.Equal(12.4, f.Current.Temperature);
        Assert.Equal(
            ["Forecast text: Environment Canada (weather.gc.ca), forecast for Peterborough City - Lakefield - Southern Peterborough County.",
             "Current conditions, hourly data, sun and UV: Open-Meteo (open-meteo.com), licensed CC BY 4.0."],
            f.Sources);
    }

    [Fact]
    public void The_reading_keeps_its_order_with_official_periods_under_the_day_headings()
    {
        var sections = ForecastWriter.Write(ForecastService.Compose(Base(), Official()), Options());

        Assert.Equal(
            ["Alerts", "Right now", "Rest of today", "Saturday, September 12", "Sunday, September 13", "Monday, September 14", "Tuesday, September 15", "Wednesday, September 16", "Thursday, September 17", "Sun and UV", "Details", "Sources"],
            sections.Select(s => s.Heading).ToArray());
        Assert.Equal(["Tonight: Clear. Fog patches developing near midnight. Low 7."], Find(sections, "Rest of today").Paragraphs);
        var saturday = Find(sections, "Saturday, September 12");
        Assert.Equal("Saturday: Sunny. Fog patches dissipating early in the morning. Wind becoming south 20 kilometres per hour in the afternoon. High 26. Humidex 31. UV index 6 or high.", saturday.Paragraphs[0]);
        Assert.Equal("Saturday night: Increasing cloudiness early in the evening. 40 percent chance of showers late in the evening and overnight with risk of thunderstorms. Low 18.", saturday.Paragraphs[1]);
        Assert.Equal(["Thursday: Cloudy. High 21."], Find(sections, "Thursday, September 17").Paragraphs);
    }

    [Fact]
    public void Right_now_names_the_station_and_uses_its_words()
    {
        var section = Find(ForecastWriter.Write(ForecastService.Compose(Base(), Official()), Options()), "Right now");

        Assert.Equal("As of 11:57 pm, Peterborough Municipal Airport reports mist and 9 degrees. Wind from the west at 4 kilometres an hour. Humidity 100 percent.", section.Paragraphs[0]);
    }

    [Fact]
    public void Details_prefer_observed_dew_point_and_visibility()
    {
        var section = Find(ForecastWriter.Write(ForecastService.Compose(Base(), Official()), Options()), "Details");

        Assert.Equal("Humidity 100 percent, dew point 9 degrees, pressure 1018 hectopascals, visibility 6 kilometres, cloud cover 20 percent.", section.Paragraphs[0]);
    }

    [Fact]
    public void Before_dawn_last_nights_period_is_still_todays()
    {
        // 1:30 am Saturday: "Tonight" (Friday's) is in progress and leads
        // the day, with Saturday's own periods after it.
        var sections = ForecastWriter.Write(ForecastService.Compose(Base(), Official()), Options(new DateTimeOffset(2026, 9, 12, 1, 30, 0, Eastern)));

        var today = Find(sections, "Rest of today");
        Assert.Equal(3, today.Paragraphs.Count);
        Assert.StartsWith("Tonight: ", today.Paragraphs[0]);
        Assert.StartsWith("Saturday: ", today.Paragraphs[1]);
        Assert.StartsWith("Saturday night: ", today.Paragraphs[2]);
        Assert.Equal("Sunday, September 13", sections[3].Heading);
    }

    [Fact]
    public void A_failed_official_fetch_is_explained_ahead_of_the_sources()
    {
        var f = Base() with { Sources = ["Environment Canada's forecast text could not be fetched this time (404 Not Found from dd.weather.gc.ca), so these sentences are written from Open-Meteo data.", .. Base().Sources] };
        var section = Find(ForecastWriter.Write(f, Options()), "Sources");

        Assert.Equal(2, section.Paragraphs.Count);
        Assert.StartsWith("Environment Canada's forecast text could not be fetched", section.Paragraphs[0]);
        Assert.StartsWith("Forecast and current conditions: Open-Meteo", section.Paragraphs[1]);
    }

    [Fact]
    public void The_nws_words_read_the_same_way()
    {
        var periods = Weatherspell.Weather.Nws.NwsClient.ParsePeriods(Fixtures.Read("nws-forecast-albany.json"));
        var official = new OfficialForecast("National Weather Service", "National Weather Service (weather.gov), forecast for Albany, NY", periods, null);
        var sections = ForecastWriter.Write(ForecastService.Compose(Base(), official), Options(new DateTimeOffset(2026, 9, 12, 2, 30, 0, Eastern)));

        Assert.Equal(["Overnight: Patchy fog after 3am. Mostly clear, with a low around 48. Wind around 0 miles per hour.",
                      "Saturday: Patchy fog before 9am. Mostly sunny, with a high near 77. South wind 0 to 12 miles per hour.",
                      "Saturday night: A slight chance of rain showers between 8pm and 11pm, then showers and thunderstorms. Mostly cloudy, with a low around 60. South wind 7 to 10 miles per hour, with gusts as high as 21 miles per hour. Chance of precipitation is 90 percent. New rainfall amounts between a half and three quarters of an inch possible."],
            Find(sections, "Rest of today").Paragraphs);
        Assert.Equal("Friday, September 18", sections[sections.Count - 4].Heading);
    }

    [Fact]
    public void Open_meteo_alone_still_reads_as_before()
    {
        var sections = ForecastWriter.Write(Base(), Options());

        Assert.Equal(["Alerts", "Right now", "Rest of today", "Saturday, September 12", "Sun and UV", "Details", "Sources"], sections.Select(s => s.Heading).ToArray());
        Assert.StartsWith("As of 11:45 pm, it's 12 degrees and foggy", Find(sections, "Right now").Paragraphs[0]);
    }
}
