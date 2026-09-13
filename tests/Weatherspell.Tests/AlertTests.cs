using System.Globalization;
using Weatherspell.Weather;
using Weatherspell.Weather.Alerts;
using Weatherspell.Weather.EnvironmentCanada;
using Weatherspell.Weather.Nws;
using Xunit;

namespace Weatherspell.Tests;

public class NwsAlertParsingTests
{
    [Fact]
    public void The_zone_and_county_copies_of_one_product_become_one_alert()
    {
        var alerts = NwsAlertsClient.Parse(Fixtures.Read("nws-alerts-spokane.json"));

        var a = Assert.Single(alerts);
        Assert.Equal("nws:KOTX.FF.A.0001.2026", a.Id);
        Assert.Equal("Flash flood watch", a.Event);
        Assert.Equal(AlertSeverity.Severe, a.Severity);
        Assert.Equal(AlertKind.Watch, a.Kind);
        Assert.Equal(new DateTimeOffset(2026, 9, 12, 19, 41, 0, TimeSpan.FromHours(-7)), a.Issued);
        Assert.Equal(new DateTimeOffset(2026, 9, 13, 2, 0, 0, TimeSpan.FromHours(-7)), a.Onset);
        Assert.Equal(new DateTimeOffset(2026, 9, 13, 16, 0, 0, TimeSpan.FromHours(-7)), a.Ends);
        Assert.Equal("the National Weather Service", a.Source);
        Assert.Equal("NWS Spokane WA", a.Sender);
        Assert.Equal("Spokane Area", a.Area);
        Assert.Null(a.Level);
        Assert.StartsWith("* WHAT...Flash flooding", a.Description);
        Assert.StartsWith("You should monitor later forecasts", a.Instruction);
        Assert.Equal("https://forecast.weather.gov/wwamap/wwatxtget.php?cwa=OTX&wwa=flash%20flood%20watch", a.Url);
    }

    [Fact]
    public void The_event_end_is_preferred_to_the_message_expiry()
    {
        // The statement is reissued at 7 am but the hazard lasts until 11 pm.
        var a = Assert.Single(NwsAlertsClient.Parse(Fixtures.Read("nws-alerts-muskegon.json")));

        Assert.Equal("Beach hazards statement", a.Event);
        Assert.Equal(AlertKind.Statement, a.Kind);
        Assert.Equal(AlertSeverity.Moderate, a.Severity);
        Assert.Equal(new DateTimeOffset(2026, 9, 13, 23, 0, 0, TimeSpan.FromHours(-4)), a.Ends);
    }

    [Fact]
    public void An_update_keeps_the_identity_of_the_event_it_continues()
    {
        var a = Assert.Single(NwsAlertsClient.Parse(Fixtures.Read("nws-alerts-pawhuska.json")));

        Assert.Equal("nws:KTSA.HT.Y.0058.2026", a.Id);
        Assert.Equal("Heat advisory", a.Event);
        Assert.Equal("NWS Tulsa OK", a.Sender);
    }

    [Fact]
    public void A_point_with_nothing_in_effect_gives_an_empty_list()
    {
        Assert.Empty(NwsAlertsClient.Parse(Fixtures.Read("nws-alerts-albany.json")));
    }
}

public class EnvironmentCanadaAlertTests
{
    private static readonly DateTimeOffset Now = new(2026, 9, 13, 4, 0, 0, TimeSpan.Zero);

