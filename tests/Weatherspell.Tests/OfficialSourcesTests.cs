using Weatherspell.Weather;
using Weatherspell.Weather.EnvironmentCanada;
using Weatherspell.Weather.Nws;
using Xunit;

namespace Weatherspell.Tests;

public class NwsParsingTests
{
    [Fact]
    public void Points_give_the_grid_forecast_the_station_list_and_a_named_area()
    {
        var (forecast, stations, attribution) = NwsClient.ParsePoints(Fixtures.Read("nws-points-albany.json"));

        Assert.Equal("https://api.weather.gov/gridpoints/ALY/72,63/forecast", forecast);
        Assert.Equal("https://api.weather.gov/gridpoints/ALY/72,63/stations", stations);
        Assert.Equal("National Weather Service (weather.gov), forecast for Albany, NY", attribution);
    }

    [Fact]
    public void The_nearest_station_is_the_first_listed()
    {
        Assert.Equal(("KALB", "Albany International Airport"), NwsClient.ParseStations(Fixtures.Read("nws-stations-albany.json")));
    }

    [Fact]
    public void Periods_keep_the_official_words_dated_by_their_local_start()
    {
        var periods = NwsClient.ParsePeriods(Fixtures.Read("nws-forecast-albany.json"));

        Assert.Equal(14, periods.Count);
        Assert.Equal(new OfficialPeriod("Overnight", new DateTime(2026, 9, 12), "Patchy fog after 3am. Mostly clear, with a low around 48. Wind around 0 miles per hour."), periods[0]);
        Assert.Equal("Saturday", periods[1].Name);
        // "Saturday Night" as the NWS cases it becomes "Saturday night"; the
        // night is dated by the day it follows; symbols become words.
        Assert.Equal("Saturday night", periods[2].Name);
        Assert.Equal(new DateTime(2026, 9, 12), periods[2].Date);
        Assert.Contains("gusts as high as 21 miles per hour. Chance of precipitation is 90 percent.", periods[2].Text);
        Assert.Equal(new DateTime(2026, 9, 18), periods[13].Date);
    }

    [Fact]
    public void The_grid_forecast_is_asked_for_SI_text_for_a_metric_reader()
    {
        const string url = "https://api.weather.gov/gridpoints/OTX/144,86/forecast";
        Assert.Equal(url + "?units=si", NwsClient.ForecastUrl(url, UnitSystem.Metric));
        Assert.Equal(url, NwsClient.ForecastUrl(url, UnitSystem.Imperial));
    }

    [Fact]
    public void An_observation_reads_SI_values_and_leaves_unreported_ones_null()
    {
        var o = NwsClient.ParseObservation(Fixtures.Read("nws-observation-kalb.json"), "Albany International Airport");

        Assert.Equal("Albany International Airport", o.Station);
        Assert.Equal(new DateTimeOffset(2026, 9, 12, 7, 5, 0, TimeSpan.Zero), o.Time);
        Assert.Equal("Patchy Fog", o.Description);
        Assert.Equal(11, o.TemperatureC);
        Assert.Null(o.FeelsLikeC);
        Assert.Null(o.Humidity);
        Assert.Equal(0, o.WindKmh);
        Assert.Equal(0, o.WindDirection);
        Assert.Null(o.WindGustKmh);
        Assert.Null(o.PressureHpa);
        Assert.Null(o.DewPointC);
        Assert.Equal(16093.44, o.VisibilityMetres!.Value, 2);
    }
}

public class EnvironmentCanadaTests
{
    [Fact]
    public void The_site_list_parses_and_finds_the_nearest_site()
    {
        var sites = SiteList.Parse(Fixtures.Read("ec-site-list.csv"));

        Assert.InRange(sites.Count, 800, 1000);
        var (site, distance) = SiteList.Nearest(sites, 44.3104, -78.2396);
        Assert.Equal(new Site("s0000629", "Peterborough", "ON", 44.30, -78.33), site);
        Assert.InRange(distance, 5, 10);
    }

