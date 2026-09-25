using Weatherspell.Weather;
using Weatherspell.Weather.OpenMeteo;
using Xunit;

namespace Weatherspell.Tests;

public class OpenMeteoParsingTests
{
    private static readonly Location Toronto = new("Toronto", "Ontario", "Canada", 43.65, -79.38, "America/Toronto");

    [Fact]
    public void Parses_a_real_forecast_response()
    {
        var fetched = new DateTimeOffset(2026, 9, 11, 20, 31, 0, TimeSpan.Zero);
        var f = OpenMeteoClient.Parse(Fixtures.Read("open-meteo-toronto.json"), Toronto, UnitSystem.Metric, fetched);

        Assert.Equal(TimeSpan.FromHours(-4), f.UtcOffset);
        Assert.Equal(new DateTime(2026, 9, 11, 16, 30, 0), f.Current.LocalTime);
        Assert.Equal(21.1, f.Current.Temperature, 3);
        Assert.Equal(0, f.Current.WeatherCode);
        Assert.True(f.Current.IsDay);
        Assert.InRange(f.Current.Humidity, 0, 100);
        Assert.InRange(f.Current.PressureHpa, 900, 1100);

        Assert.Equal(168, f.Hours.Count);
        Assert.Equal(new DateTime(2026, 9, 11, 0, 0, 0), f.Hours[0].LocalTime);
        Assert.NotNull(f.Hours[12].DewPoint);
        Assert.NotNull(f.Hours[12].VisibilityMetres);

        Assert.Equal(7, f.Days.Count);
        Assert.Equal(new DateTime(2026, 9, 11), f.Days[0].Date);
        Assert.Equal(new DateTime(2026, 9, 11, 6, 52, 0), f.Days[0].Sunrise);
        Assert.NotNull(f.Days[0].Sunset);
        Assert.NotNull(f.Days[0].DaylightSeconds);
        Assert.Equal(5.75, f.Days[0].UvIndexMax!.Value, 3);
        Assert.Equal(fetched, f.FetchedAt);
        Assert.Equal(OpenMeteoClient.SourceName, f.SourceName);
    }

    [Fact]
    public void Skips_hours_the_model_has_not_filled()
    {
        // A trailing null temperature marks the end of the model's horizon.
        var json = """
            {"utc_offset_seconds":0,
             "current":{"time":"2026-01-01T12:00","temperature_2m":1.0,"weather_code":3},
             "hourly":{"time":["2026-01-01T12:00","2026-01-01T13:00"],"temperature_2m":[1.0,null],"weather_code":[3,null]},
             "daily":{"time":["2026-01-01"],"temperature_2m_max":[2.0],"temperature_2m_min":[0.0]}}
            """;
        var f = OpenMeteoClient.Parse(json, Toronto, UnitSystem.Metric, DateTimeOffset.UtcNow);

        Assert.Single(f.Hours);
        Assert.Single(f.Days);
        Assert.Null(f.Days[0].Sunrise);
        Assert.Equal(1.0, f.Current.FeelsLike); // falls back to the temperature
    }

    [Fact]
    public void Builds_the_forecast_url_with_units_and_local_time()
    {
        var url = OpenMeteoClient.ForecastUrl(Toronto, UnitSystem.Imperial, 7);

        Assert.StartsWith("https://api.open-meteo.com/v1/forecast?latitude=43.65&longitude=-79.38&", url);
        Assert.Contains("&timezone=auto", url);
        Assert.Contains("&temperature_unit=fahrenheit", url);
        Assert.Contains("&wind_speed_unit=mph", url);
        Assert.Contains("&precipitation_unit=inch", url);
        Assert.Contains("&forecast_days=7", url);
    }

    [Fact]
    public void Parses_geocoding_results_with_region_country_and_population()
    {
        var results = OpenMeteoClient.ParseSearch(Fixtures.Read("geocoding-peterborough.json"));

        Assert.True(results.Count >= 3);
        // The geocoder knows a city and a township of that name in Ontario.
        var ontario = results.First(r => r.Name == "Peterborough" && r.Region == "Ontario");
        Assert.Equal("Peterborough", ontario.Name);
        Assert.Equal("Canada", ontario.Country);
        Assert.Equal("America/Toronto", ontario.TimeZoneId);
        Assert.Equal("Peterborough, Ontario, Canada", ontario.FullName);
        Assert.StartsWith("Peterborough, Ontario, Canada (population ", ontario.SearchResultText);
    }

    [Fact]
    public void A_territory_the_geocoder_gives_no_country_is_named_and_covered_by_the_nws()
    {
        // As the geocoder answered on 2026-09-25: a country code and no country.
        const string json = """
            {"results":[
              {"id":4568127,"name":"San Juan","latitude":18.46633,"longitude":-66.10572,"country_code":"PR","timezone":"America/Puerto_Rico","population":418140,"admin1":"San Juan"},
              {"id":5881576,"name":"Pago Pago","latitude":-14.27806,"longitude":-170.7025,"country_code":"AS","timezone":"Pacific/Pago_Pago","admin1":"Eastern District"},
              {"id":2729907,"name":"Longyearbyen","latitude":78.22334,"longitude":15.64689,"country_code":"SJ","timezone":"Arctic/Longyearbyen"}
            ]}
            """;

        var results = OpenMeteoClient.ParseSearch(json);

        Assert.Equal("San Juan, Puerto Rico", results[0].FullName);
        Assert.True(results[0].IsNwsCovered);
        Assert.Equal("Pago Pago, Eastern District, American Samoa", results[1].FullName);
        Assert.True(results[1].IsNwsCovered);
        // Elsewhere a missing country stays missing.
        Assert.Null(results[2].Country);
        Assert.False(results[2].IsNwsCovered);
    }

    [Fact]
    public void Empty_geocoding_response_yields_no_results()
    {
        Assert.Empty(OpenMeteoClient.ParseSearch("{\"generationtime_ms\":0.5}"));
    }
}
