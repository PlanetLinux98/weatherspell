using Weatherspell.Settings;
using Weatherspell.Weather;
using Xunit;

namespace Weatherspell.Tests;

public class LocationEditorTests
{
    private static readonly Location Peterborough = new("Peterborough", "Ontario", "Canada", 44.30012, -78.31623, "America/Toronto");
    private static readonly Location Albany = new("Albany", "New York", "United States", 42.65258, -73.75623, "America/New_York");
    private static readonly Location Paris = new("Paris", "Ile-de-France", "France", 48.85341, 2.3488, "Europe/Paris");

    private static List<SavedLocation> Saved(params Location[] places) => places.Select(SavedLocation.From).ToList();

    private static string[] Names(LocationEditor editor) => editor.Entries.Select(e => e.Place.Name).ToArray();

    [Fact]
    public void Moves_stop_at_either_end()
    {
        var editor = new LocationEditor(Saved(Peterborough, Albany, Paris));

        Assert.True(editor.Move(2, -1));
        Assert.Equal(["Peterborough", "Paris", "Albany"], Names(editor));
        Assert.False(editor.Move(0, -1));
        Assert.False(editor.Move(2, +1));
        Assert.Equal(["Peterborough", "Paris", "Albany"], Names(editor));
    }

    [Fact]
    public void A_place_already_in_the_list_is_not_added_twice()
    {
        var editor = new LocationEditor(Saved(Peterborough, Albany));

        Assert.Equal((1, false), editor.Add(Albany with { Latitude = 42.652581 }));
        Assert.Equal((2, true), editor.Add(Paris));
        Assert.True(editor.Entries[2].NotifyAlerts);
    }

    [Fact]
    public void The_list_names_a_nickname_with_the_full_name_and_says_when_alerts_are_not_spoken()
    {
        var editor = new LocationEditor(Saved(Peterborough));

        Assert.Equal("Peterborough, Ontario, Canada", editor.Entries[0].ToString());
        editor.Rename(0, "  Home ");
        Assert.Equal("Home (Peterborough, Ontario, Canada)", editor.Entries[0].ToString());
        Assert.Equal("Home", editor.Entries[0].DisplayName);
        editor.SetNotify(0, false);
        Assert.Equal("Home (Peterborough, Ontario, Canada); no alert notifications", editor.Entries[0].ToString());
        editor.Rename(0, " ");
        Assert.Null(editor.Entries[0].Nickname);
        Assert.Equal("Peterborough, Ontario, Canada", editor.Entries[0].DisplayName);
    }

    // Seen alert ids and the UTC offset live on the saved object, so they
    // must survive a move and a rename; nothing is written before Commit.
    [Fact]
    public void Commit_keeps_the_saved_objects_and_touches_nothing_before()
    {
        var saved = Saved(Peterborough, Albany);
        saved[1].SeenAlertIds.Add("alert-1");
        var editor = new LocationEditor(saved);

        editor.Move(1, -1);
        editor.Rename(0, "Work");
        editor.SetNotify(0, false);
        editor.Remove(1);
        editor.Add(Paris);
        Assert.Null(saved[1].Nickname);
        Assert.True(saved[1].NotifyAlerts);

        var committed = editor.Commit();

        Assert.Equal(2, committed.Count);
        Assert.Same(saved[1], committed[0]);
        Assert.Equal("Work", committed[0].Nickname);
        Assert.False(committed[0].NotifyAlerts);
        Assert.Equal(["alert-1"], committed[0].SeenAlertIds);
        Assert.Equal("Paris", committed[1].Name);
        Assert.True(committed[1].NotifyAlerts);
    }

    [Fact]
    public void The_location_on_screen_is_followed_or_replaced_by_the_one_in_its_place()
    {
        var saved = Saved(Peterborough, Albany, Paris);

        // Moved: still shown, at its new place.
        Assert.Equal(0, LocationEditor.Shown([saved[1], saved[0], saved[2]], saved[1], 1));
        // Removed: the one now where it stood.
        Assert.Equal(1, LocationEditor.Shown([saved[0], saved[2]], saved[1], 1));
        // Removed from the end: the new last one.
        Assert.Equal(1, LocationEditor.Shown([saved[0], saved[1]], saved[2], 2));
        // None left.
        Assert.Equal(-1, LocationEditor.Shown([], saved[0], 0));
        // Nothing was on screen: the first.
        Assert.Equal(0, LocationEditor.Shown([saved[0]], null, -1));
    }
}
