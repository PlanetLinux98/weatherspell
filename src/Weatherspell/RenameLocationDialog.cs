namespace Weatherspell;

// A nickname for a saved location ("Home"), which the Location box and the
// announcements then use; Manage Locations keeps the full name beside it.
// The field's label names the place, so the name a screen reader gives the
// field says which location is being renamed.
internal sealed class RenameLocationDialog : Form
{
    private const string Hint = "Leave it empty to use the full name.";

    private readonly TextBox _nickname;

    // Null when the field is left empty.
    public string? Nickname => string.IsNullOrWhiteSpace(_nickname.Text) ? null : _nickname.Text.Trim();

    public RenameLocationDialog(string fullName, string? nickname)
    {
        Text = "Rename Location";
        Icon = Icon.ExtractAssociatedIcon(Application.ExecutablePath);
        FormBorderStyle = FormBorderStyle.FixedDialog;
        StartPosition = FormStartPosition.CenterParent;
        MinimizeBox = false;
        MaximizeBox = false;
        ShowInTaskbar = false;
        // Sizes before Scaling.Apply, or they are never scaled (see Scaling).
        // Low: the height is fitted to the content on Load, and the title
        // bar does not grow with the font, so a minimum in font units left
        // a gap at large text sizes.
        MinimumSize = new Size(360, 120);
        Size = new Size(460, 180);
        Scaling.Apply(this);

        var layout = new TableLayoutPanel
        {
            Dock = DockStyle.Fill,
            ColumnCount = 1,
            RowCount = 3,
            Padding = new Padding(10, 10, 10, 0),
            TabIndex = 0,
        };
        layout.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100));
        layout.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        layout.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        layout.RowStyles.Add(new RowStyle(SizeType.Percent, 100));

        // Anchored both sides so a long name wraps within the window rather
        // than widening it.
        var label = new Label
        {
            Text = $"&Nickname for {fullName.Replace("&", "&&")}",
            AutoSize = true,
            Anchor = AnchorStyles.Left | AnchorStyles.Right,
            TabIndex = 0,
        };
        _nickname = new TextBox
        {
            AccessibleName = $"Nickname for {fullName}",
            AccessibleDescription = Hint,
            Text = nickname ?? "",
            Dock = DockStyle.Fill,
            TabIndex = 1,
        };
        layout.Controls.Add(label, 0, 0);
        layout.Controls.Add(_nickname, 0, 1);

        // Under the field, docked above the buttons and after them in tab
        // order: a label just before the button row in tab order lends it
        // its text as an accessible name, and this one did.
        var hint = new Label
        {
            Text = Hint,
            AutoSize = true,
            Dock = DockStyle.Bottom,
            Padding = new Padding(13, 4, 13, 8),
            TabIndex = 2,
        };

        var buttons = new FlowLayoutPanel
        {
            FlowDirection = FlowDirection.RightToLeft,
            Dock = DockStyle.Bottom,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            Padding = new Padding(10, 0, 10, 10),
            TabIndex = 1,
        };
        var cancel = new Button { Text = "Cancel", AutoSize = true, DialogResult = DialogResult.Cancel, TabIndex = 1 };
        var ok = new Button { Text = "OK", AutoSize = true, DialogResult = DialogResult.OK, TabIndex = 0 };
        buttons.Controls.Add(cancel);
        buttons.Controls.Add(ok);

        // Fill first, bottom-docked last: WinForms docks the last-added
        // control first.
        Controls.Add(layout);
        Controls.Add(hint);
        Controls.Add(buttons);

        AcceptButton = ok;
        CancelButton = cancel;
        // Closed up on the content once the wrapped label has its height,
        // clamped to the screen as Scaling does.
        Load += (_, _) =>
        {
            var wanted = layout.GetPreferredSize(new Size(layout.ClientSize.Width, 0)).Height + hint.Height + buttons.Height + (Height - ClientSize.Height);
            Height = Math.Min(wanted, Screen.PrimaryScreen.WorkingArea.Height);
        };
        Shown += (_, _) =>
        {
            _nickname.Focus();
            _nickname.SelectAll();
        };
    }
}
