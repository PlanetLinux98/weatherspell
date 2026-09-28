using System.Globalization;
using Weatherspell.Cache;
using Weatherspell.Settings;
using Weatherspell.Weather;
using Weatherspell.Weather.Alerts;
using Weatherspell.Weather.OpenMeteo;

namespace Weatherspell;

// The UI is hand-coded rather than designer-generated: every control's
// accessible name, tab order and label association is visible in one place
// and reviewable in a diff (see NOTES.md).
internal sealed class MainForm : Form
{
    private const int ForecastDays = 7;

    private readonly SettingsStore _store;
    private readonly ForecastCache _cache;
    private readonly OpenMeteoClient _client = new();
    private readonly ForecastService _forecasts;
    private readonly AlertService _alerts = new();
    private AppSettings _settings;

    private readonly NativeComboBox _locations;
    private readonly MenuItem _locationsMenu = new("&Locations");
    // The Locations menu's entries for the first nine saved locations.
    private readonly List<MenuItem> _locationItems = [];
    private readonly ReadingBox _forecast;
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
    private readonly SelectionHold _hold = new();
    private readonly CancellationTokenSource _closing = new();
    private bool _polling;
    private int _shownAt;
    private bool _maximized;

    public MainForm() : this(SettingsStore.Default())
    {
    }

