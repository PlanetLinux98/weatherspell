using System.Net.Http;
using System.Runtime.Serialization;
using Weatherspell.Weather.EnvironmentCanada;
using Weatherspell.Weather.Nws;

namespace Weatherspell.Weather.Alerts;

// Alerts in effect for a location, from the service that covers its country
// (in 0.1.0 Environment Canada and the NWS), most severe first. A location
// outside every covered region gets NotAvailable, which the text states
// outright; a source that cannot be reached gets a report with the problem
// named, never an empty list that would read as a quiet day.
internal sealed class AlertService
{
    private readonly NwsAlertsClient _nws = new();
    private readonly AlertsClient _canada = new();

    public async Task<AlertReport> GetAsync(Location location, DateTimeOffset now, CancellationToken cancellationToken)
    {
        (Task<IReadOnlyList<WeatherAlert>> Fetch, string Attribution)? source = location.Country switch
        {
            "Canada" => (_canada.GetAsync(location, now, cancellationToken), AlertsClient.Attribution),
            "United States" => (_nws.GetAsync(location, cancellationToken), NwsAlertsClient.Attribution),
            _ => null,
        };
        if (source is null) return AlertReport.NotAvailable;
        var (fetch, attribution) = source.Value;
        try
        {
            return new AlertReport(Order(await fetch.ConfigureAwait(false)), attribution, null);
        }
        catch (Exception ex) when (ex is HttpRequestException or IOException or InvalidDataException or SerializationException or FormatException
                                   || (ex is OperationCanceledException && !cancellationToken.IsCancellationRequested))
        {
            // HttpClient reports its timeout as a cancellation; the token
            // says whether this one was the caller's.
            return new AlertReport([], attribution, ex is OperationCanceledException ? "the alert service took too long to answer" : ex.Message);
        }
    }

    // Severity first; within it warnings before watches before advisories
    // and statements; then whichever ends soonest.
    public static IReadOnlyList<WeatherAlert> Order(IEnumerable<WeatherAlert> alerts) =>
        alerts
            .OrderByDescending(a => a.Severity)
            .ThenBy(a => a.Kind)
            .ThenBy(a => a.Ends ?? DateTimeOffset.MaxValue)
            .ThenBy(a => a.Event, StringComparer.Ordinal)
            .ToList();
}
