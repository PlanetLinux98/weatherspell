using System.Globalization;
using Weatherspell.Cache;
using Weatherspell.Weather;
using Weatherspell.Weather.Alerts;
using Weatherspell.Weather.EnvironmentCanada;
using Xunit;

namespace Weatherspell.Tests;

public class ForecastCacheTests : IDisposable
{
    private readonly string _dir = Path.Combine(Path.GetTempPath(), "weatherspell-tests-" + Guid.NewGuid().ToString("N"));
    private static readonly TimeSpan Eastern = TimeSpan.FromHours(-4);
    private static readonly Location Peterborough = new("Peterborough", "Ontario", "Canada", 44.30, -78.33, "America/Toronto");

    private ForecastCache Cache() => new(Path.Combine(_dir, "cache"));

    public void Dispose()
    {
        if (Directory.Exists(_dir)) Directory.Delete(_dir, recursive: true);
    }

    // An official forecast laid over the base: periods, a station with its
    // own words, dew point and visibility, all of which the file must keep.
    private static Forecast Sample() =>
        ForecastService.Compose(OfficialForecastWriterTests.Base(), CityPageClient.Parse(Fixtures.Read("ec-citypage-peterborough.xml")));

    private static WeatherAlert Alert(string id, string @event, DateTimeOffset? ends) =>
        new(id, @event, AlertSeverity.Severe, new DateTimeOffset(2026, 9, 11, 13, 10, 0, Eastern), new DateTimeOffset(2026, 9, 11, 14, 0, 0, Eastern), ends,
            "Environment Canada", "Environment Canada", "Peterborough City - Lakefield - Southern Peterborough County", "warning", "Text.\r\n\r\nMore \"text\".", "Take care.", "https://weather.gc.ca/");

    private static AlertReport Report(params WeatherAlert[] alerts) =>
        new(alerts, "Environment Canada (weather.gc.ca)", null, new DateTimeOffset(2026, 9, 11, 23, 58, 0, Eastern));

    [Fact]
    public void Files_are_named_by_coordinates()
    {
        Assert.Equal("44.3000_-78.3300.json", ForecastCache.FileName(Peterborough));
        Assert.Equal("44.3000_-78.3300.json", ForecastCache.FileName(Peterborough with { Nickname = "Home" }));
    }

    [Fact]
    public void A_forecast_and_its_alerts_come_back_as_they_went_in()
    {
        var cache = Cache();
        var f = Sample();
        var alerts = Report(Alert("a", "Rainfall warning", new DateTimeOffset(2026, 9, 12, 6, 30, 0, Eastern)), Alert("b", "Special weather statement", null));

        cache.Save(Peterborough, f, alerts);
        var loaded = Cache().Load(Peterborough);

        Assert.NotNull(loaded);
        var g = loaded!.Forecast;
        Assert.Equal(f.FetchedAt, g.FetchedAt);
        Assert.Equal(f.UtcOffset, g.UtcOffset);
        Assert.Equal(f.Units, g.Units);
        Assert.Equal(f.Current, g.Current);
        Assert.Equal(DateTimeKind.Unspecified, g.Current.LocalTime.Kind);
        Assert.Equal(f.Hours, g.Hours);
        Assert.Equal(f.Days, g.Days);
        Assert.Equal(f.Periods, g.Periods);
        Assert.Equal(f.SourceName, g.SourceName);
        Assert.Equal(f.Sources, g.Sources);
        Assert.Equal(alerts, loaded.Alerts! with { Alerts = alerts.Alerts });
        Assert.Equal(alerts.Alerts, loaded.Alerts!.Alerts);
        Assert.False(File.Exists(Path.Combine(cache.Directory, ForecastCache.FileName(Peterborough) + ".tmp")));
    }

    [Fact]
    public void The_text_written_from_the_cache_is_the_text_written_from_the_fetch()
    {
        var cache = Cache();
        var f = Sample();
        cache.Save(Peterborough, f, AlertReport.NotAvailable);
        var loaded = Cache().Load(Peterborough)!;

        var options = new WriterOptions(new DateTimeOffset(2026, 9, 12, 1, 0, 0, Eastern), TimeZoneInfo.CreateCustomTimeZone("t", Eastern, "t", "t"), "h:mm tt", CultureInfo.InvariantCulture);
        Assert.Equal(
            SectionLayout.Build(ForecastWriter.Write(f, options, AlertReport.NotAvailable)).Text,
            SectionLayout.Build(ForecastWriter.Write(loaded.Forecast, options, loaded.Alerts)).Text);
        Assert.Same(AlertReport.NotAvailable, loaded.Alerts);
    }

