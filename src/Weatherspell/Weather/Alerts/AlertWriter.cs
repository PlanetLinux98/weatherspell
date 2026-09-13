using System.Text.RegularExpressions;

namespace Weatherspell.Weather.Alerts;

// The Alerts section, the spoken announcement of new alerts, and the text
// of the details dialog: one line per alert, event first, in the location's
// own time, with the service's own words for the details.
internal static class AlertWriter
{
    public const string Heading = "Alerts";
    public const string NotAvailableLine = "Alerts are not available for this region.";
    public const string NoneLine = "No alerts in effect.";

    // A null report means no check was made (the writer's tests); it reads
    // the same as an uncovered region rather than as a quiet day.
    public static Section Section(AlertReport? report, Clock clock, DateTime nowLocal)
    {
        if (report is null || !report.IsAvailable) return new Section(Heading, [NotAvailableLine]);
        if (report.Problem is not null) return new Section(Heading, [$"Alerts couldn't be checked this time ({report.Problem}). Press F5 to try again."]);
        if (report.Alerts.Count == 0) return new Section(Heading, [NoneLine]);
        return new Section(Heading, report.Alerts.Select(a => Line(a, clock, nowLocal)).ToList(), report.Alerts);
    }

    public static string Line(WeatherAlert a, Clock clock, DateTime nowLocal) =>
        a.Ends is DateTimeOffset ends
            ? $"{a.Event} until {clock.TimeOnDay(clock.Local(ends), nowLocal)}, from {a.Source}. Press Enter for details."
            : $"{a.Event} from {a.Source}. Press Enter for details.";

    // "Peterborough: severe thunderstorm warning until 6:00 pm today."
    public static string Announcement(string location, IReadOnlyList<WeatherAlert> alerts, Clock clock, DateTime nowLocal)
    {
        var parts = alerts.Select(a =>
            a.Ends is DateTimeOffset ends
                ? $"{Lower(a.Event)} until {clock.TimeOnDay(clock.Local(ends), nowLocal)}"
                : Lower(a.Event)).ToList();
        return $"{location}: {JoinAnd(parts)}.";
    }

    public static IReadOnlyList<string> Details(WeatherAlert a, Clock clock, DateTime nowLocal)
    {
        string When(DateTimeOffset t) => clock.TimeOnDay(clock.Local(t), nowLocal);
        var paragraphs = new List<string>();
        string? timing = null;
        if (a.Ends is DateTimeOffset ends)
        {
            timing = a.Onset is DateTimeOffset onset && clock.Local(onset) > nowLocal
                ? $"from {When(onset)} until {When(ends)}"
                : $"until {When(ends)}";
        }
        paragraphs.Add(timing is null ? $"{a.Event} from {a.Sender}." : $"{a.Event} from {a.Sender}, in effect {timing}.");
        if (a.Area.Length > 0) paragraphs.Add($"Area: {a.Area}.");
        paragraphs.Add($"Issued {When(a.Issued)}.");
        if (a.Level is not null) paragraphs.Add(a.Level + ".");
        paragraphs.AddRange(Paragraphs(a.Description));
        if (a.Instruction is not null)
        {
            var instructions = Paragraphs(a.Instruction);
            if (instructions.Count > 0)
            {
                instructions[0] = "Instructions: " + instructions[0];
                paragraphs.AddRange(instructions);
            }
        }
        return paragraphs;
    }

    // The service's text in paragraphs: blank lines separate them, single
    // line breaks are the NWS's hard wrapping, "###" is EC's rule between
    // the alert and its boilerplate, "- " starts one of the NWS's sub-items,
    // and "* WHAT..." bullets become "What:" so a listener hears the label
    // rather than "star" and "dot dot dot".
    public static List<string> Paragraphs(string text)
    {
        var list = new List<string>();
        foreach (var block in Regex.Split(text.Replace("\r\n", "\n"), @"\n\s*\n"))
        {
            foreach (var item in Regex.Split(block, @"\n(?=- )"))
            {
                var s = Regex.Replace(item.Trim(), @"\s*\n\s*", " ");
                s = Regex.Replace(s, @"\s{2,}", " ");
                if (s.StartsWith("- ", StringComparison.Ordinal)) s = s.Substring(2);
                if (s.Length == 0 || s.Trim('#').Length == 0) continue;
                s = Regex.Replace(s, @"^\*\s*([A-Z][A-Z ]*[A-Z])\.\.\.\s*", m => OfficialText.SentenceCase(m.Groups[1].Value) + ": ");
                list.Add(OfficialText.Spoken(s));
            }
        }
        return list;
    }

    private static string Lower(string s) => s.Length == 0 ? s : char.ToLowerInvariant(s[0]) + s.Substring(1);

    private static string JoinAnd(List<string> parts) => parts.Count switch
    {
        0 => "",
        1 => parts[0],
        _ => string.Join(", ", parts.Take(parts.Count - 1)) + " and " + parts[parts.Count - 1],
    };
}
