using System.Globalization;
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

    private readonly NativeComboBox _locations;
    private readonly NativeTextBox _forecast;
    private readonly ToolStripStatusLabel _status;
    private SectionLayout _layout = SectionLayout.Empty;
    private CancellationTokenSource? _refreshing;
    private bool _suppressLocationEvents;

    // What the text box shows, kept so it can be rewritten in place: the
    // Alerts section when a poll finds them changed, the age line as the
    // clock moves, all of it when a refresh brings new data.
    private Forecast? _shown;
    private AlertReport? _shownAlerts;
    // Why the last refresh left older text on screen; null after a success.
    private string? _refreshProblem;
    private readonly System.Windows.Forms.Timer _forecastTimer = new();
    private readonly System.Windows.Forms.Timer _alertTimer = new();
    // Once a minute: the age line, and the day headings at midnight.
    private readonly System.Windows.Forms.Timer _clockTimer = new() { Interval = 60_000 };
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

        // The native Windows menu bar, not MenuStrip: on this runtime
        // MenuStrip tells a screen reader each item's mnemonic and never its
        // shortcut (see NOTES.md, #15). Ctrl+PageDown/PageUp are not in the
        // Shortcut enum, so they are written after a tab (the accelerator
        // column) and caught in ProcessCmdKey; Alt+F4 is the window's own.
        var file = new MenuItem("&File");
        file.MenuItems.Add(new MenuItem("&Refresh", async (_, _) => await RefreshAsync(keepCaret: true), Shortcut.F5));
        file.MenuItems.Add(new MenuItem("-"));
        file.MenuItems.Add(new MenuItem("E&xit\tAlt+F4", (_, _) => Close()));
        var locations = new MenuItem("&Locations");
        locations.MenuItems.Add(new MenuItem("&Find Location...", (_, _) => FindLocation(), Shortcut.CtrlL));
        var view = new MenuItem("&View");
        view.MenuItems.Add(new MenuItem("&Next Section\tCtrl+PageDown", (_, _) => JumpSection(+1)));
        view.MenuItems.Add(new MenuItem("&Previous Section\tCtrl+PageUp", (_, _) => JumpSection(-1)));
        view.MenuItems.Add(new MenuItem("-"));
        view.MenuItems.Add(new MenuItem("&Alerts", (_, _) => JumpToAlerts(), Shortcut.CtrlShiftA));
        var settings = new MenuItem("&Settings");
        // No accelerator: Windows has no conventional one for a settings
        // dialog (Ctrl+comma is macOS's), and Alt+S, S is two keys.
        settings.MenuItems.Add(new MenuItem("&Settings...", (_, _) => ShowSettings()));
        var help = new MenuItem("&Help");
        help.MenuItems.Add(new MenuItem("&About Weatherspell", (_, _) => ShowAbout()));
        Menu = new MainMenu([file, locations, view, settings, help]);

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
        _locations = new NativeComboBox
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

        // Label and text box in their own panel, label first: the native
        // edit control's accessibility (NativeTextBox) names the box from
        // the static control just before it in z-order, which is the
        // sibling order inside this panel; in the form itself the filling
        // box has to come first for docking.
        var body = new TableLayoutPanel { Dock = DockStyle.Fill, ColumnCount = 1, RowCount = 2, TabIndex = 2 };
        body.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        body.RowStyles.Add(new RowStyle(SizeType.Percent, 100));
        var forecastLabel = new Label
        {
            Text = "Forecast",
            AutoSize = true,
            Anchor = AnchorStyles.Left,
            Margin = new Padding(8, 8, 8, 2),
            TabIndex = 0,
        };
        _forecast = new NativeTextBox
        {
            AccessibleName = "Forecast",
            Multiline = true,
            ReadOnly = true,
            ScrollBars = ScrollBars.Vertical,
            Dock = DockStyle.Fill,
            Margin = new Padding(0),
            TabIndex = 1,
            WordWrap = true,
        };
        body.Controls.Add(forecastLabel, 0, 0);
        body.Controls.Add(_forecast, 0, 1);

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
        Controls.Add(body);
        Controls.Add(header);
        Controls.Add(statusBar);

        _locations.SelectedIndexChanged += async (_, _) => await OnLocationChangedAsync();
        _forecast.KeyDown += OnForecastKeyDown;
        Shown += async (_, _) => await OnShownAsync();
        FormClosed += (_, _) =>
        {
            _forecastTimer.Stop();
            _alertTimer.Stop();
            _clockTimer.Stop();
            _refreshing?.Cancel();
            _closing.Cancel();
        };

        // The forecast on screen, then every saved location's alerts, not
        // just the one on screen, so a warning for home is spoken while
        // reading somewhere else. Neither moves focus or the caret.
        _forecastTimer.Tick += async (_, _) => await RefreshAsync(keepCaret: true, automatic: true);
        _alertTimer.Tick += async (_, _) => await PollAlertsAsync();
        _clockTimer.Tick += (_, _) => RewriteIfChanged();
        ApplyIntervals();

        PopulateLocations();
    }

    private async Task OnShownAsync()
    {
        _forecast.Focus();
        if (_store.LoadProblem is string problem)
        {
            _status.Text = problem;
        }
        // Started before the first location exists, so a first run that
        // adds one is refreshed and checked like any other.
        _forecastTimer.Start();
        _alertTimer.Start();
        _clockTimer.Start();
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
    }

    // Timer intervals from the settings; a running timer restarts from now.
    private void ApplyIntervals()
    {
        _forecastTimer.Interval = _settings.ForecastRefreshMinutes * 60_000;
        _alertTimer.Interval = _settings.AlertCheckMinutes * 60_000;
    }

    private void ShowSettings()
    {
        using var dialog = new SettingsDialog(_settings);
        dialog.Applied += (_, _) =>
        {
            TrySave();
            ApplyIntervals();
        };
        dialog.ShowDialog(this);
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

    // keepCaret: F5 and the timer replace the text under the reader, who
    // stays on the same words; a location switch starts from the top.
    private async Task RefreshAsync(bool keepCaret, bool automatic = false)
    {
        var saved = CurrentSaved;
        var location = saved?.ToLocation();
        if (saved is null || location is null) return;

        _refreshing?.Cancel();
        _refreshing = new CancellationTokenSource();
        var token = _refreshing.Token;

        if (!automatic) _status.Text = $"Fetching the forecast for {location.DisplayName}...";
        try
        {
            var forecastTask = _forecasts.GetAsync(location, Units.For(location), ForecastDays, token);
            var alertsTask = _alerts.GetAsync(location, DateTimeOffset.UtcNow, token);
            var forecast = await forecastTask;
            var alerts = await alertsTask;
            if (token.IsCancellationRequested) return;
            saved.UtcOffsetSeconds = (int)forecast.UtcOffset.TotalSeconds;
            _refreshProblem = null;
            if (keepCaret) Rewrite(forecast, alerts); else Show(forecast, alerts);
            _status.Text = $"{location.DisplayName}: updated {Clock.PcTime(DateTime.Now)} from {forecast.SourceName}";
            // The next automatic refresh a full interval from this one.
            _forecastTimer.Stop();
            _forecastTimer.Start();
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
        catch (OperationCanceledException) when (token.IsCancellationRequested)
        {
        }
        catch (Exception ex) when (ex is System.Net.Http.HttpRequestException or IOException or InvalidDataException or System.Runtime.Serialization.SerializationException or FormatException or OperationCanceledException)
        {
            // HttpClient reports its timeout as a cancellation; the token
            // says whether this one was ours.
            var reason = ex is OperationCanceledException ? "the weather service took too long to answer" : ex.Message;
            if (_shown?.Location == location)
            {
                // The text on screen is this location's: it stays, dated
                // by the age line, rather than giving way to an error. A
                // failure the user asked for is spoken; the timer's is
                // left for the age line to tell.
                _refreshProblem = "couldn't reach the weather service";
                Rewrite(_shown, _shownAlerts);
                _status.Text = $"Couldn't fetch the forecast for {location.DisplayName}: {reason}";
                if (!automatic) Announcer.Say(this, $"Couldn't fetch the forecast for {location.DisplayName}. Showing the forecast from {Clock.PcTime(_shown.FetchedAt.ToLocalTime().DateTime)}.");
            }
            else
            {
                SetText([new Section("Problem", [$"Couldn't fetch the forecast for {location.DisplayName}: {reason}", "Press F5 to try again."])]);
                _status.Text = $"Couldn't fetch the forecast for {location.DisplayName}.";
            }
        }
    }

    private IReadOnlyList<Section> Render() =>
        ForecastWriter.Write(_shown!, WriterOptions.Default() with { RefreshProblem = _refreshProblem }, _shownAlerts);

    private void Show(Forecast forecast, AlertReport alerts)
    {
        _shown = forecast;
        _shownAlerts = alerts;
        SetText(Render());
    }

    // New text with the caret kept on the same words (SectionLayout.MapCaret),
    // so nothing the user did not ask for moves them.
    private void Rewrite(Forecast forecast, AlertReport? alerts)
    {
        _shown = forecast;
        _shownAlerts = alerts;
        var layout = SectionLayout.Build(Render());
        Apply(layout, layout.MapCaret(_layout, _forecast.SelectionStart));
    }

    // The clock's tick: only when the rendering has changed (the age line
    // from 30 minutes, a day heading at midnight), and never under a
    // selection the user is about to copy.
    private void RewriteIfChanged()
    {
        if (_shown is null || _forecast.SelectionLength > 0) return;
        var layout = SectionLayout.Build(Render());
        if (layout.Text == _layout.Text) return;
        Apply(layout, layout.MapCaret(_layout, _forecast.SelectionStart));
    }

    private void SetText(IReadOnlyList<Section> sections) => Apply(SectionLayout.Build(sections), 0);

    private void Apply(SectionLayout layout, int caret)
    {
        _layout = layout;
        _forecast.Text = layout.Text;
        _forecast.SelectionStart = Math.Min(Math.Max(caret, 0), _forecast.TextLength);
        _forecast.SelectionLength = 0;
        _forecast.ScrollToCaret();
    }

    private WeatherAlert? AlertAtCaret() => _layout.AlertAt(_forecast.SelectionStart);

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
                    Rewrite(_shown, report);
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
        var here = _forecast.SelectionStart;
        // No FirstOrDefault(pred, fallback) on 4.8: -1 stands in for "none".
        var target = -1;
        if (direction > 0)
        {
            foreach (var (_, o) in _layout.Headings) { if (o > here) { target = o; break; } }
        }
        else
        {
            foreach (var (_, o) in _layout.Headings) { if (o < here) target = o; else break; }
        }
        if (target < 0) return;
        MoveCaret(target);
    }

    private void JumpToAlerts()
    {
        if (_layout.Headings.Count > 0) MoveCaret(_layout.Headings[0].Offset);
    }

    private void MoveCaret(int offset)
    {
        _forecast.Focus();
        _forecast.SelectionStart = offset;
        _forecast.SelectionLength = 0;
        _forecast.ScrollToCaret();
    }

    // The section keys work wherever focus is, like the menu's own shortcuts.
    protected override bool ProcessCmdKey(ref Message msg, Keys keyData)
    {
        if (keyData == (Keys.Control | Keys.PageDown))
        {
            JumpSection(+1);
            return true;
        }
        if (keyData == (Keys.Control | Keys.PageUp))
        {
            JumpSection(-1);
            return true;
        }
        return base.ProcessCmdKey(ref msg, keyData);
    }

    private void OnForecastKeyDown(object? sender, KeyEventArgs e)
    {
        if (e.KeyCode == Keys.Enter && e.Modifiers == Keys.None && AlertAtCaret() is WeatherAlert alert)
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
