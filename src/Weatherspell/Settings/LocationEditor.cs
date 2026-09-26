using Weatherspell.Weather;

namespace Weatherspell.Settings;

// The Manage Locations dialog's working copy of the saved locations. Every
// edit stays here until Commit, so Cancel undoes all of them and Remove
// needs no "are you sure". An entry keeps the SavedLocation it came from,
// so the seen alert ids and the last UTC offset move with it; its nickname
// and notify flag are staged beside it rather than written into it, since
// an alert check can run while the dialog is open.
internal sealed class LocationEditor
{
    private readonly List<LocationEntry> _entries;

    public LocationEditor(IEnumerable<SavedLocation> saved)
    {
        _entries = saved.Select(s => new LocationEntry(s, s.ToLocation(), s.Nickname, s.NotifyAlerts)).ToList();
    }

    public IReadOnlyList<LocationEntry> Entries => _entries;

    // False at either end of the list, where there is nowhere to go.
    public bool Move(int index, int by)
    {
        var to = index + by;
        if (index < 0 || index >= _entries.Count || to < 0 || to >= _entries.Count) return false;
        var entry = _entries[index];
        _entries.RemoveAt(index);
        _entries.Insert(to, entry);
        return true;
    }

    public void Remove(int index) => _entries.RemoveAt(index);

    // A place already in the list is not added twice; its index comes
    // back with Added false.
    public (int Index, bool Added) Add(Location place)
    {
        var existing = _entries.FindIndex(e => e.Place.IsSamePlace(place));
        if (existing >= 0) return (existing, false);
        _entries.Add(new LocationEntry(null, place, null, notifyAlerts: true));
        return (_entries.Count - 1, true);
    }

    // Blank goes back to the full name.
    public void Rename(int index, string? nickname) =>
        _entries[index].Nickname = string.IsNullOrWhiteSpace(nickname) ? null : nickname!.Trim();

    public void SetNotify(int index, bool notify) => _entries[index].NotifyAlerts = notify;

    // The saved list as edited: the same objects for the locations that
    // were there, new ones for places added.
    public List<SavedLocation> Commit() =>
        _entries.Select(e =>
        {
            var saved = e.Original ?? SavedLocation.From(e.Place);
            saved.Nickname = e.Nickname;
            saved.NotifyAlerts = e.NotifyAlerts;
            return saved;
        }).ToList();

    // Which location of the committed list to show: the one that was on
    // screen, wherever it has moved to; if it was removed, the one that
    // now stands where it stood (or the last, if it stood at the end); -1
    // when none are left.
    public static int Shown(List<SavedLocation> committed, SavedLocation? before, int beforeIndex)
    {
        if (committed.Count == 0) return -1;
        var index = before is null ? -1 : committed.IndexOf(before);
        return index >= 0 ? index : Math.Min(Math.Max(beforeIndex, 0), committed.Count - 1);
    }
}

internal sealed class LocationEntry(SavedLocation? original, Location place, string? nickname, bool notifyAlerts)
{
    // Null for a place added in the dialog.
    public SavedLocation? Original { get; } = original;
    public Location Place { get; } = place;
    public string? Nickname { get; set; } = nickname;
    public bool NotifyAlerts { get; set; } = notifyAlerts;

    public string DisplayName => Nickname ?? Place.FullName;

    // "Home (Peterborough, Ontario, Canada)": the full name stays in view
    // behind a nickname. A location whose alerts are not spoken says so,
    // so arrowing through the list tells everything the dialog holds.
    public override string ToString()
    {
        var text = Nickname is null ? Place.FullName : $"{Nickname} ({Place.FullName})";
        return NotifyAlerts ? text : text + "; no alert notifications";
    }
}
