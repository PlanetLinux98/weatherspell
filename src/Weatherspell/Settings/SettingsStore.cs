using System.Runtime.Serialization.Json;
using System.Text;

namespace Weatherspell.Settings;

// settings.json under %APPDATA%\Weatherspell: survives winget upgrades and a
// read-only exe location, unlike a file beside the exe (see NOTES.md).
internal sealed class SettingsStore
{
    public string Path { get; }

    // Set when the last Load() fell back to defaults, so the UI can say so
    // once: LoadProblem in plain words, LoadError the reader's own message.
    public string? LoadProblem { get; private set; }
    public string? LoadError { get; private set; }

    // Where an unreadable settings.json is kept: the next save replaces the
    // file, and the locations in it would otherwise be lost for good (#22).
    public string BadCopyPath => Path + ".bad";

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
        LoadError = null;
        if (!File.Exists(Path))
        {
            return new AppSettings();
        }
        try
        {
            // Read as text first: File.ReadAllText drops a UTF-8 byte-order
            // mark, which the JSON reader would otherwise reject, and Notepad
            // and PowerShell both write one.
            var json = File.ReadAllText(Path, Encoding.UTF8);
            using var stream = new MemoryStream(Encoding.UTF8.GetBytes(json));
            var serializer = new DataContractJsonSerializer(typeof(AppSettings));
            var settings = (AppSettings?)serializer.ReadObject(stream) ?? new AppSettings();
            settings.Normalize();
            return settings;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or System.Runtime.Serialization.SerializationException or System.Xml.XmlException)
        {
            LoadError = ex.Message;
            var folder = System.IO.Path.GetDirectoryName(Path);
            try
            {
                File.Copy(Path, BadCopyPath, overwrite: true);
                LoadProblem = $"Weatherspell couldn't read its settings, so it has started without your saved locations. The file it couldn't read was kept as {System.IO.Path.GetFileName(BadCopyPath)} in {folder}.";
            }
            catch (Exception copy) when (copy is IOException or UnauthorizedAccessException)
            {
                LoadProblem = $"Weatherspell couldn't read its settings, so it has started without your saved locations. The file is settings.json in {folder}.";
            }
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
