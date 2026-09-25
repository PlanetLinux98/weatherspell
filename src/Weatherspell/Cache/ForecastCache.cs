using System.Globalization;
using System.Runtime.Serialization;
using System.Runtime.Serialization.Json;
using System.Text;
using System.Xml;
using Weatherspell.Settings;
using Weatherspell.Weather;
using Weatherspell.Weather.Alerts;

namespace Weatherspell.Cache;

// The last forecast and alerts read from a location's cache file.
internal sealed record CachedForecast(Forecast Forecast, AlertReport? Alerts);

// One file per saved location under %APPDATA%\Weatherspell\cache\: the last
// forecast and alerts fetched for it, so a launch or a switch that cannot
// reach the weather service still has something to read. Overwritten on
// every successful fetch and never appended to, so the cache is the size
// of the saved locations and no more; files for locations no longer saved
// are pruned at startup. Read only after a fetch has failed: showing it
// first and swapping in the live text would replace the words under a
// reader (see NOTES.md).
internal sealed class ForecastCache
{
    public string Directory { get; }

    public ForecastCache(string directory)
    {
        Directory = directory;
    }

    // Beside settings.json, so a test's store gets a test cache too.
    public static ForecastCache Beside(SettingsStore store) =>
        new(Path.Combine(Path.GetDirectoryName(store.Path)!, "cache"));

    // Coordinates, not the name: a nickname or a rename keeps the file, and
    // nothing from a geocoder's name has to be made safe for a filename.
    public static string FileName(Location l) =>
        string.Format(CultureInfo.InvariantCulture, "{0:F4}_{1:F4}.json", l.Latitude, l.Longitude);

    private string PathFor(Location l) => Path.Combine(Directory, FileName(l));

    // Null when there is no file or it cannot be read; the next successful
    // fetch replaces a bad one, so nothing is reported. Any failure, not
    // only the expected ones: a damaged file (a null entry, an offset out
    // of range) raised the .NET error dialog (#22), and the cache is only
    // ever a convenience.
    public CachedForecast? Load(Location location)
    {
        try
        {
            var file = Read(PathFor(location));
            return file?.Forecast is null ? null : new CachedForecast(file.Forecast.To(location), file.Alerts?.To());
        }
        catch (Exception)
        {
            return null;
        }
    }

    // alerts is null when this fetch's check failed: the file keeps the
    // alerts from the last check that succeeded, still dated by it.
    public void Save(Location location, Forecast forecast, AlertReport? alerts)
    {
        var path = PathFor(location);
        var file = Read(path) ?? new CacheFile();
        file.Forecast = ForecastData.From(forecast);
        if (alerts is not null) file.Alerts = AlertsData.From(alerts);
        Write(path, file);
    }

    // The alert poll's result for a location that has a file; one without
    // a forecast would have nothing to show, so none is started here.
    public void SaveAlerts(Location location, AlertReport alerts)
    {
        var path = PathFor(location);
        var file = Read(path);
        if (file?.Forecast is null) return;
        file.Alerts = AlertsData.From(alerts);
        Write(path, file);
    }

    public void Prune(IEnumerable<Location> keep)
    {
        if (!System.IO.Directory.Exists(Directory)) return;
        var names = new HashSet<string>(keep.Select(FileName), StringComparer.OrdinalIgnoreCase);
        foreach (var path in System.IO.Directory.GetFiles(Directory, "*.json"))
        {
            if (!names.Contains(Path.GetFileName(path))) File.Delete(path);
        }
    }

    private static CacheFile? Read(string path)
    {
        if (!File.Exists(path)) return null;
        try
        {
            // As text first, as the settings store does: a byte-order mark
            // would be rejected by the JSON reader.
            var json = File.ReadAllText(path, Encoding.UTF8);
            using var stream = new MemoryStream(Encoding.UTF8.GetBytes(json));
            var file = (CacheFile?)new DataContractJsonSerializer(typeof(CacheFile)).ReadObject(stream);
            return file?.Version == CacheFile.CurrentVersion ? file : null;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or SerializationException or XmlException)
        {
            return null;
        }
    }

    // A temp file swapped in, as the settings store does, so a crash
    // mid-write never leaves a half-written file behind.
    private static void Write(string path, CacheFile file)
    {
        System.IO.Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        var temp = path + ".tmp";
        using (var stream = File.Create(temp))
        using (var writer = JsonReaderWriterFactory.CreateJsonWriter(stream, Encoding.UTF8, ownsStream: false))
        {
            new DataContractJsonSerializer(typeof(CacheFile)).WriteObject(writer, file);
            writer.Flush();
        }
        if (File.Exists(path))
        {
            File.Replace(temp, path, null);
        }
        else
        {
            File.Move(temp, path);
        }
    }
}
