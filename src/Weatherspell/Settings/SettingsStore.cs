using System.Runtime.Serialization.Json;
using System.Text;

namespace Weatherspell.Settings;

// settings.json under %APPDATA%\Weatherspell: survives winget upgrades and a
// read-only exe location, unlike a file beside the exe (see NOTES.md).
internal sealed class SettingsStore
{
    public string Path { get; }

    // Set when the last Load() fell back to defaults, so the UI can say so once.
    public string? LoadProblem { get; private set; }

    public SettingsStore(string path)
    {
        Path = path;
    }

    public static SettingsStore Default() =>
        new(System.IO.Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "Weatherspell", "settings.json"));

    public AppSettings Load()
    {
        LoadProblem = null;
        if (!File.Exists(Path))
        {
            return new AppSettings();
        }
        try
        {
            using var stream = File.OpenRead(Path);
            var serializer = new DataContractJsonSerializer(typeof(AppSettings));
            var settings = (AppSettings?)serializer.ReadObject(stream) ?? new AppSettings();
            settings.Normalize();
            return settings;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or System.Runtime.Serialization.SerializationException or System.Xml.XmlException)
        {
            LoadProblem = $"Settings could not be read ({ex.Message}); starting with defaults.";
            return new AppSettings();
        }
    }

    // Write to a temp file then swap it in, so a crash mid-write never leaves
    // a half-written settings.json behind.
    public void Save(AppSettings settings)
    {
        settings.Normalize();
        var directory = System.IO.Path.GetDirectoryName(Path)!;
        Directory.CreateDirectory(directory);
        var temp = Path + ".tmp";
        using (var stream = File.Create(temp))
        using (var writer = JsonReaderWriterFactory.CreateJsonWriter(stream, Encoding.UTF8, ownsStream: false, indent: true))
        {
            new DataContractJsonSerializer(typeof(AppSettings)).WriteObject(writer, settings);
            writer.Flush();
        }
        if (File.Exists(Path))
        {
            File.Replace(temp, Path, null);
        }
        else
        {
            File.Move(temp, Path);
        }
    }
}