    [Fact]
    public void An_alert_at_a_point_carries_its_text_level_and_times()
    {
        var alerts = AlertsClient.Parse(Fixtures.Read("ec-alerts-gander.json"), Now, "https://weather.gc.ca/en/location/index.html?coords=48.96,-54.61");

        var a = Assert.Single(alerts);
        Assert.Equal("ec:64919237566271632202609120507", a.Id);
        Assert.Equal("Frost advisory", a.Event);
        Assert.Equal(AlertKind.Advisory, a.Kind);
        // Yellow outranks what an advisory would be on its own.
        Assert.Equal(AlertSeverity.Moderate, a.Severity);
        Assert.Equal(new DateTimeOffset(2026, 9, 13, 1, 5, 46, 284, TimeSpan.Zero), a.Issued);
        Assert.Equal(new DateTimeOffset(2026, 9, 13, 0, 55, 0, TimeSpan.Zero), a.Onset);
        // The event's end, not the message's expiry an hour later.
        Assert.Equal(new DateTimeOffset(2026, 9, 13, 9, 0, 0, TimeSpan.Zero), a.Ends);
        Assert.Equal("Environment Canada", a.Source);
        Assert.Equal("Environment Canada", a.Sender);
        Assert.Equal("Gander and vicinity", a.Area);
        Assert.Equal("Yellow level, moderate impact, high confidence", a.Level);
        Assert.StartsWith("Areas of frost are expected.", a.Description);
        Assert.Null(a.Instruction);
        Assert.Equal("https://weather.gc.ca/en/location/index.html?coords=48.96,-54.61", a.Url);
    }

    [Fact]
    public void Continued_warnings_are_kept_and_told_apart()
    {
        var alerts = AlertsClient.Parse(Fixtures.Read("ec-alerts-warnings.json"), new DateTimeOffset(2026, 9, 13, 4, 0, 0, TimeSpan.Zero), null);

        Assert.Equal(2, alerts.Count);
        Assert.All(alerts, a => Assert.Equal("Storm surge warning", a.Event));
        // A yellow warning is still a warning: Severe, not Moderate.
        Assert.All(alerts, a => Assert.Equal(AlertSeverity.Severe, a.Severity));
        Assert.NotEqual(alerts[0].Id, alerts[1].Id);
        Assert.Equal(new DateTimeOffset(2026, 9, 15, 18, 0, 0, TimeSpan.Zero), alerts[1].Ends);
    }

    [Fact]
    public void A_stale_alert_is_dropped()
    {
        Assert.Empty(AlertsClient.Parse(Fixtures.Read("ec-alerts-gander.json"), new DateTimeOffset(2026, 9, 14, 0, 0, 0, TimeSpan.Zero), null));
    }

    [Fact]
    public void A_point_with_nothing_in_effect_gives_an_empty_list()
    {
        Assert.Empty(AlertsClient.Parse(Fixtures.Read("ec-alerts-peterborough.json"), Now, null));
    }

    // The internal enum cannot appear in a public test signature, hence the names.
    [Theory]
    [InlineData("red", "advisory", "Extreme")]
    [InlineData("orange", "watch", "Severe")]
    [InlineData("yellow", "warning", "Severe")]
    [InlineData("yellow", "statement", "Moderate")]
    [InlineData(null, "warning", "Severe")]
    [InlineData(null, "watch", "Moderate")]
    [InlineData(null, "advisory", "Minor")]
    [InlineData("", "statement", "Minor")]
    [InlineData(null, null, "Unknown")]
    public void Severity_is_the_colour_level_with_the_type_as_a_floor(string? colour, string? type, string expected)
    {
        Assert.Equal(expected, AlertsClient.Severity(colour, type).ToString());
    }
}

public class AlertOrderingAndTrackingTests
{
    private static WeatherAlert Alert(string id, string @event, AlertSeverity severity, int endsHour) =>
        new(id, @event, severity, new DateTimeOffset(2026, 9, 11, 12, 0, 0, TimeSpan.FromHours(-4)), null,
            new DateTimeOffset(2026, 9, 11, endsHour, 0, 0, TimeSpan.FromHours(-4)), "Environment Canada", "Environment Canada", "Somewhere", null, "Text.", null, null);

    [Fact]
    public void Most_severe_first_then_warnings_before_watches_then_soonest_to_end()
    {
        var ordered = AlertService.Order(
        [
            Alert("a", "Frost advisory", AlertSeverity.Minor, 8),
            Alert("b", "Severe thunderstorm watch", AlertSeverity.Severe, 21),
            Alert("c", "Rainfall warning", AlertSeverity.Severe, 23),
            Alert("d", "Tornado warning", AlertSeverity.Extreme, 15),
            Alert("e", "Severe thunderstorm warning", AlertSeverity.Severe, 18),
        ]);

        Assert.Equal(["d", "e", "c", "b", "a"], ordered.Select(a => a.Id).ToArray());
    }

