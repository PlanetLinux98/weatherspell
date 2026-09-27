namespace Weatherspell;

// Plain text and buttons, like a message box, so a screen reader reads the
// text as the dialog opens. The full credits and licences are in the user
// guide.
internal sealed class AboutDialog : Form
{
    public AboutDialog()
    {
        Text = "About Weatherspell";
        Icon = Icon.ExtractAssociatedIcon(Application.ExecutablePath);
        FormBorderStyle = FormBorderStyle.FixedDialog;
        StartPosition = FormStartPosition.CenterParent;
        MinimizeBox = false;
        MaximizeBox = false;
        ShowInTaskbar = false;
        // NVDA reads a dialog's static text when it opens, but only for a
        // window whose role is dialog; a WinForms form reports "client".
        // A multiline text box is left out of that reading, hence labels.
        AccessibleRole = AccessibleRole.Dialog;
        // Sizes before Scaling.Apply, or they are never scaled (see Scaling).
        // Low: the height is fitted to the text on Load (see RenameLocationDialog).
        MinimumSize = new Size(400, 160);
        Size = new Size(500, 300);
        Scaling.Apply(this);

        var layout = new TableLayoutPanel
        {
            Dock = DockStyle.Fill,
            ColumnCount = 1,
            Padding = new Padding(12, 12, 12, 4),
            TabIndex = 1,
        };
        layout.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100));
        var paragraphs = AboutText.Paragraphs(AppVersion.Display);
        layout.RowCount = paragraphs.Count;
        for (var i = 0; i < paragraphs.Count; i++)
        {
            layout.RowStyles.Add(new RowStyle(SizeType.AutoSize));
            // Anchored both sides so the text wraps within the window.
            layout.Controls.Add(new Label
            {
                Text = paragraphs[i],
                UseMnemonic = false,
                AutoSize = true,
                Anchor = AnchorStyles.Left | AnchorStyles.Right,
                Margin = new Padding(3, 0, 3, 10),
                TabIndex = i,
            }, 0, i);
        }

        // Before the text in tab order: a label just before the button row
        // lends it its text as an accessible name (see RenameLocationDialog).
        var buttons = new FlowLayoutPanel
        {
            FlowDirection = FlowDirection.RightToLeft,
            Dock = DockStyle.Bottom,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            Padding = new Padding(8, 4, 8, 8),
            TabIndex = 0,
        };
        var ok = new Button { Text = "OK", AutoSize = true, DialogResult = DialogResult.OK, TabIndex = 2 };
        var website = new Button { Text = "&Website", AutoSize = true, TabIndex = 1 };
        var credits = new Button { Text = "&Credits and Licences", AutoSize = true, TabIndex = 0 };
        buttons.Controls.Add(ok);
        buttons.Controls.Add(website);
        buttons.Controls.Add(credits);

        // Fill first, bottom-docked last: WinForms docks the last-added
        // control first.
        Controls.Add(layout);
        Controls.Add(buttons);

        AcceptButton = ok;
        CancelButton = ok;
        credits.Click += (_, _) => Browser.OpenGuide(this, UserGuide.Credits);
        website.Click += (_, _) => Browser.Open(this, Browser.RepositoryUrl, Text);
        // Closed up on the text once the wrapped labels have their height,
        // clamped to the screen as Scaling does.
        Load += (_, _) =>
        {
            var wanted = layout.GetPreferredSize(new Size(layout.ClientSize.Width, 0)).Height + buttons.Height + (Height - ClientSize.Height);
            Height = Math.Min(wanted, Screen.PrimaryScreen.WorkingArea.Height);
        };
        // Focus on OK, as in a message box; Tab reaches the other buttons.
        Shown += (_, _) => ok.Focus();
    }
}
