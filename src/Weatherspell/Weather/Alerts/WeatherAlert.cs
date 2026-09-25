namespace Weatherspell.Weather.Alerts;

// The services' own scales, aligned: the NWS says Extreme, Severe, Moderate,
// Minor, Unknown; Environment Canada's colour levels and alert types map onto
// the same words (see AlertsClient). The announcement threshold setting and
// the ordering of the Alerts section both read this.
internal enum AlertSeverity { Unknown, Minor, Moderate, Severe, Extreme }

// From the last word of the event name, which both services use consistently
// ("Flash flood watch", "special weather statement"); an ordering tie-break
// within one severity.
internal enum AlertKind { Warning, Watch, Advisory, Statement, Other }

// One alert in effect for a location, source-neutral, as the Alerts section
// and the details dialog read it. Id is stable across a service's updates of
// the same alert, so each alert is announced once (each client says how).
// Times keep the offset the service gave them; the writer moves them to the
// location's own zone. Source is the phrase that follows "from" in a
// sentence ("Environment Canada", "the National Weather Service"); Sender
// is the issuing office as the service names it. Ends is when the hazard
// is expected to end, and only that: Expires is when this message runs
// out, which for a hurricane watch is when the next update is due, not
// when the watch ends, so it is never read as "until" (#20).
internal sealed record WeatherAlert(
    string Id,
    string Event,
    AlertSeverity Severity,
    DateTimeOffset Issued,
    DateTimeOffset? Onset,
    DateTimeOffset? Ends,
    string Source,
    string Sender,
    string Area,
    string? Level,
    string Description,
    string? Instruction,
    string? Url,
    DateTimeOffset? Expires = null)
{
    public AlertKind Kind
    {
        get
        {
            var words = Event.Split(' ');
            return words[words.Length - 1].ToLowerInvariant() switch
            {
                "warning" => AlertKind.Warning,
                "watch" => AlertKind.Watch,
                "advisory" => AlertKind.Advisory,
                "statement" => AlertKind.Statement,
                _ => AlertKind.Other,
            };
        }
    }
}

// What one check for a location produced. Attribution is null where no
// source covers the region; Problem is set when the source could not be
// reached or read, in which case Alerts is empty and must not be read as
// "none in effect", unless CheckedAt is also set: then Alerts are the last
// ones known, from a check at that time, kept so an outage does not hide a
// warning that may still be in effect (OrLastKnown).
internal sealed record AlertReport(IReadOnlyList<WeatherAlert> Alerts, string? Attribution, string? Problem, DateTimeOffset? CheckedAt = null)
{
    public static readonly AlertReport NotAvailable = new([], null, null);

    public bool IsAvailable => Attribution is not null;
    public bool Checked => Attribution is not null && Problem is null;

    // This check's failure carrying an earlier report's alerts, dated by
    // that report: what is shown while the service is unreachable. A check
    // that succeeded, a region no source covers, and an earlier report that
    // knew nothing all give this report unchanged. The writer drops alerts
    // whose end has passed when it reads a dated report.
    public AlertReport OrLastKnown(AlertReport? earlier) =>
        Checked || !IsAvailable || earlier?.CheckedAt is null
            ? this
            : this with { Alerts = earlier.Alerts, CheckedAt = earlier.CheckedAt };
}
