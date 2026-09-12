using System.Text;
using Weatherspell.Settings;
using Weatherspell.Weather;
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
    private AppSettings _settings;

    private readonly ComboBox _locations;
    private readonly TextBox _forecast;
    private readonly ToolStripStatusLabel _status;
    private readonly List<int> _headingOffsets = [];
    private CancellationTokenSource? _refreshing;
    private bool _suppressLocationEvents;

    public MainForm() : this(SettingsStore.Default())
    {
    }

    public MainForm(SettingsStore store)
    {
        _store = store;
        _settings = store.Load();

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
        FormClosed += (_, _) => _refreshing?.Cancel();

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

    private Location? CurrentLocation =>
        _locations.SelectedIndex >= 0 && _locations.SelectedIndex < _settings.Locations.Count
            ? _settings.Locations[_locations.SelectedIndex].ToLocation()
            : null;

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
        var location = CurrentLocation;
        if (location is null) return;

        _refreshing?.Cancel();
        _refreshing = new CancellationTokenSource();
        var token = _refreshing.Token;
        var caret = keepCaret ? _forecast.SelectionStart : 0;

        _status.Text = $"Fetching the forecast for {location.DisplayName}...";
        try
        {
            var forecast = await _client.GetForecastAsync(location, Units.FromWindowsRegion(), ForecastDays, token);
            if (token.IsCancellationRequested) return;
            var sections = ForecastWriter.Write(forecast, WriterOptions.Default());
            SetText(sections, caret);
            _status.Text = $"{location.DisplayName}: updated {Clock.PcTime(DateTime.Now)} from {forecast.SourceName}";
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

    // Heading line, blank line, paragraphs separated by blank lines, blank
    // line: a steady rhythm for line-by-line reading, and a heading offset
    // list for Ctrl+PageDown/PageUp.
    private void SetText(IReadOnlyList<Section> sections, int caret = 0)
    {
        var sb = new StringBuilder();
        _headingOffsets.Clear();
        foreach (var section in sections)
        {
            if (sb.Length > 0) sb.Append("\r\n");
            _headingOffsets.Add(sb.Length);
            sb.Append(section.Heading).Append("\r\n\r\n");
            foreach (var paragraph in section.Paragraphs)
            {
                sb.Append(paragraph).Append("\r\n\r\n");
            }
        }
        _forecast.Text = sb.ToString();
        _forecast.SelectionStart = Math.Min(Math.Max(caret, 0), _forecast.TextLength);
        _forecast.SelectionLength = 0;
        _forecast.ScrollToCaret();
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
        _forecast.Focus();
        _forecast.SelectionStart = target;
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
            $"Weatherspell {AppVersion.Display}\nA text-based weather app for Windows.\n\nForecast data: {OpenMeteoClient.SourceNote}.\nPostal codes for Canada, the UK, Australia, New Zealand and Ireland: {PostalCodes.SourceNote}.",
            "About Weatherspell", MessageBoxButtons.OK, MessageBoxIcon.Information);
    }
}
