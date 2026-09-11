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