    [Fact]
    public void Only_unseen_alerts_are_new_and_the_seen_list_follows_what_is_in_effect()
    {
        var seen = new List<string>();
        var a = Alert("a", "Rainfall warning", AlertSeverity.Severe, 18);
        var b = Alert("b", "Frost advisory", AlertSeverity.Minor, 8);
        var c = Alert("c", "Fog advisory", AlertSeverity.Minor, 9);

        Assert.Equal(["a", "b"], AlertTracker.Update(seen, [a, b]).Select(x => x.Id).ToArray());
        Assert.Equal(["a", "b"], seen);

        Assert.Equal(["c"], AlertTracker.Update(seen, [b, c]).Select(x => x.Id).ToArray());
        Assert.Equal(["b", "c"], seen);

        Assert.Empty(AlertTracker.Update(seen, []));
        Assert.Empty(seen);

        // Issued afresh after ending: new again.
        Assert.Equal(["b"], AlertTracker.Update(seen, [b]).Select(x => x.Id).ToArray());
    }
}

public class AlertWriterTests
{
    private static readonly TimeSpan Eastern = TimeSpan.FromHours(-4);
    // 2:45 pm on Friday 11 September 2026, location time.
    private static readonly DateTime NowLocal = new(2026, 9, 11, 14, 45, 0);

    private static Clock EasternClock(TimeSpan? pcOffset = null) =>
        new(Eastern, TimeZoneInfo.CreateCustomTimeZone("test-pc", pcOffset ?? Eastern, "Test", "Test"), "h:mm tt", CultureInfo.InvariantCulture);

    private static WeatherAlert Alert(string @event, DateTimeOffset? ends, AlertSeverity severity = AlertSeverity.Severe) =>
        new("id-" + @event, @event, severity, new DateTimeOffset(2026, 9, 11, 13, 10, 0, Eastern), null, ends,
            "Environment Canada", "Environment Canada", "Peterborough City - Lakefield - Southern Peterborough County", null, "Text.", null, null);

    [Fact]
    public void Lines_name_the_event_when_it_ends_and_the_source()
    {
        var clock = EasternClock();

        Assert.Equal("Severe thunderstorm warning until 6:00 pm today, from Environment Canada. Press Enter for details.",
            AlertWriter.Line(Alert("Severe thunderstorm warning", new DateTimeOffset(2026, 9, 11, 18, 0, 0, Eastern)), clock, NowLocal));
        Assert.Equal("Frost advisory until 6:30 am tomorrow, from Environment Canada. Press Enter for details.",
            AlertWriter.Line(Alert("Frost advisory", new DateTimeOffset(2026, 9, 12, 6, 30, 0, Eastern)), clock, NowLocal));
        Assert.Equal("Rainfall warning until 6:30 am Sunday, from Environment Canada. Press Enter for details.",
            AlertWriter.Line(Alert("Rainfall warning", new DateTimeOffset(2026, 9, 13, 6, 30, 0, Eastern)), clock, NowLocal));
        Assert.Equal("Storm surge warning until 6:30 am on September 20, from Environment Canada. Press Enter for details.",
            AlertWriter.Line(Alert("Storm surge warning", new DateTimeOffset(2026, 9, 20, 6, 30, 0, Eastern)), clock, NowLocal));
        Assert.Equal("Special weather statement from Environment Canada. Press Enter for details.",
            AlertWriter.Line(Alert("Special weather statement", null), clock, NowLocal));
    }

    [Fact]
    public void End_times_are_the_location_s_own_with_the_pc_time_in_brackets()
    {
        // The alert's own stamp is UTC, as Environment Canada sends them;
        // the PC is three hours behind the location.
        var line = AlertWriter.Line(Alert("Rainfall warning", new DateTimeOffset(2026, 9, 12, 2, 0, 0, TimeSpan.Zero)), EasternClock(TimeSpan.FromHours(-7)), NowLocal);

        Assert.Equal("Rainfall warning until 10:00 pm today (7:00 pm today your time), from Environment Canada. Press Enter for details.", line);
    }

