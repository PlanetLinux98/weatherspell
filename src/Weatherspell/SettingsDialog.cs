using Weatherspell.Settings;

namespace Weatherspell;

// Two groups of combo boxes. OK applies and closes, Apply applies and
// stays, Cancel closes and keeps whatever Apply already did, as Windows
// dialogs do. The lists and their wording are SettingsChoices'; the form
// only moves values between them and the AppSettings it was given.
internal sealed class SettingsDialog : Form
{
    private readonly AppSettings _settings;
    private readonly int[] _forecastChoices;
    private readonly int[] _alertChoices;
    private readonly ComboBox _forecastMinutes;
    private readonly ComboBox _alertMinutes;
    private readonly ComboBox _announcements;

    // Raised after the AppSettings has taken the fields' values.
    public event EventHandler? Applied;

    public SettingsDialog(AppSettings settings)
    {
        _settings = settings;
        Text = "Settings";
        Icon = Icon.ExtractAssociatedIcon(Application.ExecutablePath);
        // Fixed: nothing here gains from more room, and the height is fitted
        // to the content below.
        FormBorderStyle = FormBorderStyle.FixedDialog;
        StartPosition = FormStartPosition.CenterParent;
        MinimizeBox = false;
        MaximizeBox = false;
        ShowInTaskbar = false;
        // Sizes before Scaling.Apply, or they are never scaled (see Scaling).
        MinimumSize = new Size(400, 200);
        Size = new Size(480, 240);
        Scaling.Apply(this);

        // Groups stacked in a table above the buttons, which dock to the
        // bottom so a small screen or a large font can never push them off.
        var layout = new TableLayoutPanel
        {
            Dock = DockStyle.Fill,
            ColumnCount = 1,
            RowCount = 3,
            Padding = new Padding(10, 10, 10, 0),
            TabIndex = 0,
        };
        layout.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        layout.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        layout.RowStyles.Add(new RowStyle(SizeType.Percent, 100));

        var forecast = Group(layout, "Forecast", 0, rows: 1);
        _forecastChoices = SettingsChoices.Minutes(SettingsChoices.ForecastMinutes, settings.ForecastRefreshMinutes);
        _forecastMinutes = AddRow(forecast, 0, "&Refresh the forecast every", "Refresh the forecast every",
            _forecastChoices.Select(SettingsChoices.MinutesLabel), Array.IndexOf(_forecastChoices, settings.ForecastRefreshMinutes));

        var alerts = Group(layout, "Alerts and announcements", 1, rows: 2);
        _alertChoices = SettingsChoices.Minutes(SettingsChoices.AlertMinutes, settings.AlertCheckMinutes);
        _alertMinutes = AddRow(alerts, 0, "&Check for alerts every", "Check for alerts every",
            _alertChoices.Select(SettingsChoices.MinutesLabel), Array.IndexOf(_alertChoices, settings.AlertCheckMinutes));
        _announcements = AddRow(alerts, 1, "Announce &new alerts", "Announce new alerts",
            SettingsChoices.Announcements.Select(a => a.Label), Array.FindIndex(SettingsChoices.Announcements, a => a.Value == settings.AlertAnnouncements));


        var buttons = new FlowLayoutPanel
        {
            FlowDirection = FlowDirection.RightToLeft,
            Dock = DockStyle.Bottom,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            Padding = new Padding(10, 0, 10, 10),
            TabIndex = 1,
        };
        // Right-to-left flow: added right to left, read left to right as
        // OK, Cancel, Apply, and tabbed in that order.
        var apply = new Button { Text = "&Apply", AutoSize = true, TabIndex = 2 };
        var cancel = new Button { Text = "Cancel", AutoSize = true, DialogResult = DialogResult.Cancel, TabIndex = 1 };
        var ok = new Button { Text = "OK", AutoSize = true, DialogResult = DialogResult.OK, TabIndex = 0 };
        buttons.Controls.Add(apply);
        buttons.Controls.Add(cancel);
        buttons.Controls.Add(ok);

        // Fill first, bottom-docked last: WinForms docks the last-added
        // control first.
        Controls.Add(layout);
        Controls.Add(buttons);

        AcceptButton = ok;
        CancelButton = cancel;
        ok.Click += (_, _) => ApplyChoices();
        apply.Click += (_, _) => ApplyChoices();
        // By Load every handle exists and the rows have taken their combo
        // boxes' real heights, so the window can close up on its content;
        // a font tall enough to overflow the screen is clamped like Scaling.
        Load += (_, _) =>
        {
            var wanted = layout.GetPreferredSize(Size.Empty).Height + buttons.Height + (Height - ClientSize.Height);
            Height = Math.Min(wanted, Screen.PrimaryScreen.WorkingArea.Height);
        };
        Shown += (_, _) => _forecastMinutes.Focus();
    }

