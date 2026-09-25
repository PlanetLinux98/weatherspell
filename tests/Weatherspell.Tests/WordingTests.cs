using Weatherspell.Weather;
using Xunit;

namespace Weatherspell.Tests;

public class WordingTests
{
    [Theory]
    [InlineData(0, "north")]
    [InlineData(11, "north")]
    [InlineData(12, "north-northeast")]
    [InlineData(45, "northeast")]
    [InlineData(142, "southeast")]
    [InlineData(225, "southwest")]
    [InlineData(359, "north")]
    [InlineData(-90, "west")]
    public void Compass_points_from_degrees(double degrees, string expected)
    {
        Assert.Equal(expected, Compass.FromDegrees(degrees));
    }

    [Theory]
    [InlineData(0, "clear")]
    [InlineData(2, "partly cloudy")]
    [InlineData(45, "foggy")]
    [InlineData(63, "rain")]
    [InlineData(75, "heavy snow")]
    [InlineData(80, "light rain showers")]
    [InlineData(95, "thunderstorms")]
    [InlineData(99, "thunderstorms with heavy hail")]
    [InlineData(42, "unknown conditions")]
    public void Weather_codes_become_phrases(int code, string expected)
    {
        Assert.Equal(expected, WeatherCodes.Describe(code));
    }

    [Fact]
    public void Precipitation_word_follows_the_code_family()
    {
        Assert.Equal("snow", WeatherCodes.PrecipitationWord(73));
        Assert.Equal("snow", WeatherCodes.PrecipitationWord(85));
        Assert.Equal("rain", WeatherCodes.PrecipitationWord(61));
        Assert.Equal("precipitation", WeatherCodes.PrecipitationWord(2));
    }

    [Theory]
    [InlineData(21.4, "21 degrees")]
    [InlineData(21.5, "22 degrees")]
    [InlineData(-0.4, "0 degrees")]
    [InlineData(-4.6, "minus 5 degrees")]
    [InlineData(1.2, "1 degree")]
    [InlineData(-0.8, "minus 1 degree")]
    public void Degrees_are_whole_and_say_minus(double value, string expected)
    {
        Assert.Equal(expected, Units.Degrees(value));
    }

    [Fact]
    public void Quantities_are_worded_per_unit_system()
    {
        Assert.Equal("12 kilometres an hour", Units.Speed(12.3, UnitSystem.Metric));
        Assert.Equal("8 miles an hour", Units.Speed(7.6, UnitSystem.Imperial));
        Assert.Equal("24 kilometres", Units.Distance(24100, UnitSystem.Metric));
        Assert.Equal("800 metres", Units.Distance(800, UnitSystem.Metric));
        Assert.Equal("under a mile", Units.Distance(800, UnitSystem.Imperial));
        Assert.Equal("1 kilometre", Units.Distance(1200, UnitSystem.Metric));
        Assert.Equal("1 mile", Units.Distance(1700, UnitSystem.Imperial));
        Assert.Equal("1017 hectopascals", Units.Pressure(1016.8, UnitSystem.Metric));
        Assert.Equal("30.03 inches of mercury", Units.Pressure(1016.8, UnitSystem.Imperial));
        Assert.Equal("less than a millimetre", Units.PrecipitationAmount(0.4, UnitSystem.Metric));
        Assert.Equal("1 millimetre", Units.PrecipitationAmount(1.2, UnitSystem.Metric));
        Assert.Equal("8 millimetres", Units.PrecipitationAmount(8.4, UnitSystem.Metric));
        Assert.Equal("0.3 inches", Units.PrecipitationAmount(0.33, UnitSystem.Imperial));
        Assert.Equal("1 kilometre an hour", Units.Speed(1.2, UnitSystem.Metric));
        Assert.Equal("1 centimetre", Units.Centimetres(1.4));
        Assert.Equal("4 centimetres", Units.Centimetres(3.6));
    }