    [Fact]
    public void The_section_states_a_gap_a_failure_or_a_quiet_day_outright()
    {
        var clock = EasternClock();

        var unsupported = AlertWriter.Section(AlertReport.NotAvailable, clock, NowLocal);
        Assert.Equal("Alerts", unsupported.Heading);
        Assert.Equal(["Alerts are not available for this region."], unsupported.Paragraphs);
        Assert.Null(unsupported.Alerts);

        Assert.Equal(["Alerts are not available for this region."], AlertWriter.Section(null, clock, NowLocal).Paragraphs);

        var failed = AlertWriter.Section(new AlertReport([], "Environment Canada (weather.gc.ca)", "503 Service Unavailable from api.weather.gc.ca"), clock, NowLocal);
        Assert.Equal(["Alerts couldn't be checked this time (503 Service Unavailable from api.weather.gc.ca). Press F5 to try again."], failed.Paragraphs);

        Assert.Equal(["No alerts in effect."], AlertWriter.Section(new AlertReport([], "Environment Canada (weather.gc.ca)", null), clock, NowLocal).Paragraphs);
    }

    [Fact]
    public void The_section_keeps_each_line_s_alert_for_the_enter_key()
    {
        var a = Alert("Tornado warning", new DateTimeOffset(2026, 9, 11, 15, 15, 0, Eastern), AlertSeverity.Extreme);
        var b = Alert("Severe thunderstorm watch", new DateTimeOffset(2026, 9, 11, 21, 0, 0, Eastern));

        var section = AlertWriter.Section(new AlertReport([a, b], "Environment Canada (weather.gc.ca)", null), EasternClock(), NowLocal);

        Assert.Equal(2, section.Paragraphs.Count);
        Assert.StartsWith("Tornado warning until 3:15 pm today", section.Paragraphs[0]);
        Assert.Equal([a, b], section.Alerts);
    }

    [Fact]
    public void Announcements_name_the_location_and_join_several_alerts()
    {
        var clock = EasternClock();
        var a = Alert("Tornado warning", new DateTimeOffset(2026, 9, 11, 18, 15, 0, Eastern), AlertSeverity.Extreme);
        var b = Alert("Severe thunderstorm watch", new DateTimeOffset(2026, 9, 11, 21, 0, 0, Eastern));
        var c = Alert("Special weather statement", null);

        Assert.Equal("Peterborough: tornado warning until 6:15 pm today.", AlertWriter.Announcement("Peterborough", [a], clock, NowLocal));
        Assert.Equal("Peterborough: tornado warning until 6:15 pm today and severe thunderstorm watch until 9:00 pm today.", AlertWriter.Announcement("Peterborough", [a, b], clock, NowLocal));
        Assert.Equal("Home: tornado warning until 6:15 pm today, severe thunderstorm watch until 9:00 pm today and special weather statement.", AlertWriter.Announcement("Home", [a, b, c], clock, NowLocal));
    }

    [Fact]
    public void Details_of_an_nws_alert_read_the_bullets_as_labelled_paragraphs()
    {
        var alert = Assert.Single(NwsAlertsClient.Parse(Fixtures.Read("nws-alerts-spokane.json")));
        var pacific = TimeSpan.FromHours(-7);
        var clock = new Clock(pacific, TimeZoneInfo.CreateCustomTimeZone("test-pc", pacific, "Test", "Test"), "h:mm tt", CultureInfo.InvariantCulture);

        var d = AlertWriter.Details(alert, clock, new DateTime(2026, 9, 12, 20, 0, 0));

        Assert.Equal("Flash flood watch from NWS Spokane WA, in effect from 2:00 am tomorrow until 4:00 pm tomorrow.", d[0]);
        Assert.Equal("Area: Spokane Area.", d[1]);
        Assert.Equal("Issued 7:41 pm today.", d[2]);
        Assert.Equal("What: Flash flooding and debris flows caused by excessive rainfall are possible over the burn scar.", d[3]);
        Assert.Equal("Where: A portion of Northeast Washington, including the following area and county, Spokane Area.", d[4]);
        Assert.Equal("When: From 2 AM PDT Sunday through Sunday afternoon.", d[5]);
        Assert.StartsWith("Impacts: Moderate to heavy rainfall over the burn scar is expected to develop Sunday morning. Rainfall rates", d[6]);
        Assert.Equal("Additional details:", d[7]);
        Assert.StartsWith("National Weather Service Meteorologists are forecasting", d[8]);
        Assert.Equal("Some locations that may experience flash flooding include... Spokane and Nine Mile Falls.", d[9]);
        Assert.Equal("http://www.weather.gov/safety/flood", d[10]);
        Assert.Equal("Instructions: You should monitor later forecasts and be prepared to take action should Flash Flood Warnings be issued.", d[11]);
        Assert.Equal(12, d.Count);
    }

