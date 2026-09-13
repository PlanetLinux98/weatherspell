using Weatherspell.Settings;
using Weatherspell.Weather;
using Xunit;

namespace Weatherspell.Tests;

public class SettingsStoreTests : IDisposable
{
    private readonly string _dir = Path.Combine(Path.GetTempPath(), "weatherspell-tests-" + Guid.NewGuid().ToString("N"));

    private SettingsStore Store() => new(Path.Combine(_dir, "nested", "settings.json"));

    public void Dispose()
    {
        if (Directory.Exists(_dir)) Directory.Delete(_dir, recursive: true);
    }

    [Fact]
    public void Missing_file_gives_defaults_without_a_problem()
    {
        var store = Store();
        var settings = store.Load();

        Assert.Empty(settings.Locations);
        Assert.Equal(0, settings.LastLocation);
        Assert.Null(store.LoadProblem);
    }

    [Fact]
    public void Round_trips_locations_and_creates_the_folder()
    {
        var store = Store();
        var settings = new AppSettings();
        settings.Locations.Add(SavedLocation.From(new Location("Peterborough", "Ontario", "Canada", 44.3, -78.32, "America/Toronto", "Home")));
        settings.Locations.Add(SavedLocation.From(new Location("Reykjavik", null, "Iceland", 64.15, -21.94, "Atlantic/Reykjavik")));
        settings.LastLocation = 1;

        store.Save(settings);
        var loaded = Store().Load();

        Assert.Equal(2, loaded.Locations.Count);
        Assert.Equal("Home", loaded.Locations[0].ToLocation().DisplayName);
        Assert.Equal("Peterborough, Ontario, Canada", loaded.Locations[0].ToLocation().FullName);
        Assert.Equal("Reykjavik, Iceland", loaded.Locations[1].ToLocation().FullName);
        Assert.Equal(1, loaded.LastLocation);
        Assert.True(loaded.Locations[0].NotifyAlerts);
        Assert.False(File.Exists(store.Path + ".tmp"));
    }

    [Fact]
    public void Corrupt_file_falls_back_to_defaults_and_reports_it()
    {
        var store = Store();
        Directory.CreateDirectory(Path.GetDirectoryName(store.Path)!);
        File.WriteAllText(store.Path, "{ this is not json");

        var settings = store.Load();

        Assert.Empty(settings.Locations);
        Assert.NotNull(store.LoadProblem);
        Assert.Contains("defaults", store.LoadProblem);
    }

    [Fact]
    public void Reads_a_file_saved_with_a_byte_order_mark()
    {
        // Notepad and PowerShell's Set-Content both write UTF-8 with a BOM.
        var store = Store();
        Directory.CreateDirectory(Path.GetDirectoryName(store.Path)!);
        File.WriteAllText(store.Path, "{\"version\":1,\"locations\":[{\"name\":\"Toronto\",\"latitude\":43.65,\"longitude\":-79.38}],\"lastLocation\":0}", new System.Text.UTF8Encoding(encoderShouldEmitUTF8Identifier: true));

        var settings = store.Load();

        Assert.Null(store.LoadProblem);
        Assert.Single(settings.Locations);
        Assert.Equal("Toronto", settings.Locations[0].Name);
    }

    [Fact]
    public void Round_trips_alert_state_and_settings()
    {
        var store = Store();
        var settings = new AppSettings { AlertCheckMinutes = 5, AlertAnnouncements = "severe" };
        var home = SavedLocation.From(new Location("Peterborough", "Ontario", "Canada", 44.3, -78.32, "America/Toronto"));
        home.SeenAlertIds.Add("ec:64919237566271632202609120507");
        home.UtcOffsetSeconds = -14400;
        home.NotifyAlerts = false;
        settings.Locations.Add(home);

        store.Save(settings);
        var loaded = Store().Load();

        Assert.Equal(5, loaded.AlertCheckMinutes);
        Assert.Equal("severe", loaded.AlertAnnouncements);
        Assert.Equal(["ec:64919237566271632202609120507"], loaded.Locations[0].SeenAlertIds);
        Assert.Equal(-14400, loaded.Locations[0].UtcOffsetSeconds);
        Assert.False(loaded.Locations[0].NotifyAlerts);
    }

    [Fact]
    public void Older_files_and_odd_values_normalize_to_the_defaults()
    {
        var store = Store();
        Directory.CreateDirectory(Path.GetDirectoryName(store.Path)!);
        File.WriteAllText(store.Path, "{\"version\":1,\"locations\":[{\"name\":\"X\",\"latitude\":1,\"longitude\":2}],\"lastLocation\":0,\"alertCheckMinutes\":0,\"alertAnnouncements\":\"loud\"}");

        var settings = store.Load();

        Assert.Equal(10, settings.AlertCheckMinutes);
        Assert.Equal("all", settings.AlertAnnouncements);
        Assert.Empty(settings.Locations[0].SeenAlertIds);
        Assert.Null(settings.Locations[0].UtcOffsetSeconds);
        Assert.True(settings.Locations[0].NotifyAlerts);
    }

    [Theory]
    [InlineData("all", "Minor", true)]
    [InlineData("severe", "Moderate", false)]
    [InlineData("severe", "Severe", true)]
    [InlineData("severe", "Extreme", true)]
    [InlineData("off", "Extreme", false)]
    public void The_announcement_setting_is_a_severity_threshold(string setting, string severity, bool spoken)
    {
        var parsed = (Weather.Alerts.AlertSeverity)Enum.Parse(typeof(Weather.Alerts.AlertSeverity), severity);
        Assert.Equal(spoken, new AppSettings { AlertAnnouncements = setting }.Announces(parsed));
    }

    [Fact]
    public void Out_of_range_last_location_is_clamped()
    {
        var store = Store();
        Directory.CreateDirectory(Path.GetDirectoryName(store.Path)!);
        File.WriteAllText(store.Path, "{\"version\":1,\"locations\":[{\"name\":\"X\",\"latitude\":1,\"longitude\":2}],\"lastLocation\":7}");

        var settings = store.Load();

        Assert.Single(settings.Locations);
        Assert.Equal(0, settings.LastLocation);
    }
}
