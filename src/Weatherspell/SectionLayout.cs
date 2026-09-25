using Weatherspell.Weather;
using Weatherspell.Weather.Alerts;

namespace Weatherspell;

// The forecast text box's contents, built from the writer's sections:
// heading line, blank line, paragraphs separated by blank lines, blank
// line, a steady rhythm for line-by-line reading. Records where each
// heading starts (Ctrl+PageDown/PageUp) and which characters are an alert
// line (Enter opens it), and maps a caret across a rewrite so a refresh
// the user did not ask for leaves them on the same words. Line breaks are
// "\n", one character, as RichEdit counts them (ReadingBox), so offsets
// here are the box's own.
internal sealed class SectionLayout
{
    private const string Break = "\n";

    public static readonly SectionLayout Empty = Build([]);

    public string Text { get; }
    public IReadOnlyList<(string Heading, int Offset)> Headings { get; }
    public IReadOnlyList<(int Start, int End, WeatherAlert Alert)> AlertRanges { get; }

    private SectionLayout(string text, List<(string, int)> headings, List<(int, int, WeatherAlert)> alertRanges)
    {
        Text = text;
        Headings = headings;
        AlertRanges = alertRanges;
    }

    public static SectionLayout Build(IReadOnlyList<Section> sections)
    {
        var sb = new System.Text.StringBuilder();
        var headings = new List<(string, int)>();
        var alertRanges = new List<(int, int, WeatherAlert)>();
        foreach (var section in sections)
        {
            if (sb.Length > 0) sb.Append(Break);
            headings.Add((section.Heading, sb.Length));
            sb.Append(section.Heading).Append(Break).Append(Break);
            for (var i = 0; i < section.Paragraphs.Count; i++)
            {
                var paragraph = section.Paragraphs[i];
                if (section.Alerts is not null && i < section.Alerts.Count && section.Alerts[i] is WeatherAlert alert)
                {
                    alertRanges.Add((sb.Length, sb.Length + paragraph.Length, alert));
                }
                sb.Append(paragraph).Append(Break).Append(Break);
            }
        }
        return new SectionLayout(sb.ToString(), headings, alertRanges);
    }

    public WeatherAlert? AlertAt(int offset)
    {
        foreach (var (start, end, alert) in AlertRanges)
        {
            if (offset >= start && offset <= end) return alert;
        }
        return null;
    }

    // The heading at or before the offset; -1 before the first.
    public int SectionAt(int offset)
    {
        var index = -1;
        for (var i = 0; i < Headings.Count; i++)
        {
            if (Headings[i].Offset <= offset) index = i; else break;
        }
        return index;
    }

    // What a rewrite does with the text box. The same words are never set
    // again, whatever else changed (an alert check's time, an alert's
    // details behind the same line): setting them drops a selection, which
    // NVDA announces as "unselected". A rewrite the user did not ask for
    // waits while text is selected; the clock's tick applies it once the
    // selection is gone.
    public static RewritePlan Plan(SectionLayout shown, SectionLayout next, bool selecting, bool automatic) =>
        next.Text == shown.Text ? RewritePlan.KeepText
        : automatic && selecting ? RewritePlan.Wait
        : RewritePlan.Replace;

    // Where a caret in an earlier layout belongs in this one: the same
    // distance into the section with the same heading; the section at the
    // same position when that heading has gone (a day rolled over); the
    // heading itself when the section is now too short for the distance.
    public int MapCaret(SectionLayout from, int caret)
    {
        if (Headings.Count == 0) return Math.Min(Math.Max(caret, 0), Text.Length);
        var i = from.SectionAt(caret);
        if (i < 0) return Math.Min(caret, Text.Length);
        var j = -1;
        for (var k = 0; k < Headings.Count; k++)
        {
            if (Headings[k].Heading == from.Headings[i].Heading) { j = k; break; }
        }
        if (j < 0) j = Math.Min(i, Headings.Count - 1);
        var start = Headings[j].Offset;
        var end = j + 1 < Headings.Count ? Headings[j + 1].Offset : Text.Length;
        var distance = caret - from.Headings[i].Offset;
        return distance < end - start ? start + distance : start;
    }
}

// KeepText: take the new layout (its alert lines may point at newer
// details) and leave the text box alone. Wait: leave both until later.
internal enum RewritePlan
{
    KeepText,
    Wait,
    Replace,
}
