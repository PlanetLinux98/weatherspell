using System.Drawing;
using Weatherspell.Settings;
using Xunit;

namespace Weatherspell.Tests;

public class WindowPlacementTests : IDisposable
{
    // A 150 percent display: 1920 x 1080 less a 72-pixel taskbar, with a
    // second screen to its right; the caption as SystemInformation gives it,
    // and Segoe UI 9 pt's average character as WinForms measures it there
    // (7 by 15 at 96 DPI; not in proportion).
    private static readonly Rectangle Main = new(0, 0, 1920, 1008);
    private static readonly Rectangle Right = new(1920, 0, 1920, 1008);
    private const int Caption = 34;
    private static readonly SizeF Char150 = new(10, 25);
    private static readonly SizeF Char100 = new(7, 15);

    private static SavedWindow Saved(int left, int top, int width = 1080, int height = 840, bool maximized = false, double charWidth = 10, double charHeight = 25) =>
        new() { Left = left, Top = top, Width = width, Height = height, Maximized = maximized, CharWidth = charWidth, CharHeight = charHeight };

    [Fact]
    public void A_window_on_a_current_screen_comes_back_where_it_was()
    {
        Assert.Equal(new Rectangle(200, 100, 1080, 840), WindowPlacement.Restore(Saved(200, 100), Char150, [Main], Caption));
        Assert.Equal(new Rectangle(2300, 50, 1080, 840), WindowPlacement.Restore(Saved(2300, 50), Char150, [Main, Right], Caption));
    }

    [Fact]
    public void A_window_on_a_screen_that_has_gone_opens_centred()
    {
        Assert.Null(WindowPlacement.Restore(Saved(2300, 50), Char150, [Main], Caption));
        // Only the last few pixels of the title bar still on the screen.
        Assert.Null(WindowPlacement.Restore(Saved(1880, 50), Char150, [Main], Caption));
        // Title bar above the top of the screen.
        Assert.Null(WindowPlacement.Restore(Saved(200, -200), Char150, [Main], Caption));
    }

    [Fact]
    public void The_size_follows_a_change_of_display_scale_or_text_size()
    {
        Assert.Equal(new Rectangle(200, 100, 756, 504), WindowPlacement.Restore(Saved(200, 100), Char100, [Main], Caption));
        // No character size recorded: taken as it is.
        Assert.Equal(new Rectangle(200, 100, 1080, 840), WindowPlacement.Restore(Saved(200, 100, charWidth: 0, charHeight: 0), Char100, [Main], Caption));
    }

    [Fact]
    public void A_window_hanging_off_its_screen_is_brought_back_onto_it()
    {
        // Saved on a larger display: shrunk to the screen and moved in.
        Assert.Equal(new Rectangle(0, 0, 1920, 1008), WindowPlacement.Restore(Saved(100, 60, 2400, 1300), Char150, [Main], Caption));
        // Its bottom below the taskbar.
        Assert.Equal(new Rectangle(600, 168, 1080, 840), WindowPlacement.Restore(Saved(600, 500), Char150, [Main], Caption));
    }

    [Fact]
    public void A_snapped_window_keeps_its_reach_past_the_edge()
    {
        // Snapped left at 150 percent: the invisible resize border takes the
        // bounds 11 pixels past the left edge and the bottom.
        Assert.Equal(new Rectangle(-11, 0, 982, 1019), WindowPlacement.Restore(Saved(-11, 0, 982, 1019), Char150, [Main], Caption));
    }

    [Fact]
    public void Save_records_the_bounds_state_and_character_size()
    {
        var saved = WindowPlacement.Save(new Rectangle(200, 100, 1080, 840), maximized: true, new SizeF(9.916667f, 25f));

        Assert.Equal(Saved(200, 100, maximized: true, charWidth: 9.92), saved);
    }

    private readonly string _dir = Path.Combine(Path.GetTempPath(), "weatherspell-tests-" + Guid.NewGuid().ToString("N"));

    public void Dispose()
    {
        if (Directory.Exists(_dir)) Directory.Delete(_dir, recursive: true);
    }

    [Fact]
    public void The_window_round_trips_through_settings_and_a_bad_one_is_dropped()
    {
        var store = new SettingsStore(Path.Combine(_dir, "settings.json"));
        Assert.Null(store.Load().Window);

        var settings = new AppSettings { Window = Saved(200, 100, maximized: true) };
        store.Save(settings);
        Assert.Equal(Saved(200, 100, maximized: true), store.Load().Window);

        File.WriteAllText(store.Path, "{\"window\":{\"left\":5,\"top\":5,\"width\":0,\"height\":400}}");
        Assert.Null(store.Load().Window);
        Assert.Null(store.LoadProblem);
    }
}
