using System.ComponentModel;
using System.Diagnostics;
using System.Text;

namespace Weatherspell;

// The full text of one alert in a read-only text box, so it can be read
// line by line, selected and copied like the forecast. Enter or Escape
// closes; the official page opens in the browser.
internal sealed class AlertDialog : Form
{
    private readonly TextBox _text;

    public AlertDialog(string title, IReadOnlyList<string> paragraphs, string? url)
    {
        Text = title;
        Icon = Icon.ExtractAssociatedIcon(Application.ExecutablePath);
        FormBorderStyle = FormBorderStyle.Sizable;
        StartPosition = FormStartPosition.CenterParent;
        MinimizeBox = false;
        MaximizeBox = true;
        ShowInTaskbar = false;
        // Sizes before Scaling.Apply, or they are never scaled (see Scaling).
        MinimumSize = new Size(400, 300);
        Size = new Size(600, 460);
        Scaling.Apply(this);

        var label = new Label
        {
            Text = "&Details",
            AutoSize = true,
            Dock = DockStyle.Top,
            Padding = new Padding(8, 8, 8, 2),
            TabIndex = 0,
        };
        _text = new TextBox
        {
            AccessibleName = "Details",
            Multiline = true,
            ReadOnly = true,
            ScrollBars = ScrollBars.Vertical,
            Dock = DockStyle.Fill,
            WordWrap = true,
            TabIndex = 1,
        };
        var sb = new StringBuilder();
        foreach (var paragraph in paragraphs)
        {
            if (sb.Length > 0) sb.Append("\r\n\r\n");
            sb.Append(paragraph);
        }
        _text.Text = sb.ToString();

        var buttons = new FlowLayoutPanel
        {
            FlowDirection = FlowDirection.RightToLeft,
            Dock = DockStyle.Bottom,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            Padding = new Padding(8, 4, 8, 8),
            TabIndex = 2,
        };
        var close = new Button { Text = "Close", AutoSize = true, DialogResult = DialogResult.Cancel, TabIndex = 1 };
        var official = new Button { Text = "&Official page", AutoSize = true, Enabled = url is not null, TabIndex = 0 };
        buttons.Controls.Add(close);
        buttons.Controls.Add(official);

        // Fill first, bottom-docked last: WinForms docks the last-added
        // control first.
        Controls.Add(_text);
        Controls.Add(label);
        Controls.Add(buttons);

        AcceptButton = close;
        CancelButton = close;
        official.Click += (_, _) => OpenOfficialPage(url!);
        // Focus lands in the text with the caret at the top and nothing
        // selected; a text box given focus would otherwise select it all.
        Shown += (_, _) =>
        {
            _text.Focus();
            _text.SelectionStart = 0;
            _text.SelectionLength = 0;
        };
    }

    private void OpenOfficialPage(string url)
    {
        try
        {
            Process.Start(new ProcessStartInfo(url) { UseShellExecute = true });
        }
        catch (Exception ex) when (ex is Win32Exception or InvalidOperationException)
        {
            MessageBox.Show(this, $"Couldn't open the browser: {ex.Message}\n\n{url}", Text, MessageBoxButtons.OK, MessageBoxIcon.Warning);
        }
    }
}
