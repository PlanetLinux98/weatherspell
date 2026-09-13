using System.Globalization;
using System.Text;
using Weatherspell.Settings;
using Weatherspell.Weather;
using Weatherspell.Weather.Alerts;
using Weatherspell.Weather.EnvironmentCanada;
using Weatherspell.Weather.Nws;
using Weatherspell.Weather.OpenMeteo;

namespace Weatherspell;

// The UI is hand-coded rather than designer-generated: every control's
// accessible name, tab order and label association is visible in one place
// and reviewable in a diff (see NOTES.md).
internal sealed class MainForm : Form
{
    private const int ForecastDays = 7;

    private readonly SettingsStore _store;
    private readonly OpenMeteoClient _client = new();
    private readonly ForecastService _forecasts;
    private readonly AlertService _alerts = new();
    private AppSettings _settings;

    private readonly ComboBox _locations;
    private readonly TextBox _forecast;
    private readonly ToolStripStatusLabel _status;
    private readonly List<int> _headingOffsets = [];
    private readonly List<(int Start, int End, WeatherAlert Alert)> _alertRanges = [];
    private CancellationTokenSource? _refreshing;
    private bool _suppressLocationEvents;

    // What the text box shows, kept so the Alerts section can be rewritten
    // when a poll finds the on-screen location's alerts changed.
    private Forecast? _shown;
    private AlertReport? _shownAlerts;
    private readonly System.Windows.Forms.Timer _alertTimer = new();
    private readonly CancellationTokenSource _closing = new();
    private bool _polling;

    public MainForm() : this(SettingsStore.Default())
    {
    }

