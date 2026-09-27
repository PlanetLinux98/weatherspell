using System.IO;
using System.Text;

namespace Weatherspell;

// The guide is built into the exe (one file, nothing beside it) and written
// out to a temporary file when asked for, so it opens in whatever the user
// has chosen for web pages, works offline and always matches this version.
internal static class UserGuide
{
    public const string FileName = "user-guide.html";
    public const string Credits = "credits-and-licences";

    public static string Folder => Path.Combine(Path.GetTempPath(), "Weatherspell");

    public static string Html()
    {
        using var stream = typeof(UserGuide).Assembly.GetManifestResourceStream(FileName)
            ?? throw new InvalidOperationException("The user guide is missing from the exe.");
        using var reader = new StreamReader(stream, Encoding.UTF8);
        return reader.ReadToEnd();
    }

    // Returns the page to open. Opening a file drops any #fragment, so a
    // section is reached through a small page that forwards to it.
    public static string Prepare(string folder, string? section = null)
    {
        var utf8 = new UTF8Encoding(false);
        Directory.CreateDirectory(folder);
        var guide = Path.Combine(folder, FileName);
        File.WriteAllText(guide, Html(), utf8);
        if (section is null) return guide;

        var target = $"{FileName}#{section}";
        var forward = Path.Combine(folder, $"user-guide-{section}.html");
        File.WriteAllText(forward,
            $"<!DOCTYPE html>\n<html lang=\"en-CA\">\n<head>\n<meta charset=\"utf-8\">\n" +
            $"<meta http-equiv=\"refresh\" content=\"0; url={target}\">\n<title>Weatherspell User Guide</title>\n</head>\n" +
            $"<body>\n<p><a href=\"{target}\">Weatherspell User Guide</a></p>\n</body>\n</html>\n",
            utf8);
        return forward;
    }
}