    [Fact]
    public void Wall_clock_times_survive_a_dst_change_on_the_pc()
    {
        // 2:30 am on a spring-forward day does not exist in most zones; the
        // serializer's own date form would move it.
        var cache = Cache();
        var f = Sample();
        var current = f.Current with { LocalTime = new DateTime(2026, 3, 8, 2, 30, 0) };
        cache.Save(Peterborough, f with { Current = current }, null);

        Assert.Equal(current, Cache().Load(Peterborough)!.Forecast.Current);
    }

    [Fact]
    public void The_location_asked_for_is_the_one_read_back()
    {
        var cache = Cache();
        cache.Save(Peterborough, Sample(), null);
        var home = Peterborough with { Nickname = "Home" };

        Assert.Equal(home, Cache().Load(home)!.Forecast.Location);
    }

    [Fact]
    public void A_failed_check_keeps_the_alerts_from_the_last_one_that_succeeded()
    {
        var cache = Cache();
        var report = Report(Alert("a", "Rainfall warning", null));
        cache.Save(Peterborough, Sample(), report);

        cache.Save(Peterborough, Sample(), null);
        var loaded = Cache().Load(Peterborough)!;

        Assert.Equal(report.CheckedAt, loaded.Alerts!.CheckedAt);
        Assert.Equal(["a"], loaded.Alerts.Alerts.Select(a => a.Id).ToArray());
    }

    [Fact]
    public void The_alert_poll_updates_a_file_that_exists_and_starts_none()
    {
        var cache = Cache();
        var later = Report(Alert("b", "Frost advisory", null)) with { CheckedAt = new DateTimeOffset(2026, 9, 12, 0, 30, 0, Eastern) };

        cache.SaveAlerts(Peterborough, later);
        Assert.Null(Cache().Load(Peterborough));
        Assert.False(Directory.Exists(cache.Directory) && Directory.GetFiles(cache.Directory).Length > 0);

        cache.Save(Peterborough, Sample(), Report());
        cache.SaveAlerts(Peterborough, later);
        var loaded = Cache().Load(Peterborough)!;

        Assert.Equal(later.CheckedAt, loaded.Alerts!.CheckedAt);
        Assert.Equal(["b"], loaded.Alerts.Alerts.Select(a => a.Id).ToArray());
    }

    [Fact]
    public void Missing_unreadable_and_foreign_files_read_as_no_cache()
    {
        var cache = Cache();
        Assert.Null(cache.Load(Peterborough));

        Directory.CreateDirectory(cache.Directory);
        var path = Path.Combine(cache.Directory, ForecastCache.FileName(Peterborough));
        File.WriteAllText(path, "{ this is not json");
        Assert.Null(cache.Load(Peterborough));

        File.WriteAllText(path, "{\"version\":1,\"forecast\":{\"fetchedAt\":\"2026-09-11T23:58:00.0000000-04:00\"}}");
        Assert.Null(cache.Load(Peterborough));

        File.WriteAllText(path, "{\"version\":99}");
        Assert.Null(cache.Load(Peterborough));

        // A bad file is simply replaced by the next fetch.
        cache.Save(Peterborough, Sample(), null);
        Assert.NotNull(cache.Load(Peterborough));
    }

    [Fact]
    public void Pruning_keeps_the_saved_locations_and_drops_the_rest()
    {
        var cache = Cache();
        var toronto = new Location("Toronto", "Ontario", "Canada", 43.65, -79.38, "America/Toronto");
        cache.Prune([Peterborough]);
        cache.Save(Peterborough, Sample(), null);
        cache.Save(toronto, Sample(), null);
        File.WriteAllText(Path.Combine(cache.Directory, "notes.txt"), "not ours");

        cache.Prune([Peterborough with { Nickname = "Home" }]);

        Assert.NotNull(cache.Load(Peterborough));
        Assert.Null(cache.Load(toronto));
        Assert.True(File.Exists(Path.Combine(cache.Directory, "notes.txt")));
    }
}