    public MainForm(SettingsStore store)
    {
        _store = store;
        _settings = store.Load();
        _forecasts = new ForecastService(_client);

        Text = "Weatherspell";
        // The exe's embedded icon, so the title bar and Alt+Tab match Explorer
        // without shipping a loose .ico.
        Icon = Icon.ExtractAssociatedIcon(Application.ExecutablePath);
        StartPosition = FormStartPosition.CenterScreen;
        // Sizes before Scaling.Apply, or they are never scaled (see Scaling).
        MinimumSize = new Size(480, 360);
        Size = new Size(720, 560);
        Scaling.Apply(this);

        var menu = new MenuStrip { TabIndex = 0 };
        var file = new ToolStripMenuItem("&File");
        file.DropDownItems.Add(new ToolStripMenuItem("&Refresh", null, async (_, _) => await RefreshAsync(keepCaret: true)) { ShortcutKeys = Keys.F5 });
        file.DropDownItems.Add(new ToolStripSeparator());
        // Alt+F4 already closes the window; the display string only advertises it.
        file.DropDownItems.Add(new ToolStripMenuItem("E&xit", null, (_, _) => Close()) { ShortcutKeyDisplayString = "Alt+F4" });
        var locations = new ToolStripMenuItem("&Locations");
        locations.DropDownItems.Add(new ToolStripMenuItem("&Find Location...", null, (_, _) => FindLocation()) { ShortcutKeys = Keys.Control | Keys.L });
        var view = new ToolStripMenuItem("&View");
        view.DropDownItems.Add(new ToolStripMenuItem("&Next Section", null, (_, _) => JumpSection(+1)) { ShortcutKeys = Keys.Control | Keys.PageDown });
        view.DropDownItems.Add(new ToolStripMenuItem("&Previous Section", null, (_, _) => JumpSection(-1)) { ShortcutKeys = Keys.Control | Keys.PageUp });
        view.DropDownItems.Add(new ToolStripSeparator());
        view.DropDownItems.Add(new ToolStripMenuItem("&Alerts", null, (_, _) => JumpToAlerts()) { ShortcutKeys = Keys.Control | Keys.Shift | Keys.A });
        var help = new ToolStripMenuItem("&Help");
        help.DropDownItems.Add(new ToolStripMenuItem("&About Weatherspell", null, (_, _) => ShowAbout()));
        menu.Items.AddRange([file, locations, view, help]);
        MainMenuStrip = menu;

        // Header row: the label is the combo box's visible name; AccessibleName
        // repeats it verbatim so the announced name never drifts from the screen.
        var header = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            ColumnCount = 2,
            Padding = new Padding(8, 8, 8, 0),
            TabIndex = 1,
        };
        header.ColumnStyles.Add(new ColumnStyle(SizeType.AutoSize));
        header.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100));
        header.RowStyles.Add(new RowStyle(SizeType.Percent, 100));
        // Alt+O: Alt+L belongs to the Locations menu.
        var locationLabel = new Label { Text = "L&ocation", AutoSize = true, Anchor = AnchorStyles.Left, TabIndex = 0 };
        _locations = new ComboBox
        {
            AccessibleName = "Location",
            DropDownStyle = ComboBoxStyle.DropDownList,
            Anchor = AnchorStyles.Left | AnchorStyles.Right,
            TabIndex = 1,
        };
        header.Controls.Add(locationLabel, 0, 0);
        header.Controls.Add(_locations, 1, 0);
        // A combo box takes its height from the font only once its handle
        // exists, after an auto-sized row has already measured it; at large
        // text sizes the row came out short and clipped the text. So the
        // header is sized from the combo's real height, whenever that changes.
        void FitHeader() => header.Height = header.Padding.Vertical + _locations.Margin.Vertical + _locations.Height;
        _locations.HandleCreated += (_, _) => FitHeader();
        _locations.SizeChanged += (_, _) => FitHeader();
        FitHeader();

        var forecastLabel = new Label
        {
            Text = "Forecast",
            AutoSize = true,
            Dock = DockStyle.Top,
            Padding = new Padding(8, 8, 8, 2),
            TabIndex = 2,
        };
        _forecast = new TextBox
        {
            AccessibleName = "Forecast",
            Multiline = true,
            ReadOnly = true,
            ScrollBars = ScrollBars.Vertical,
            Dock = DockStyle.Fill,
            TabIndex = 3,
            WordWrap = true,
        };

        var statusBar = new StatusStrip { TabIndex = 4 };
        // Spring: a status label wider than the strip is not clipped by
        // WinForms, it vanishes. Filling the strip truncates with an ellipsis
        // instead, and the full text stays available to a screen reader.
        _status = new ToolStripStatusLabel("Ready")
        {
            Spring = true,
            TextAlign = ContentAlignment.MiddleLeft,
        };
        statusBar.Items.Add(_status);

        // Fill first: WinForms docks the last-added control first, so the
        // filling control must sit at index 0 to take what the others leave.
        Controls.Add(_forecast);
        Controls.Add(forecastLabel);
        Controls.Add(header);
        Controls.Add(statusBar);
        Controls.Add(menu);

        _locations.SelectedIndexChanged += async (_, _) => await OnLocationChangedAsync();
        _forecast.KeyDown += OnForecastKeyDown;
        Shown += async (_, _) => await OnShownAsync();
        FormClosed += (_, _) =>
        {
            _alertTimer.Stop();
            _refreshing?.Cancel();
            _closing.Cancel();
        };

        // Every saved location's alerts, not just the one on screen, so a
        // warning for home is spoken while reading somewhere else.
        _alertTimer.Interval = _settings.AlertCheckMinutes * 60_000;
        _alertTimer.Tick += async (_, _) => await PollAlertsAsync();

        PopulateLocations();
    }

    private async Task OnShownAsync()
    {
        _forecast.Focus();
        if (_store.LoadProblem is string problem)
        {
            _status.Text = problem;
        }
        if (_settings.Locations.Count == 0)
        {
            SetText([new Section("Welcome", ["No location yet. Press Ctrl+L, or use Locations > Find Location, to add one."])]);
            FindLocation();
            return;
        }
        await RefreshAsync(keepCaret: false);
        // The refresh checked the location on screen; the others now, then
        // all of them on the timer.
        await PollAlertsAsync(includeCurrent: false);
        _alertTimer.Start();
    }

    private void PopulateLocations()
    {
        _suppressLocationEvents = true;
        _locations.BeginUpdate();
        _locations.Items.Clear();
        foreach (var saved in _settings.Locations)
        {
            _locations.Items.Add(saved.ToLocation().DisplayName);
        }
        _locations.EndUpdate();
        if (_locations.Items.Count > 0)
        {
            _locations.SelectedIndex = Math.Min(_settings.LastLocation, _locations.Items.Count - 1);
        }
        _suppressLocationEvents = false;
    }

    private SavedLocation? CurrentSaved =>
        _locations.SelectedIndex >= 0 && _locations.SelectedIndex < _settings.Locations.Count
            ? _settings.Locations[_locations.SelectedIndex]
            : null;

    private Location? CurrentLocation => CurrentSaved?.ToLocation();

    private async Task OnLocationChangedAsync()
    {
        if (_suppressLocationEvents || _locations.SelectedIndex < 0) return;
        _settings.LastLocation = _locations.SelectedIndex;
        TrySave();
        await RefreshAsync(keepCaret: false);
    }

    private void FindLocation()
    {
        using var dialog = new FindLocationDialog(new LocationSearch(_client));
        if (dialog.ShowDialog(this) != DialogResult.OK || dialog.Chosen is null) return;

        _settings.Locations.Add(SavedLocation.From(dialog.Chosen));
        _settings.LastLocation = _settings.Locations.Count - 1;
        TrySave();
        PopulateLocations();
        // PopulateLocations selects the new entry silently; fetch it now.
        _ = RefreshAsync(keepCaret: false);
        _forecast.Focus();
    }

    private async Task RefreshAsync(bool keepCaret)
    {
        var saved = CurrentSaved;
        var location = saved?.ToLocation();
        if (saved is null || location is null) return;

        _refreshing?.Cancel();
        _refreshing = new CancellationTokenSource();
        var token = _refreshing.Token;
        var caret = keepCaret ? _forecast.SelectionStart : 0;

        _status.Text = $"Fetching the forecast for {location.DisplayName}...";
        try
        {
            var forecastTask = _forecasts.GetAsync(location, Units.FromWindowsRegion(), ForecastDays, token);
            var alertsTask = _alerts.GetAsync(location, DateTimeOffset.UtcNow, token);
            var forecast = await forecastTask;
            var alerts = await alertsTask;
            if (token.IsCancellationRequested) return;
            saved.UtcOffsetSeconds = (int)forecast.UtcOffset.TotalSeconds;
            Show(forecast, alerts, caret);
            _status.Text = $"{location.DisplayName}: updated {Clock.PcTime(DateTime.Now)} from {forecast.SourceName}";
            if (alerts.Checked)
            {
                var fresh = AlertTracker.Update(saved.SeenAlertIds, alerts.Alerts);
                // F5 keeps the caret, so an alert that has appeared above it
                // is spoken; a location shown from the top is read from its
                // Alerts line anyway.
                if (keepCaret && saved.NotifyAlerts) Announce(saved, fresh);
            }
            TrySave();
        }
        catch (OperationCanceledException)
        {
        }
        catch (Exception ex) when (ex is System.Net.Http.HttpRequestException or IOException or InvalidDataException or System.Runtime.Serialization.SerializationException or FormatException)
        {
            SetText([new Section("Problem", [$"Couldn't fetch the forecast for {location.DisplayName}: {ex.Message}", "Press F5 to try again."])]);
            _status.Text = $"Couldn't fetch the forecast for {location.DisplayName}.";
        }
    }

    private void Show(Forecast forecast, AlertReport alerts, int caret)
    {
        _shown = forecast;
        _shownAlerts = alerts;
        SetText(ForecastWriter.Write(forecast, WriterOptions.Default(), alerts), caret);
    }

    // Heading line, blank line, paragraphs separated by blank lines, blank
    // line: a steady rhythm for line-by-line reading, and a heading offset
    // list for Ctrl+PageDown/PageUp.
    private void SetText(IReadOnlyList<Section> sections, int caret = 0)
    {
        var sb = new StringBuilder();
        _headingOffsets.Clear();
        _alertRanges.Clear();
        foreach (var section in sections)
        {
            if (sb.Length > 0) sb.Append("\r\n");
            _headingOffsets.Add(sb.Length);
            sb.Append(section.Heading).Append("\r\n\r\n");
            for (var i = 0; i < section.Paragraphs.Count; i++)
            {
                var paragraph = section.Paragraphs[i];
                if (section.Alerts is not null && i < section.Alerts.Count)
                {
                    _alertRanges.Add((sb.Length, sb.Length + paragraph.Length, section.Alerts[i]));
                }
                sb.Append(paragraph).Append("\r\n\r\n");
            }
        }
        _forecast.Text = sb.ToString();
        _forecast.SelectionStart = Math.Min(Math.Max(caret, 0), _forecast.TextLength);
        _forecast.SelectionLength = 0;
        _forecast.ScrollToCaret();
    }

    private WeatherAlert? AlertAtCaret()
    {
        var here = _forecast.SelectionStart;
        foreach (var (start, end, alert) in _alertRanges)
        {
            if (here >= start && here <= end) return alert;
        }
        return null;
    }

    private void ShowAlertDetails(WeatherAlert alert)
    {
        if (_shown is null) return;
        var o = WriterOptions.Default();
        var clock = new Clock(_shown.UtcOffset, o.PcZone, o.TimePattern, o.Culture);
        var nowLocal = o.Now.ToOffset(_shown.UtcOffset).DateTime;
        using var dialog = new AlertDialog(alert.Event, AlertWriter.Details(alert, clock, nowLocal), alert.Url);
        dialog.ShowDialog(this);
    }

    // Every saved location that asks to be notified, plus the one on
    // screen so its Alerts section stays current. A source that cannot be
    // reached leaves that location's seen list alone, so nothing is
    // announced twice after an outage. Never moves focus or the caret.
    private async Task PollAlertsAsync(bool includeCurrent = true)
    {
        if (_polling) return;
        _polling = true;
        try
        {
            var changed = false;
            foreach (var saved in _settings.Locations.ToList())
            {
                var isCurrent = ReferenceEquals(saved, CurrentSaved);
                if (isCurrent ? !includeCurrent : !saved.NotifyAlerts) continue;
                var location = saved.ToLocation();
                var report = await _alerts.GetAsync(location, DateTimeOffset.UtcNow, _closing.Token);
                if (!report.Checked || !_settings.Locations.Contains(saved)) continue;
                var before = saved.SeenAlertIds.ToList();
                var fresh = AlertTracker.Update(saved.SeenAlertIds, report.Alerts);
                if (!before.SequenceEqual(saved.SeenAlertIds)) changed = true;
                if (saved.NotifyAlerts) Announce(saved, fresh);
                // Only if the text on screen is still this location's: the
                // user may have switched while the check was in flight.
                if (ReferenceEquals(saved, CurrentSaved) && _shown?.Location == location && _shownAlerts is not null && Signature(_shownAlerts) != Signature(report))
                {
                    RewriteAlerts(report);
                }
            }
            if (changed) TrySave();
        }
        catch (OperationCanceledException)
        {
        }
        finally
        {
            _polling = false;
        }
    }

    private static string Signature(AlertReport r) =>
        r.Problem ?? string.Join("|", r.Alerts.Select(a => $"{a.Id}@{a.Issued:o}-{a.Ends:o}"));

    // The Alerts section is the first, so a caret further down moves by as
    // much as the section grew or shrank and stays on the same words.
    private void RewriteAlerts(AlertReport report)
    {
        var caret = _forecast.SelectionStart;
        var before = _headingOffsets.Count > 1 ? _headingOffsets[1] : 0;
        Show(_shown!, report, caret);
        var after = _headingOffsets.Count > 1 ? _headingOffsets[1] : 0;
        if (caret >= before)
        {
            _forecast.SelectionStart = Math.Min(caret + (after - before), _forecast.TextLength);
            _forecast.SelectionLength = 0;
            _forecast.ScrollToCaret();
        }
    }

    private void Announce(SavedLocation saved, IReadOnlyList<WeatherAlert> fresh)
    {
        var spoken = fresh.Where(a => _settings.Announces(a.Severity)).ToList();
        if (spoken.Count == 0) return;
        // The location's own time, from its last forecast; a location whose
        // forecast never loaded is announced in this PC's time.
        var offset = saved.UtcOffsetSeconds is int seconds ? TimeSpan.FromSeconds(seconds) : TimeZoneInfo.Local.GetUtcOffset(DateTime.Now);
        var clock = new Clock(offset, TimeZoneInfo.Local, Clock.WindowsTimePattern(), CultureInfo.CurrentCulture);
        var nowLocal = DateTimeOffset.UtcNow.ToOffset(offset).DateTime;
        Announcer.Alert(this, AlertWriter.Announcement(saved.ToLocation().DisplayName, spoken, clock, nowLocal));
    }

    private void JumpSection(int direction)
    {
        if (_headingOffsets.Count == 0) return;
        var here = _forecast.SelectionStart;
        // No FirstOrDefault(pred, fallback) on 4.8: -1 stands in for "none".
        var target = -1;
        if (direction > 0)
        {
            foreach (var o in _headingOffsets) { if (o > here) { target = o; break; } }
        }
        else
        {
            foreach (var o in _headingOffsets) { if (o < here) target = o; else break; }
        }
        if (target < 0) return;
        MoveCaret(target);
    }

    private void JumpToAlerts()
    {
        if (_headingOffsets.Count > 0) MoveCaret(_headingOffsets[0]);
    }

    private void MoveCaret(int offset)
    {
        _forecast.Focus();
        _forecast.SelectionStart = offset;
        _forecast.SelectionLength = 0;
        _forecast.ScrollToCaret();
    }

    private void OnForecastKeyDown(object? sender, KeyEventArgs e)
    {
        if (e.Control && e.KeyCode == Keys.PageDown)
        {
            JumpSection(+1);
            e.Handled = true;
            e.SuppressKeyPress = true;
        }
        else if (e.Control && e.KeyCode == Keys.PageUp)
        {
            JumpSection(-1);
            e.Handled = true;
            e.SuppressKeyPress = true;
        }
        else if (e.KeyCode == Keys.Enter && e.Modifiers == Keys.None && AlertAtCaret() is WeatherAlert alert)
        {
            e.Handled = true;
            e.SuppressKeyPress = true;
            ShowAlertDetails(alert);
        }
    }

    private void TrySave()
    {
        try
        {
            _store.Save(_settings);
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            _status.Text = $"Couldn't save settings: {ex.Message}";
        }
    }

    private void ShowAbout()
    {
        MessageBox.Show(this,
            $"Weatherspell {AppVersion.Display}\nA text-based weather app for Windows.\n\nForecast data: {OpenMeteoClient.SourceNote}.\nForecast text, current conditions and alerts: {CityPageClient.SourceName} (weather.gc.ca) in Canada, {NwsClient.SourceName} (weather.gov) in the United States.\nPostal codes for Canada, the UK, Australia, New Zealand and Ireland: {PostalCodes.SourceNote}.",
            "About Weatherspell", MessageBoxButtons.OK, MessageBoxIcon.Information);
    }
}
