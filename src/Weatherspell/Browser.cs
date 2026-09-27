using System.ComponentModel;
using System.Diagnostics;
using System.IO;

namespace Weatherspell;

// Pages and links open in the program the user has chosen for them. A
// failure says so and shows where it was going, so it can be reached by hand.
internal static class Browser
{
    public const string RepositoryUrl = "https://github.com/PlanetLinux98/weatherspell";

    public static void Open(IWin32Window owner, string target, string caption)
    {
        try
        {
            Process.Start(new ProcessStartInfo(target) { UseShellExecute = true });
        }
        catch (Exception ex) when (ex is Win32Exception or InvalidOperationException)
        {
            MessageBox.Show(owner, $"Couldn't open the browser: {ex.Message}\n\n{target}", caption, MessageBoxButtons.OK, MessageBoxIcon.Warning);
        }
    }

    public static void OpenGuide(IWin32Window owner, string? section = null)
    {
        const string caption = "Weatherspell User Guide";
        string page;
        try
        {
            page = UserGuide.Prepare(UserGuide.Folder, section);
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            MessageBox.Show(owner, $"Couldn't open the user guide: {ex.Message}", caption, MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return;
        }
        Open(owner, page, caption);
    }
}
