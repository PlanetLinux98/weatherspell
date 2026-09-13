namespace Weatherspell.Weather.Alerts;

// Which of a location's alerts are new since the last check, given the ids
// seen before. The seen list is replaced with the current ids, so an alert
// that ends and is later issued afresh under a new id counts as new again,
// and the list never grows past what is in effect. Callers keep the list
// per saved location and persist it, so a relaunch during a long-running
// alert does not announce it a second time.
internal static class AlertTracker
{
    public static IReadOnlyList<WeatherAlert> Update(List<string> seen, IReadOnlyList<WeatherAlert> current)
    {
        var fresh = current.Where(a => !seen.Contains(a.Id)).ToList();
        seen.Clear();
        seen.AddRange(current.Select(a => a.Id).Distinct());
        return fresh;
    }
}