    private void ApplyChoices()
    {
        _settings.ForecastRefreshMinutes = _forecastChoices[_forecastMinutes.SelectedIndex];
        _settings.AlertCheckMinutes = _alertChoices[_alertMinutes.SelectedIndex];
        _settings.AlertAnnouncements = SettingsChoices.Announcements[_announcements.SelectedIndex].Value;
        Applied?.Invoke(this, EventArgs.Empty);
    }

    // A group box in its own row of the outer table, sized by its rows (see
    // AddRow): label column as wide as its widest label, the combo box
    // taking the rest.
    private static (GroupBox Box, TableLayoutPanel Table, TableLayoutPanel Outer) Group(TableLayoutPanel outer, string title, int row, int rows)
    {
        var box = new GroupBox
        {
            Text = title,
            Dock = DockStyle.Fill,
            Margin = new Padding(3, 3, 3, 8),
            TabIndex = row,
        };
        var table = new TableLayoutPanel
        {
            Dock = DockStyle.Fill,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            ColumnCount = 2,
            RowCount = rows,
            Padding = new Padding(6, 2, 6, 4),
            TabIndex = 0,
        };
        table.ColumnStyles.Add(new ColumnStyle(SizeType.AutoSize));
        table.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100));
        for (var i = 0; i < rows; i++) table.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        box.Controls.Add(table);
        outer.Controls.Add(box, 0, row);
        return (box, table, outer);
    }

    // Label then combo box in one row and in tab order; the label's text is
    // the combo box's accessible name, verbatim.
    private static ComboBox AddRow((GroupBox Box, TableLayoutPanel Table, TableLayoutPanel Outer) group, int row, string labelText, string name, IEnumerable<string> items, int selected)
    {
        var label = new Label { Text = labelText, AutoSize = true, Anchor = AnchorStyles.Left, TabIndex = row * 2 };
        var combo = new ComboBox
        {
            AccessibleName = name,
            DropDownStyle = ComboBoxStyle.DropDownList,
            // Top, not centred: centring is done at the height the combo box
            // had before its handle, and is not redone when it grows.
            Anchor = AnchorStyles.Top | AnchorStyles.Left | AnchorStyles.Right,
            TabIndex = row * 2 + 1,
        };
        foreach (var item in items) combo.Items.Add(item);
        combo.SelectedIndex = selected >= 0 ? selected : 0;
        group.Table.Controls.Add(label, 0, row);
        group.Table.Controls.Add(combo, 1, row);
        // A combo box takes its height from the font only once its handle
        // exists, after an auto-sized row has measured it and come out
        // short (clipping at large text sizes), so every row on the way up
        // is sized from the real height whenever that changes: the combo's
        // row, the group box from its rows (auto-sizing measured it a few
        // pixels short as well), and the group's row in the outer table.
        void Fit()
        {
            group.Table.RowStyles[row] = new RowStyle(SizeType.Absolute, combo.Height + combo.Margin.Vertical);
            var boxHeight = group.Table.GetPreferredSize(Size.Empty).Height + group.Box.Height - group.Box.DisplayRectangle.Height;
            group.Outer.RowStyles[group.Outer.GetRow(group.Box)] = new RowStyle(SizeType.Absolute, boxHeight + group.Box.Margin.Vertical);
        }
        combo.HandleCreated += (_, _) => Fit();
        combo.SizeChanged += (_, _) => Fit();
        return combo;
    }
}