    [Fact]
    public void The_newest_file_for_a_site_is_picked_from_an_hour_listing()
    {
        var listing = Fixtures.Read("ec-hour-listing-on-03.html");

        Assert.Equal("20260912T035744.540Z_MSC_CitypageWeather_s0000629_en.xml", CityPageClient.LatestFile(listing, "s0000629"));
        Assert.Null(CityPageClient.LatestFile(listing, "s9999999"));
    }

    [Fact]
    public void Periods_are_dated_from_the_issue_day_by_their_names()
    {
        var official = CityPageClient.Parse(Fixtures.Read("ec-citypage-peterborough.xml"));

        Assert.Equal("Environment Canada", official.SourceName);
        Assert.Equal("Environment Canada (weather.gc.ca), forecast for Peterborough City - Lakefield - Southern Peterborough County", official.Attribution);
        // Issued Friday the 11th at 3:30 pm: Tonight is the 11th, the named
        // days follow, nights belong to their day.
        Assert.Equal(12, official.Periods.Count);
        Assert.Equal(new OfficialPeriod("Tonight", new DateTime(2026, 9, 11), "Clear. Fog patches developing near midnight. Low 7."), official.Periods[0]);
        Assert.Equal(new OfficialPeriod("Saturday", new DateTime(2026, 9, 12), "Sunny. Fog patches dissipating early in the morning. Wind becoming south 20 kilometres per hour in the afternoon. High 26. Humidex 31. UV index 6 or high."), official.Periods[1]);
        Assert.Equal(("Saturday night", new DateTime(2026, 9, 12)), (official.Periods[2].Name, official.Periods[2].Date));
        Assert.Equal(("Thursday", new DateTime(2026, 9, 17)), (official.Periods[11].Name, official.Periods[11].Date));
    }

    [Fact]
    public void Current_conditions_come_from_the_named_station_in_SI()
    {
        var o = CityPageClient.Parse(Fixtures.Read("ec-citypage-peterborough.xml")).Observation!;

        Assert.Equal("Peterborough Municipal Airport", o.Station);
        Assert.Equal(new DateTimeOffset(2026, 9, 12, 3, 57, 0, TimeSpan.Zero), o.Time);
        Assert.Equal("Mist", o.Description);
        Assert.Equal(9.2, o.TemperatureC);
        Assert.Null(o.FeelsLikeC);
        Assert.Equal(100, o.Humidity);
        Assert.Equal(4, o.WindKmh);
        Assert.Equal(260, o.WindDirection);
        Assert.Null(o.WindGustKmh);
        Assert.Equal(1018, o.PressureHpa!.Value, 3);
        Assert.Equal(9.2, o.DewPointC);
        Assert.Equal(6400, o.VisibilityMetres!.Value, 3);
    }

    [Fact]
    public void A_page_without_current_conditions_has_no_observation()
    {
        var xml = Fixtures.Read("ec-citypage-peterborough.xml");
        var start = xml.IndexOf("<currentConditions>", StringComparison.Ordinal);
        var end = xml.IndexOf("</currentConditions>", StringComparison.Ordinal) + "</currentConditions>".Length;
        var stripped = xml.Substring(0, start) + "<currentConditions/>" + xml.Substring(end);

        Assert.Null(CityPageClient.Parse(stripped).Observation);
    }
}

public class OfficialTextTests
{
    [Theory]
    [InlineData("Chance of precipitation is 90%.", "Chance of precipitation is 90 percent.")]
    [InlineData("Wind becoming south 20 km/h in the afternoon.", "Wind becoming south 20 kilometres per hour in the afternoon.")]
    [InlineData("South wind 7 to 10 mph, with gusts as high as 21 mph.", "South wind 7 to 10 miles per hour, with gusts as high as 21 miles per hour.")]
    [InlineData("  Sunny. High 26.  ", "Sunny. High 26.")]
    public void Symbols_become_the_words_the_service_says_aloud(string written, string spoken)
    {
        Assert.Equal(spoken, OfficialText.Spoken(written));
    }
}
