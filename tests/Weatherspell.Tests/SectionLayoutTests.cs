using Weatherspell.Weather;
using Weatherspell.Weather.Alerts;
using Xunit;

namespace Weatherspell.Tests;

public class SectionLayoutTests
{
    private static readonly WeatherAlert Frost = new("ec:1", "Frost advisory", AlertSeverity.Moderate, new DateTimeOffset(2026, 9, 12, 22, 35, 0, TimeSpan.FromHours(-2.5)), null, null, "Environment Canada", "Environment Canada", "Gander and vicinity", null, "Areas of frost are expected.", null, "https://weather.gc.ca/");

    private static IReadOnlyList<Section> Sections(bool alert = false, bool restOfToday = true) =>
    [
        alert
            ? new Section("Alerts", ["Frost advisory from Environment Canada, in effect until 6:30 am tomorrow. Press Enter for details."], [Frost])
            : new Section("Alerts", ["No alerts in effect."]),
        new Section("Right now", ["As of 2:30 pm, it's 21 degrees.", "Humidity 52 percent."]),
        .. restOfToday ? new[] { new Section("Rest of today", ["This evening: clear."]) } : [],
        new Section("Saturday, September 12", ["Saturday: sunny.", "Saturday night: clear."]),
        new Section("Sources", ["Open-Meteo."]),
    ];

    [Fact]
    public void Text_has_a_steady_rhythm_and_headings_are_located()
    {
        var layout = SectionLayout.Build(Sections());

        Assert.StartsWith("Alerts\r\n\r\nNo alerts in effect.\r\n\r\n\r\nRight now\r\n\r\nAs of 2:30 pm, it's 21 degrees.\r\n\r\nHumidity 52 percent.\r\n\r\n\r\nRest of today", layout.Text);
        Assert.Equal(["Alerts", "Right now", "Rest of today", "Saturday, September 12", "Sources"], layout.Headings.Select(h => h.Heading).ToArray());
        Assert.Equal(0, layout.Headings[0].Offset);
        Assert.Equal("Right now", layout.Text.Substring(layout.Headings[1].Offset, 9));
        Assert.Empty(layout.AlertRanges);
    }

    [Fact]
    public void Alert_lines_are_found_by_offset()
    {
        var layout = SectionLayout.Build(Sections(alert: true));
        var (start, end, alert) = Assert.Single(layout.AlertRanges);

        Assert.Same(Frost, alert);
        Assert.Same(Frost, layout.AlertAt(start));
        Assert.Same(Frost, layout.AlertAt(end));
        Assert.Null(layout.AlertAt(end + 1));
        Assert.Null(layout.AlertAt(0));
    }

    // A rewrite keeps the reader on the same words: the same distance into
    // the section with the same heading, however much the text above grew.
    [Fact]
    public void The_caret_follows_its_heading_across_a_rewrite()
    {
        var before = SectionLayout.Build(Sections());
        var after = SectionLayout.Build(Sections(alert: true));
        var humidity = before.Text.IndexOf("Humidity", StringComparison.Ordinal);

        var mapped = after.MapCaret(before, humidity);

        Assert.Equal("Humidity", after.Text.Substring(mapped, 8));
        Assert.NotEqual(humidity, mapped);
    }

    [Fact]
    public void A_vanished_section_maps_to_the_one_at_its_position_and_a_short_one_to_its_heading()
    {
        var before = SectionLayout.Build(Sections());
        var after = SectionLayout.Build(Sections(restOfToday: false));
        var evening = before.Text.IndexOf("This evening", StringComparison.Ordinal);

        // Rest of today is gone; the caret lands the same distance into the
        // section now third, Saturday.
        var mapped = after.MapCaret(before, evening);
        Assert.Equal(2, after.SectionAt(mapped));
        Assert.Equal(evening - before.Headings[2].Offset, mapped - after.Headings[2].Offset);

        // Same heading, but the section is now too short for the distance: the heading itself.
        var shorter = SectionLayout.Build([new Section("Right now", ["Short."])]);
        var deep = before.Text.IndexOf("Humidity", StringComparison.Ordinal);
        Assert.Equal(shorter.Headings[0].Offset, shorter.MapCaret(before, deep));
    }

    [Fact]
    public void An_empty_layout_keeps_the_caret_in_range()
    {
        var before = SectionLayout.Build(Sections());
        Assert.Equal(0, SectionLayout.Empty.MapCaret(before, 40));
        Assert.Equal(0, SectionLayout.Empty.Headings.Count);
    }
}
