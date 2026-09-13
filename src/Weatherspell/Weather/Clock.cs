using System.Globalization;

namespace Weatherspell.Weather;

// Time words. Forecast times are the location's own; when this PC's clock
// differs, the PC time follows in brackets so a faraway sunrise still makes
// sense to the reader.
internal sealed class Clock
{
    private readonly TimeSpan _locationOffset;
    private readonly TimeZoneInfo _pcZone;
    private readonly string _timePattern;
    private readonly CultureInfo _culture;

    public Clock(TimeSpan locationOffset, TimeZoneInfo pcZone, string timePattern, CultureInfo culture)
    {
        _locationOffset = locationOffset;
        _pcZone = pcZone;
        _timePattern = timePattern;
        _culture = culture;
    }

    public static string WindowsTimePattern() =>
        CultureInfo.CurrentCulture.DateTimeFormat.ShortTimePattern;

    // This PC's clock in the same voice as the forecast text.
    public static string PcTime(DateTime local) =>
        Lowercase(local.ToString(WindowsTimePattern(), CultureInfo.CurrentCulture), CultureInfo.CurrentCulture);

    public string Time(DateTime local)
    {
        var text = Bare(local);
        var utc = ToUtc(local);
        var pcOffset = _pcZone.GetUtcOffset(utc);
        if (pcOffset == _locationOffset)
        {
            return text;
        }
        var pcLocal = utc + pcOffset;
        var suffix = pcLocal.Date == local.Date ? "" : (pcLocal.Date > local.Date ? " tomorrow" : " yesterday");
        return $"{text} ({Bare(pcLocal)}{suffix} your time)";
    }

    // "4:00 pm today", "6:30 am tomorrow", "6:30 am Sunday", "6:30 am on
    // September 20": when an alert ends, relative to the location's own
    // day, and in brackets relative to this PC's day when the zones differ.
    public string TimeOnDay(DateTime local, DateTime nowLocal)
    {
        var text = $"{Bare(local)} {DayWord(local.Date, nowLocal.Date)}";
        var utc = ToUtc(local);
        var pcOffset = _pcZone.GetUtcOffset(utc);
        if (pcOffset == _locationOffset)
        {
            return text;
        }
        var pcLocal = utc + pcOffset;
        var pcNow = ToUtc(nowLocal) + pcOffset;
        return $"{text} ({Bare(pcLocal)} {DayWord(pcLocal.Date, pcNow.Date)} your time)";
    }

    private string DayWord(DateTime date, DateTime today) =>
        (date - today).Days switch
        {
            0 => "today",
            1 => "tomorrow",
            -1 => "yesterday",
            > 1 and < 7 => date.ToString("dddd", _culture),
            _ => "on " + date.ToString("MMMM d", _culture),
        };

    // A service's timestamp as the location's own wall-clock time.
    public DateTime Local(DateTimeOffset time) => time.ToOffset(_locationOffset).DateTime;

    private DateTime ToUtc(DateTime local) =>
        new DateTimeOffset(DateTime.SpecifyKind(local, DateTimeKind.Unspecified), _locationOffset).UtcDateTime;

    // "2:45 pm": the designator lowercased so it reads as a word, not initials.
    private string Bare(DateTime local) =>
        Lowercase(local.ToString(_timePattern, _culture), _culture);

    private static string Lowercase(string s, CultureInfo culture)
    {
        var am = culture.DateTimeFormat.AMDesignator;
        var pm = culture.DateTimeFormat.PMDesignator;
        if (am.Length > 0) s = s.Replace(am, am.ToLower(culture));
        if (pm.Length > 0) s = s.Replace(pm, pm.ToLower(culture));
        return s;
    }

    public string DayHeading(DateTime date) =>
        date.ToString("dddd, MMMM d", _culture);

    public static string Duration(double seconds)
    {
        var total = (int)Math.Round(seconds / 60);
        var hours = total / 60;
        var minutes = total % 60;
        var h = hours == 1 ? "1 hour" : $"{hours} hours";
        var m = minutes == 1 ? "1 minute" : $"{minutes} minutes";
        if (hours == 0) return m;
        return minutes == 0 ? h : $"{h} and {m}";
    }

    public static string Age(TimeSpan age)
    {
        if (age < TimeSpan.FromMinutes(1)) return "just now";
        if (age < TimeSpan.FromHours(1))
        {
            var m = (int)age.TotalMinutes;
            return m == 1 ? "1 minute ago" : $"{m} minutes ago";
        }
        if (age < TimeSpan.FromDays(1))
        {
            var h = (int)age.TotalHours;
            return h == 1 ? "1 hour ago" : $"{h} hours ago";
        }
        var d = (int)age.TotalDays;
        return d == 1 ? "1 day ago" : $"{d} days ago";
    }
}
