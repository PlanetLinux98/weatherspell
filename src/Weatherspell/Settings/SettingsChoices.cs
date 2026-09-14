namespace Weatherspell.Settings;

// What the Settings dialog offers for each field, kept out of the form so
// the lists and their wording are testable. The intervals are short lists
// in a combo box rather than a number field: a native combo box is the
// control screen readers read best, and there is nothing to mistype. A
// value outside the list (a hand-edited settings.json) is kept and shown
// as its own entry, in order, so opening the dialog never changes it.
internal static class SettingsChoices
{
    public static readonly int[] ForecastMinutes = [15, 30, 60, 120];
    public static readonly int[] AlertMinutes = [5, 10, 15, 30];

    public static readonly (string Value, string Label)[] Announcements =
    [
        ("all", "All new alerts"),
        ("severe", "Severe and extreme only"),
        ("off", "Off"),
    ];

    public static int[] Minutes(int[] presets, int current)
    {
        var list = presets.ToList();
        if (current >= 1 && !list.Contains(current))
        {
            list.Add(current);
            list.Sort();
        }
        return [.. list];
    }

    // "15 minutes", "1 hour", "1 hour and 30 minutes", "2 hours".
    public static string MinutesLabel(int minutes) => Weather.Clock.Duration(minutes * 60.0);

    public static string AnnouncementLabel(string value)
    {
        foreach (var (v, label) in Announcements)
        {
            if (v == value) return label;
        }
        return Announcements[0].Label;
    }
}
