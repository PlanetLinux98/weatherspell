namespace Weatherspell;

// The UI is hand-coded rather than designer-generated: every control's
// accessible name, tab order and label association is visible in one place
// and reviewable in a diff (see NOTES.md).
internal sealed class MainForm : Form
{
    private readonly TextBox _forecast;
    private readonly ToolStripStatusLabel _status;

    public MainForm()
    {
        Text = "Weatherspell";
        AutoScaleMode = AutoScaleMode.Dpi;
        StartPosition = FormStartPosition.CenterScreen;
        MinimumSize = new Size(480, 360);
        Size = new Size(640, 480);

        var menu = new MenuStrip { TabIndex = 0 };
        var file = new ToolStripMenuItem("&File");
        // Alt+F4 already closes the window; the display string only advertises it.
        file.DropDownItems.Add(new ToolStripMenuItem("E&xit", null, (_, _) => Close())
        {
            ShortcutKeyDisplayString = "Alt+F4",
        });
        var help = new ToolStripMenuItem("&Help");
        help.DropDownItems.Add(new ToolStripMenuItem("&About Weatherspell", null, (_, _) => ShowAbout()));
        menu.Items.AddRange([file, help]);
        MainMenuStrip = menu;

        // The label is the text box's visible name; AccessibleName repeats it
        // verbatim so the announced name never drifts from what is shown.
        var forecastLabel = new Label
        {
            Text = "Forecast",
            AutoSize = true,
            Dock = DockStyle.Top,
            Padding = new Padding(8, 8, 8, 2),
            TabIndex = 1,
        };
        _forecast = new TextBox
        {
            AccessibleName = "Forecast",
            Multiline = true,
            ReadOnly = true,
            ScrollBars = ScrollBars.Vertical,
            Dock = DockStyle.Fill,
            Margin = new Padding(8),
            TabIndex = 2,
            Text = "No location chosen yet.",
        };

        var statusBar = new StatusStrip { TabIndex = 3 };
        _status = new ToolStripStatusLabel("Ready");
        statusBar.Items.Add(_status);

        // Fill first: WinForms docks the last-added control first, so the
        // filling control must sit at index 0 to take what the others leave.
        Controls.Add(_forecast);
        Controls.Add(forecastLabel);
        Controls.Add(statusBar);
        Controls.Add(menu);
    }

    private void ShowAbout()
    {
        MessageBox.Show(this,
            $"Weatherspell {AppVersion.Display}\nA plain-text weather app for Windows.",
            "About Weatherspell", MessageBoxButtons.OK, MessageBoxIcon.Information);
    }
}