    [Fact]
    public void A_spoken_pc_time_names_its_day_unless_it_is_today()
    {
        var now = new DateTime(2026, 9, 25, 18, 14, 0);
        var at = new DateTime(2026, 9, 25, 10, 43, 0);

        Assert.Equal(Clock.PcTime(at), Clock.PcTimeOnDay(at, now));
        Assert.Equal(Clock.PcTime(at) + " yesterday", Clock.PcTimeOnDay(at.AddDays(-1), now));
        Assert.Equal(Clock.PcTime(at) + " on " + at.AddDays(-10).ToString("MMMM d", System.Globalization.CultureInfo.CurrentCulture), Clock.PcTimeOnDay(at.AddDays(-10), now));
    }

    [Theory]
    [InlineData("Gander Int'l Airport", "Gander International Airport")]
    [InlineData("VISITORS CENTER AT FURNACE CREEK DEATH VALLEY", "Visitors Center at Furnace Creek Death Valley")]
    [InlineData("Spokane, Spokane International Airport", "Spokane, Spokane International Airport")]
    [InlineData("Burlington Lift Bridge", "Burlington Lift Bridge")]
    public void Station_names_read_as_words(string name, string expected)
    {
        Assert.Equal(expected, OfficialText.StationName(name));
    }

    [Fact]
    public void Heavier_hours_of_the_same_weather_are_not_named_twice()
    {
        Assert.Equal(WeatherCodes.Kind(71), WeatherCodes.Kind(73));
        Assert.Equal(WeatherCodes.Kind(80), WeatherCodes.Kind(81));
        Assert.NotEqual(WeatherCodes.Kind(51), WeatherCodes.Kind(80));
        Assert.NotEqual(WeatherCodes.Kind(61), WeatherCodes.Kind(66));
        Assert.True(WeatherCodes.IsNoun(53));
        Assert.True(WeatherCodes.IsNoun(48));
        Assert.False(WeatherCodes.IsNoun(45));
        Assert.False(WeatherCodes.IsNoun(3));
    }

    // One system per location's whole text: Environment Canada writes only
    // metric, the NWS writes either, so only Canada overrides the region.
    [Theory]
    [InlineData("Canada", false, true)]
    [InlineData("Canada", true, true)]
    [InlineData("United States", false, false)]
    [InlineData("United States", true, true)]
    [InlineData("France", false, false)]
    [InlineData(null, true, true)]
    public void Units_follow_the_location_s_service_or_else_the_region(string? country, bool metricRegion, bool metric)
    {
        var location = new Location("Somewhere", null, country, 45, -75, null);
        var region = metricRegion ? UnitSystem.Metric : UnitSystem.Imperial;
        Assert.Equal(metric ? UnitSystem.Metric : UnitSystem.Imperial, Units.For(location, region));
    }

    [Fact]
    public void Ages_and_durations_read_naturally()
    {
        Assert.Equal("just now", Clock.Age(TimeSpan.FromSeconds(20)));
        Assert.Equal("1 minute ago", Clock.Age(TimeSpan.FromSeconds(90)));
        Assert.Equal("5 minutes ago", Clock.Age(TimeSpan.FromMinutes(5.4)));
        Assert.Equal("2 hours ago", Clock.Age(TimeSpan.FromHours(2.9)));
        Assert.Equal("1 day ago", Clock.Age(TimeSpan.FromHours(30)));
        Assert.Equal("less than a minute ago", Clock.AgeAfterFrom(TimeSpan.FromSeconds(20)));
        Assert.Equal("2 hours ago", Clock.AgeAfterFrom(TimeSpan.FromHours(2.9)));
        Assert.Equal("12 hours and 42 minutes", Clock.Duration(45720));
        Assert.Equal("9 hours", Clock.Duration(32400));
        Assert.Equal("1 hour and 1 minute", Clock.Duration(3660));
        Assert.Equal("45 minutes", Clock.Duration(2700));
    }
}
