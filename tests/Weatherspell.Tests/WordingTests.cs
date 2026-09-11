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
        Assert.Equal("1017 hectopascals", Units.Pressure(1016.8, UnitSystem.Metric));
        Assert.Equal("30.03 inches of mercury", Units.Pressure(1016.8, UnitSystem.Imperial));
        Assert.Equal("less than a millimetre", Units.PrecipitationAmount(0.4, UnitSystem.Metric));
        Assert.Equal("1 millimetre", Units.PrecipitationAmount(1.2, UnitSystem.Metric));
        Assert.Equal("8 millimetres", Units.PrecipitationAmount(8.4, UnitSystem.Metric));
        Assert.Equal("0.3 inches", Units.PrecipitationAmount(0.33, UnitSystem.Imperial));
    }

    [Fact]
    public void Ages_and_durations_read_naturally()
    {
        Assert.Equal("just now", Clock.Age(TimeSpan.FromSeconds(20)));
        Assert.Equal("1 minute ago", Clock.Age(TimeSpan.FromSeconds(90)));
        Assert.Equal("5 minutes ago", Clock.Age(TimeSpan.FromMinutes(5.4)));
        Assert.Equal("2 hours ago", Clock.Age(TimeSpan.FromHours(2.9)));
        Assert.Equal("1 day ago", Clock.Age(TimeSpan.FromHours(30)));
        Assert.Equal("12 hours and 42 minutes", Clock.Duration(45720));
        Assert.Equal("9 hours", Clock.Duration(32400));
        Assert.Equal("1 hour and 1 minute", Clock.Duration(3660));
        Assert.Equal("45 minutes", Clock.Duration(2700));
    }
}