    public MainForm(SettingsStore store)
    {
        _store = store;
        _settings = store.Load();
        _cache = ForecastCache.Beside(store);
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
        RestoreWindow();

        // The native Windows menu bar, not MenuStrip: on this runtime
        // MenuStrip tells a screen reader each item's mnemonic and never its
        // shortcut (see NOTES.md, #15). Ctrl+PageDown/PageUp are not in the
        // Shortcut enum, so they are written after a tab (the accelerator
        // column) and caught in ProcessCmdKey; Alt+F4 is the window's own.
        var file = new MenuItem("&File");
        file.MenuItems.Add(new MenuItem("&Refresh", async (_, _) => await Guard(() => RefreshAsync(keepCaret: true)), Shortcut.F5));
        file.MenuItems.Add(new MenuItem("-"));
        file.MenuItems.Add(new MenuItem("E&xit\tAlt+F4", (_, _) => Close()));
        // The saved locations are added below these by PopulateLocations;
        // the one on screen is checked as the menu opens.
        _locationsMenu.MenuItems.Add(new MenuItem("&Manage Locations...", (_, _) => ManageLocations(), Shortcut.CtrlL));
        _locationsMenu.MenuItems.Add(new MenuItem("&Add Location...", (_, _) => AddLocation(), Shortcut.CtrlShiftL));
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
        help.MenuItems.Add(new MenuItem("&User Guide", (_, _) => Browser.OpenGuide(this), Shortcut.F1));
        help.MenuItems.Add(new MenuItem("-"));
        help.MenuItems.Add(new MenuItem("&About Weatherspell", (_, _) => ShowAbout()));
        Menu = new MainMenu([file, _locationsMenu, view, settings, help]);

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
            // Top, not centred: the table centres at the height the combo
            // box had before its handle and does not redo it when it grows,
            // which clipped its bottom at large text sizes (#16).
            Anchor = AnchorStyles.Top | AnchorStyles.Left | AnchorStyles.Right,
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

        // Label and text box in their own panel, label first: the control's
        // own accessibility (ReadingBox) names the box from the static
        // control just before it in z-order, which is the sibling order
        // inside this panel; in the form itself the filling box has to come
        // first for docking.
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
        _forecast = new ReadingBox
        {
            AccessibleName = "Forecast",
            Dock = DockStyle.Fill,
            Margin = new Padding(0),
            TabIndex = 1,
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

        _locations.SelectedIndexChanged += async (_, _) => await Guard(OnLocationChangedAsync);
        _locationsMenu.Popup += (_, _) =>
        {
            for (var i = 0; i < _locationItems.Count; i++) _locationItems[i].Checked = i == _locations.SelectedIndex;
        };
        _forecast.KeyDown += OnForecastKeyDown;
        _forecast.MouseDoubleClick += OnForecastDoubleClick;
        Shown += async (_, _) => await Guard(OnShownAsync);
        // Minimized from maximized, the state alone no longer says which it
        // will return to, so the last one that was not minimized is kept.
        Resize += (_, _) =>
        {
            if (WindowState != FormWindowState.Minimized) _maximized = WindowState == FormWindowState.Maximized;
        };
        FormClosing += (_, _) => RememberWindow();
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
        _forecastTimer.Tick += async (_, _) => await Guard(() => RefreshAsync(keepCaret: true, automatic: true), automatic: true);
        _alertTimer.Tick += async (_, _) => await Guard(() => PollAlertsAsync(), automatic: true);
        _clockTimer.Tick += (_, _) => RewriteIfChanged();
        ApplyIntervals();

        PopulateLocations();
    }

    private async Task OnShownAsync()
    {
        _shownAt = Environment.TickCount;
        _forecast.Focus();
        if (_store.LoadProblem is string problem)
        {
            // Said before the first-run dialog opens over it, where the
            // status bar alone went unnoticed (#22).
            _status.Text = $"Settings could not be read: {_store.LoadError}";
            MessageBox.Show(this, problem, "Weatherspell", MessageBoxButtons.OK, MessageBoxIcon.Warning);
        }
        // Started before the first location exists, so a first run that
        // adds one is refreshed and checked like any other.
        _forecastTimer.Start();
        _alertTimer.Start();
        _clockTimer.Start();
        // Not after settings that could not be read: no location is loaded,
        // so every location's file would go.
        if (_store.LoadProblem is null) TryCache(() => _cache.Prune(_settings.Locations.Select(l => l.ToLocation())));
        if (_settings.Locations.Count == 0)
        {
            ShowWelcome();
            AddLocation();
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

        // "&1 Home" with Ctrl+1, to "&9" with Ctrl+9: the menu shows each
        // shortcut, and a location past the ninth is reached in the box.
        while (_locationsMenu.MenuItems.Count > LocationsMenuCommands) _locationsMenu.MenuItems.RemoveAt(LocationsMenuCommands);
        _locationItems.Clear();
        var count = Math.Min(9, _settings.Locations.Count);
        if (count > 0) _locationsMenu.MenuItems.Add(new MenuItem("-"));
        for (var i = 0; i < count; i++)
        {
            var index = i;
            var name = _settings.Locations[i].ToLocation().DisplayName.Replace("&", "&&");
            var item = new MenuItem($"&{i + 1} {name}", (_, _) => ShowLocation(index), (Shortcut)((int)Shortcut.Ctrl1 + i)) { RadioCheck = true };
            _locationItems.Add(item);
            _locationsMenu.MenuItems.Add(item);
        }
    }

    // Manage Locations and Add Location, above the saved locations.
    private const int LocationsMenuCommands = 2;

    // Ctrl+1 to Ctrl+9. Focus stays where it is, so the location is named;
    // the refresh then says when its forecast is ready, as for the box.
    private void ShowLocation(int index)
    {
        if (index >= _locations.Items.Count) return;
        _locations.SelectedIndex = index;
        Announcer.Say(this, _settings.Locations[index].ToLocation().DisplayName);
    }

    private void ShowWelcome()
    {
        _refreshing?.Cancel();
        _shown = null;
        _shownAlerts = null;
        _refreshProblem = null;
        SetText([new Section("Welcome", ["No location yet. Press Ctrl+Shift+L, or use Locations > Add Location, to add one."])]);
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

    // The edits land only on OK (see LocationEditor). The location on
    // screen stays on screen wherever it has moved, and a rename leaves its
    // text as it is; if it was removed, the one now in its place is shown.
    private void ManageLocations()
    {
        var before = CurrentSaved;
        var beforeIndex = _locations.SelectedIndex;
        using var dialog = new ManageLocationsDialog(_settings.Locations, beforeIndex, new LocationSearch(_client));
        if (dialog.ShowDialog(this) != DialogResult.OK) return;

        var committed = dialog.Editor.Commit();
        var show = LocationEditor.Shown(committed, before, beforeIndex);
        _settings.Locations = committed;
        _settings.LastLocation = Math.Max(show, 0);
        TrySave();
        // A removed location's file goes now rather than at the next launch;
        // not after settings that could not be read (see OnShownAsync).
        if (_store.LoadProblem is null) TryCache(() => _cache.Prune(committed.Select(l => l.ToLocation())));
        PopulateLocations();
        if (show < 0)
        {
            ShowWelcome();
            _status.Text = "Ready";
        }
        else if (!ReferenceEquals(committed[show], before))
        {
            _ = Guard(() => RefreshAsync(keepCaret: false));
        }
    }

    private void AddLocation()
    {
        using var dialog = new AddLocationDialog(new LocationSearch(_client));
        if (dialog.ShowDialog(this) != DialogResult.OK || dialog.Chosen is null) return;

        // A place already saved is switched to, not saved twice under the
        // same name (#22).
        var saved = _settings.IndexOf(dialog.Chosen);
        if (saved >= 0)
        {
            _ = SayAsync($"{_settings.Locations[saved].ToLocation().DisplayName} is already saved.", Environment.TickCount, CancellationToken.None);
            _locations.SelectedIndex = saved;
            _forecast.Focus();
            return;
        }

        _settings.Locations.Add(SavedLocation.From(dialog.Chosen));
        _settings.LastLocation = _settings.Locations.Count - 1;
        TrySave();
        PopulateLocations();
        // PopulateLocations selects the new entry silently; fetch it now.
        _ = Guard(() => RefreshAsync(keepCaret: false));
        _forecast.Focus();
    }

    // The last resort for a refresh or a poll: whatever the expected
    // failures (the network, a service's data) do not cover ends in the
    // status bar and, while the text is still waiting, a Problem section,
    // rather than the .NET error dialog with the text left at "Fetching"
    // (#22).
    private async Task Guard(Func<Task> work, bool automatic = false)
    {
        try
        {
            await work();
        }
        catch (Exception ex) when (ex is not OperationCanceledException)
        {
            _status.Text = $"Something went wrong: {ex.Message}";
            if (_shown is null) SetText([new Section("Problem", [$"Something went wrong: {ex.Message}", "Press F5 to try again."])]);
            if (!automatic) Announcer.Say(this, "Something went wrong. Press F5 to try again.");
        }
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
        var started = Environment.TickCount;

        if (!automatic) _status.Text = $"Fetching the forecast for {location.DisplayName}...";
        // A launch or a switch says what is happening rather than showing
        // an empty box or the previous location's text; F5 and the timer
        // keep the text, and the reader's place in it, until there is
        // something new.
        if (!keepCaret) ShowFetching(location);

        var forecastTask = _forecasts.GetAsync(location, Units.For(location), ForecastDays, token);
        var alertsTask = _alerts.GetAsync(location, DateTimeOffset.UtcNow, token);
        Forecast? forecast = null;
        var reason = "";
        try
        {
            forecast = await forecastTask;
        }
        catch (OperationCanceledException) when (token.IsCancellationRequested)
        {
            return;
        }
        catch (Exception ex) when (ex is System.Net.Http.HttpRequestException or IOException or InvalidDataException or System.Runtime.Serialization.SerializationException or FormatException or OperationCanceledException)
        {
            // HttpClient reports its timeout as a cancellation; the token
            // says whether this one was ours. The innermost message names
            // the cause ("The remote name could not be resolved"); the
            // outer one says only that a request failed.
            reason = ex is OperationCanceledException ? "the weather service took too long to answer" : ex.GetBaseException().Message;
        }
        // The alert check reduces its own failures to a report, so it is
        // waited for either way: an alert service that answers while the
        // forecast's does not still gives live alerts.
        AlertReport alerts;
        try
        {
            alerts = await alertsTask;
        }
        catch (OperationCanceledException) when (token.IsCancellationRequested)
        {
            return;
        }
        if (token.IsCancellationRequested) return;

        if (forecast is null)
        {
            var failure = ShowFailure(saved, location, reason, alerts, keepCaret, automatic);
            if (failure is not null) await SayAsync(failure, started, token);
            return;
        }
        saved.UtcOffsetSeconds = (int)forecast.UtcOffset.TotalSeconds;
        _refreshProblem = null;
        if (keepCaret) Rewrite(forecast, alerts, automatic); else Show(forecast, alerts);
        _status.Text = $"{location.DisplayName}: updated {Clock.PcTime(DateTime.Now)} from {forecast.SourceName}";
        // The next automatic refresh a full interval from this one.
        _forecastTimer.Stop();
        _forecastTimer.Start();
        TrackAlerts(saved, alerts, announce: keepCaret);
        TrySave();
        TryCache(() => _cache.Save(location, forecast, alerts.Checked || !alerts.IsAvailable ? alerts : null));
        // Neither the new text nor F5's rewrite makes a sound of its own
        // (NVDA ignores an edit control's value changing); the timer's
        // rewrites stay silent by design. A launch or a switch names the
        // alerts in effect, which TrackAlerts has marked seen unspoken;
        // F5's new ones were announced there.
        if (!automatic) await SayAsync($"{location.DisplayName}: forecast {(keepCaret ? "updated" : "ready")}{(keepCaret ? "" : InEffect(alerts))}.", started, token);
    }

    // NVDA drops a notification from a window it has not yet seen come to
    // the front, and speaks one raised straight after a key ahead of its
    // own report of that key (the combo box's new location). A fetch that
    // fails at once, with no network, did both: at launch the failure was
    // never heard, and on a switch it came before the location's name. So
    // what a refresh has to say waits until the window has been up for a
    // moment and the key that started it has been answered (#17).
    private async Task SayAsync(string text, int started, CancellationToken token)
    {
        var now = Environment.TickCount;
        var wait = Math.Max(1500 - unchecked(now - _shownAt), 400 - unchecked(now - started));
        if (wait > 0)
        {
            try
            {
                await Task.Delay(wait, token);
            }
            catch (OperationCanceledException)
            {
                return;
            }
        }
        if (!token.IsCancellationRequested) Announcer.Say(this, text);
    }

    private void ShowFetching(Location location)
    {
        _shown = null;
        _shownAlerts = null;
        _refreshProblem = null;
        SetText([new Section($"Fetching the forecast for {location.DisplayName}...", [])]);
    }

    // What a failed fetch leaves on screen, dated by the age line under
    // Right now rather than giving way to an error: the text already there
    // (F5, the timer), else the cached text, else a Problem section. A
    // failure the user asked for is spoken (the sentence is returned for
    // the caller to say); the timer's is left for the age line to tell.
    // Live alerts go with whichever text is shown; a failed check keeps
    // the last known alerts, dated.
    private string? ShowFailure(SavedLocation saved, Location location, string reason, AlertReport alerts, bool keepCaret, bool automatic)
    {
        _refreshProblem = "couldn't reach the weather service";
        string spoken;
        if (_shown?.Location.IsSamePlace(location) == true)
        {
            Rewrite(_shown, alerts.OrLastKnown(_shownAlerts), automatic);
            _status.Text = $"Couldn't fetch the forecast for {location.DisplayName}: {reason}";
            spoken = $"Couldn't fetch the forecast for {location.DisplayName}. Showing the forecast from {Clock.PcTimeOnDay(_shown.FetchedAt.ToLocalTime().DateTime, DateTime.Now)}.";
        }
        else if (_cache.Load(location) is CachedForecast cached)
        {
            Show(cached.Forecast, alerts.OrLastKnown(cached.Alerts));
            _status.Text = $"Couldn't fetch the forecast for {location.DisplayName}: {reason}";
            spoken = $"Couldn't fetch the forecast for {location.DisplayName}. Showing the forecast from {Clock.PcTimeOnDay(cached.Forecast.FetchedAt.ToLocalTime().DateTime, DateTime.Now)}{(keepCaret ? "" : InEffect(alerts))}.";
        }
        else
        {
            _refreshProblem = null;
            SetText([new Section("Problem", [$"Couldn't fetch the forecast for {location.DisplayName}: {reason}", "Press F5 to try again."])]);
            _status.Text = $"Couldn't fetch the forecast for {location.DisplayName}.";
            spoken = $"Couldn't fetch the forecast for {location.DisplayName}.";
        }
        if (alerts.Checked)
        {
            TrackAlerts(saved, alerts, announce: keepCaret);
            TrySave();
            TryCache(() => _cache.SaveAlerts(location, alerts));
        }
        return automatic ? null : spoken;
    }

    // "; frost advisory in effect", or nothing (see AlertWriter.InEffect).
    private static string InEffect(AlertReport alerts) =>
        AlertWriter.InEffect(alerts) is string inEffect ? "; " + inEffect : "";

    // announce: F5 keeps the caret, so an alert that has appeared above it
    // is spoken; a location shown from the top is read from its Alerts
    // line anyway.
    private void TrackAlerts(SavedLocation saved, AlertReport alerts, bool announce)
    {
        if (!alerts.Checked) return;
        var fresh = AlertTracker.Update(saved.SeenAlertIds, alerts.Alerts);
        if (announce && saved.NotifyAlerts) Announce(saved, fresh);
    }

    private IReadOnlyList<Section> Render() =>
        ForecastWriter.Write(_shown!, WriterOptions.Default() with { RefreshProblem = _refreshProblem }, _shownAlerts);

    private void Show(Forecast forecast, AlertReport alerts)
    {
        _shown = forecast;
        _shownAlerts = alerts;
        SetText(Render());
    }

    // New text with the caret kept on the same words (SectionLayout.MapCaret)
    // and the view where it was, so nothing the user did not ask for moves
    // them; see SectionLayout.Plan for when the text box is left alone.
    private void Rewrite(Forecast forecast, AlertReport? alerts, bool automatic)
    {
        _shown = forecast;
        _shownAlerts = alerts;
        var layout = SectionLayout.Build(Render());
        var selecting = _hold.Holds(_forecast.SelectionLength > 0, DateTime.UtcNow);
        switch (SectionLayout.Plan(_layout, layout, selecting, automatic))
        {
            case RewritePlan.KeepText:
                _layout = layout;
                _hold.Release();
                break;
            case RewritePlan.Replace:
                Replace(layout);
                break;
        }
    }

    // The clock's tick: only when the rendering has changed (the age line
    // from 30 minutes, a day heading at midnight, or a rewrite that waited
    // for a selection), and not under a selection the user may be about to
    // copy, unless it has been there too long (SelectionHold).
    private void RewriteIfChanged()
    {
        if (_shown is null) return;
        var layout = SectionLayout.Build(Render());
        if (layout.Text == _layout.Text)
        {
            _hold.Release();
            return;
        }
        if (_hold.Holds(_forecast.SelectionLength > 0, DateTime.UtcNow)) return;
        Replace(layout);
    }

    private void Replace(SectionLayout layout)
    {
        _hold.Release();
        var caret = layout.MapCaret(_layout, _forecast.SelectionStart);
        _layout = layout;
        _forecast.ReplaceText(layout.Text, caret);
    }

    private void SetText(IReadOnlyList<Section> sections) => Apply(SectionLayout.Build(sections), 0);

    private void Apply(SectionLayout layout, int caret)
    {
        _hold.Release();
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
                if (!_settings.Locations.Contains(saved)) continue;
                if (report.Checked)
                {
                    var before = saved.SeenAlertIds.ToList();
                    var fresh = AlertTracker.Update(saved.SeenAlertIds, report.Alerts);
                    if (!before.SequenceEqual(saved.SeenAlertIds)) changed = true;
                    if (saved.NotifyAlerts) Announce(saved, fresh);
                    TryCache(() => _cache.SaveAlerts(location, report));
                }
                // Only if the text on screen is still this location's: the
                // user may have switched while the check was in flight. A
                // check that failed dates the alerts it leaves on screen.
                if (ReferenceEquals(saved, CurrentSaved) && _shown?.Location.IsSamePlace(location) == true && _shownAlerts is not null)
                {
                    Rewrite(_shown, report.OrLastKnown(_shownAlerts), automatic: true);
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
            for (var i = 0; i < _layout.Headings.Count; i++) { if (_layout.Headings[i].Offset > here) { target = i; break; } }
        }
        else
        {
            for (var i = 0; i < _layout.Headings.Count; i++) { if (_layout.Headings[i].Offset < here) target = i; else break; }
        }
        if (target < 0)
        {
            Announcer.Say(this, direction > 0 ? "No next section." : "No previous section.");
            return;
        }
        MoveCaret(_layout.Headings[target]);
    }

    private void JumpToAlerts()
    {
        if (_layout.Headings.Count > 0) MoveCaret(_layout.Headings[0]);
    }

    // NVDA reads the new line only after the navigation keys it knows
    // (arrows, Home, End, Ctrl+Home); after these jumps it said nothing,
    // so the heading is spoken (#17). When the jump moves focus into the
    // text box, NVDA reads the line itself as focus arrives.
    private void MoveCaret((string Heading, int Offset) to)
    {
        var speak = _forecast.Focused;
        _forecast.Focus();
        _forecast.SelectionStart = to.Offset;
        _forecast.SelectionLength = 0;
        _forecast.ScrollToCaret();
        if (speak) Announcer.Say(this, to.Heading);
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

    // RichEdit has already selected the word under the pointer; the caret
    // goes back to that point, since a selection left behind would hold off
    // the next automatic refresh.
    private void OnForecastDoubleClick(object? sender, MouseEventArgs e)
    {
        if (e.Button != MouseButtons.Left) return;
        var index = _forecast.GetCharIndexFromPosition(e.Location);
        if (_layout.AlertAt(index) is not WeatherAlert alert) return;
        _forecast.Select(index, 0);
        ShowAlertDetails(alert);
    }

    // The cache is a convenience: a folder that cannot be written costs the
    // offline view, not the forecast.
    private void TryCache(Action write)
    {
        try
        {
            write();
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            _status.Text = $"Couldn't keep the forecast for offline use: {ex.Message}";
        }
    }

    // After Scaling.Apply, so the saved pixels are not scaled a second time.
    // A place no current screen shows opens centred; maximized stays
    // maximized either way.
    private void RestoreWindow()
    {
        if (_settings.Window is not SavedWindow saved) return;
        var workingAreas = Screen.AllScreens.Select(s => s.WorkingArea).ToList();
        if (WindowPlacement.Restore(saved, CurrentAutoScaleDimensions, workingAreas, SystemInformation.CaptionHeight) is Rectangle bounds)
        {
            StartPosition = FormStartPosition.Manual;
            Bounds = bounds;
        }
        if (saved.Maximized) WindowState = FormWindowState.Maximized;
        _maximized = saved.Maximized;
    }

    // Written only when it changed, so an ordinary close does not rewrite
    // settings.json, and never after settings that could not be read: the
    // user may be mending that file by hand while the app is open.
    private void RememberWindow()
    {
        if (_store.LoadProblem is not null) return;
        var normal = WindowState == FormWindowState.Normal ? Bounds : RestoreBounds;
        var window = WindowPlacement.Save(normal, _maximized, CurrentAutoScaleDimensions);
        if (window == _settings.Window) return;
        _settings.Window = window;
        TrySave();
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
        using var dialog = new AboutDialog();
        dialog.ShowDialog(this);
    }
}
