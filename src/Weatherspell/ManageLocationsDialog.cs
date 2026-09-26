using Weatherspell.Settings;
using Weatherspell.Weather;

namespace Weatherspell;

// The saved locations in their order (Ctrl+1 to Ctrl+9 follow it), with
// Move Up, Move Down, Add, Rename and Remove, and whether each location's
// new alerts are spoken. Edits go to a LocationEditor and reach the
// settings only on OK. The buttons act on the selected location and are
// never disabled: a button greyed out under focus throws focus somewhere
// else. Their Alt keys press them without taking focus from the list, so a
// location can be arrowed to and moved with Alt+U and Alt+D; Delete and F2
// in the list remove and rename, as in Explorer.
internal sealed class ManageLocationsDialog : Form
{
    private readonly LocationSearch _search;
    private readonly ListBox _list;
    private readonly CheckBox _notify;
    private readonly Button _add;
    private bool _updating;

    public LocationEditor Editor { get; }

    public ManageLocationsDialog(IEnumerable<SavedLocation> saved, int selected, LocationSearch search)
    {
        Editor = new LocationEditor(saved);
        _search = search;
        Text = "Manage Locations";
        Icon = Icon.ExtractAssociatedIcon(Application.ExecutablePath);
        FormBorderStyle = FormBorderStyle.Sizable;
        StartPosition = FormStartPosition.CenterParent;
        MinimizeBox = false;
        MaximizeBox = false;
        ShowInTaskbar = false;
        // Sizes before Scaling.Apply, or they are never scaled (see Scaling).
        MinimumSize = new Size(440, 300);
        Size = new Size(560, 380);
        Scaling.Apply(this);

        // The list fills the left column with its label above and the
        // check box below; the buttons stack in the right column. OK and
        // Cancel dock to the bottom, so a small screen or a large font
        // shortens the list rather than pushing them off the window.
        var layout = new TableLayoutPanel
        {
            Dock = DockStyle.Fill,
            ColumnCount = 2,
            RowCount = 3,
            Padding = new Padding(10, 10, 10, 0),
            TabIndex = 0,
        };
        layout.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100));
        layout.ColumnStyles.Add(new ColumnStyle(SizeType.AutoSize));
        layout.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        layout.RowStyles.Add(new RowStyle(SizeType.Percent, 100));
        layout.RowStyles.Add(new RowStyle(SizeType.AutoSize));

        var listLabel = new Label { Text = "Saved &locations", AutoSize = true, TabIndex = 0 };
        // A scroll bar for a line wider than the list, rather than cutting
        // it off, at the minimum width or a large text size.
        _list = new ListBox { AccessibleName = "Saved locations", Dock = DockStyle.Fill, IntegralHeight = false, HorizontalScrollbar = true, TabIndex = 1 };
        // Straight after the list in tab order: it belongs to the location
        // selected there.
        _notify = new CheckBox { Text = "&Notify me about alerts", AutoSize = true, Margin = new Padding(3, 6, 3, 3), TabIndex = 2 };

        var side = new TableLayoutPanel
        {
            ColumnCount = 1,
            RowCount = 5,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            Anchor = AnchorStyles.Top | AnchorStyles.Left,
            Margin = new Padding(8, 0, 0, 0),
            TabIndex = 3,
        };
        side.ColumnStyles.Add(new ColumnStyle(SizeType.AutoSize));
        var moveUp = SideButton(side, "Move &Up");
        var moveDown = SideButton(side, "Move &Down");
        _add = SideButton(side, "&Add...");
        var rename = SideButton(side, "Rena&me...");
        var remove = SideButton(side, "&Remove");

        layout.Controls.Add(listLabel, 0, 0);
        layout.Controls.Add(_list, 0, 1);
        layout.Controls.Add(_notify, 0, 2);
        layout.Controls.Add(side, 1, 1);

        var buttons = new FlowLayoutPanel
        {
            FlowDirection = FlowDirection.RightToLeft,
            Dock = DockStyle.Bottom,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            Padding = new Padding(10, 6, 10, 10),
            TabIndex = 1,
        };
        var cancel = new Button { Text = "Cancel", AutoSize = true, DialogResult = DialogResult.Cancel, TabIndex = 1 };
        var ok = new Button { Text = "OK", AutoSize = true, DialogResult = DialogResult.OK, TabIndex = 0 };
        buttons.Controls.Add(cancel);
        buttons.Controls.Add(ok);

        // Fill first, bottom-docked last: WinForms docks the last-added
        // control first.
        Controls.Add(layout);
        Controls.Add(buttons);

        AcceptButton = ok;
        CancelButton = cancel;

        foreach (var entry in Editor.Entries) _list.Items.Add(entry);
        if (_list.Items.Count > 0) _list.SelectedIndex = Math.Min(Math.Max(selected, 0), _list.Items.Count - 1);
        ShowSelection();

        _list.SelectedIndexChanged += (_, _) => ShowSelection();
        _list.KeyDown += OnListKeyDown;
        _notify.CheckedChanged += (_, _) => OnNotifyChanged();
        moveUp.Click += (_, _) => MoveSelected(-1);
        moveDown.Click += (_, _) => MoveSelected(+1);
        _add.Click += (_, _) => Add();
        rename.Click += (_, _) => Rename();
        remove.Click += (_, _) => Remove();
        Shown += (_, _) =>
        {
            if (_list.Items.Count > 0) _list.Focus(); else _add.Focus();
        };
    }

    // One button per row of the side column, each as wide as the widest.
    private static Button SideButton(TableLayoutPanel side, string text)
    {
        var row = side.Controls.Count;
        var button = new Button { Text = text, AutoSize = true, Dock = DockStyle.Fill, TabIndex = row };
        side.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        side.Controls.Add(button, 0, row);
        return button;
    }

    private void OnListKeyDown(object? sender, KeyEventArgs e)
    {
        if (e.Modifiers != Keys.None) return;
        if (e.KeyCode == Keys.Delete)
        {
            e.Handled = true;
            Remove();
        }
        else if (e.KeyCode == Keys.F2)
        {
            e.Handled = true;
            Rename();
        }
    }

    private void ShowSelection()
    {
        if (_updating) return;
        var index = _list.SelectedIndex;
        _updating = true;
        _notify.Checked = index >= 0 && Editor.Entries[index].NotifyAlerts;
        _updating = false;
        _notify.Enabled = index >= 0;
    }

    private void OnNotifyChanged()
    {
        var index = _list.SelectedIndex;
        if (_updating || index < 0) return;
        Editor.SetNotify(index, _notify.Checked);
        ShowEntry(index, index);
    }

    // With focus in the list, the list reports the location as it is
    // selected again at its new place, as it does for an arrow key; from a
    // button, nothing would, so the new position is spoken.
    private void MoveSelected(int by)
    {
        var index = _list.SelectedIndex;
        if (index < 0)
        {
            Say("No location selected.");
            return;
        }
        if (!Editor.Move(index, by))
        {
            Say(by < 0 ? "Already at the top." : "Already at the bottom.");
            return;
        }
        var speak = !_list.Focused;
        ShowEntry(index, index + by);
        if (speak) Say($"{Editor.Entries[index + by].DisplayName}, {index + by + 1} of {_list.Items.Count}.");
    }

    private void Add()
    {
        using var dialog = new AddLocationDialog(_search);
        if (dialog.ShowDialog(this) != DialogResult.OK || dialog.Chosen is null) return;
        var (index, added) = Editor.Add(dialog.Chosen);
        _updating = true;
        if (added) _list.Items.Add(Editor.Entries[index]);
        _list.SelectedIndex = index;
        _updating = false;
        ShowSelection();
        // Into the list, where the new location is read out with its place
        // in the order and can be moved straight away.
        _list.Focus();
        if (!added) _ = SayLaterAsync($"{Editor.Entries[index].DisplayName} is already in the list.");
    }

    private void Rename()
    {
        var index = _list.SelectedIndex;
        if (index < 0)
        {
            Say("No location selected.");
            return;
        }
        var entry = Editor.Entries[index];
        using var dialog = new RenameLocationDialog(entry.Place.FullName, entry.Nickname);
        if (dialog.ShowDialog(this) != DialogResult.OK) return;
        Editor.Rename(index, dialog.Nickname);
        ShowEntry(index, index);
        // Back in the list, which reads the location under its new name.
        _list.Focus();
    }

    private void Remove()
    {
        var index = _list.SelectedIndex;
        if (index < 0)
        {
            Say("No location selected.");
            return;
        }
        var name = Editor.Entries[index].DisplayName;
        Editor.Remove(index);
        _updating = true;
        _list.Items.RemoveAt(index);
        if (_list.Items.Count > 0) _list.SelectedIndex = Math.Min(index, _list.Items.Count - 1);
        _updating = false;
        ShowSelection();
        Say(_list.Items.Count > 0 ? $"Removed {name}." : $"Removed {name}. No saved locations.");
    }

    // The list item at from, taken out and put back at to with its current
    // text, and selected: the list keeps the text it was given, and setting
    // an item in place skips a change of case alone.
    private void ShowEntry(int from, int to)
    {
        _updating = true;
        _list.BeginUpdate();
        _list.Items.RemoveAt(from);
        _list.Items.Insert(to, Editor.Entries[to]);
        _list.EndUpdate();
        _list.SelectedIndex = to;
        _updating = false;
    }

    private void Say(string text) => Announcer.Say(this, text);

    // After a dialog of its own closes, NVDA reads where focus lands; the
    // pause lets that come first, as MainForm.SayAsync does for a key.
    private async Task SayLaterAsync(string text)
    {
        await Task.Delay(400);
        Say(text);
    }
}