    [Fact]
    public void Details_of_an_environment_canada_alert_carry_its_level_and_drop_the_rule()
    {
        var alert = Assert.Single(AlertsClient.Parse(Fixtures.Read("ec-alerts-gander.json"), new DateTimeOffset(2026, 9, 13, 1, 30, 0, TimeSpan.Zero), null));
        var newfoundland = new TimeSpan(-2, -30, 0);
        var clock = new Clock(newfoundland, TimeZoneInfo.CreateCustomTimeZone("test-pc", newfoundland, "Test", "Test"), "h:mm tt", CultureInfo.InvariantCulture);

        // 11 pm NDT on the 12th: the advisory was issued at 10:35 pm and
        // ends at 6:30 am tomorrow (09:00 UTC).
        var d = AlertWriter.Details(alert, clock, new DateTime(2026, 9, 12, 23, 0, 0));

        Assert.Equal("Frost advisory from Environment Canada, in effect until 6:30 am tomorrow.", d[0]);
        Assert.Equal("Area: Gander and vicinity.", d[1]);
        Assert.Equal("Issued 10:35 pm today.", d[2]);
        Assert.Equal("Yellow level, moderate impact, high confidence.", d[3]);
        Assert.Equal("Areas of frost are expected.", d[4]);
        Assert.StartsWith("Locations: Deer Lake - Humber Valley,", d[5]);
        Assert.Equal("Minimum temperatures: +5 to +1 (coolest in low-lying areas)", d[6]);
        Assert.Equal("Time span: early Sunday morning.", d[7]);
        Assert.StartsWith("Remarks: Patchy frost", d[8]);
        Assert.Equal("Damage to plants, trees, and crops is possible.", d[9]);
        Assert.StartsWith("Please continue to monitor alerts", d[10]);
        Assert.Equal(11, d.Count);
    }

    [Fact]
    public void Paragraphs_join_wrapped_lines_and_speak_symbols()
    {
        Assert.Equal(
            ["What: Winds of 90 percent strength.", "Dense fog will persist.", "Visibility near zero at times."],
            AlertWriter.Paragraphs("* WHAT...Winds of 90%\nstrength.\r\n\r\nDense fog will persist.\n\n###\n\nVisibility near zero\nat times."));
    }

    [Fact]
    public void The_forecast_text_leads_with_the_alerts_and_credits_their_source()
    {
        var f = OfficialForecastWriterTests.Base();
        var a = Alert("Rainfall warning", new DateTimeOffset(2026, 9, 11, 23, 0, 0, Eastern));
        var report = new AlertReport([a], "Environment Canada (weather.gc.ca)", null);
        var options = new WriterOptions(new DateTimeOffset(NowLocal, Eastern), TimeZoneInfo.CreateCustomTimeZone("test-eastern", Eastern, "Test Eastern", "Test Eastern"), "h:mm tt", CultureInfo.InvariantCulture);

        var sections = ForecastWriter.Write(f, options, report);

        Assert.Equal("Alerts", sections[0].Heading);
        Assert.Equal(["Rainfall warning until 11:00 pm today, from Environment Canada. Press Enter for details."], sections[0].Paragraphs);
        Assert.Equal([a], sections[0].Alerts);
        Assert.Equal("Alerts: Environment Canada (weather.gc.ca).", sections[sections.Count - 1].Paragraphs[sections[sections.Count - 1].Paragraphs.Count - 1]);

        var quiet = ForecastWriter.Write(f, options, AlertReport.NotAvailable);
        Assert.Equal(["Alerts are not available for this region."], quiet[0].Paragraphs);
        Assert.DoesNotContain(quiet[quiet.Count - 1].Paragraphs, p => p.StartsWith("Alerts:"));
    }
}
