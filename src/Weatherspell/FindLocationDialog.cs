using Weatherspell.Weather;
using Weatherspell.Weather.OpenMeteo;

namespace Weatherspell;

// Type a place, press Enter to search, arrow through the results, Enter to
// add. The list only changes when the user asks, never as they type.
internal sealed class FindLocationDialog : Form
{
    private readonly OpenMeteoClient _client;
    private readonly TextBox _query;
    private readonly Button _search;
    private readonly ListBox _results;
    private readonly Label _status;
    private readonly Button _add;
    private CancellationTokenSource? _searching;

    public Location? Chosen { get; private set; }

    public FindLocationDialog(OpenMeteoClient client)
    {
        _client = client;
        Text = "Find Location";
        Icon = Icon.ExtractAssociatedIcon(Application.ExecutablePath);
        AutoScaleMode = AutoScaleMode.Dpi;
        FormBorderStyle = FormBorderStyle.Sizable;
        StartPosition = FormStartPosition.CenterParent;
        MinimizeBox = false;
        MaximizeBox = false;
        ShowInTaskbar = false;
        MinimumSize = new Size(420, 360);
        Size = new Size(520, 420);

        var layout = new TableLayoutPanel
        {
            Dock = DockStyle.Fill,
            ColumnCount = 1,
            RowCount = 6,
            Padding = new Padding(10),
        };
        layout.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        layout.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        layout.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        layout.RowStyles.Add(new RowStyle(SizeType.Percent, 100));
        layout.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        layout.RowStyles.Add(new RowStyle(SizeType.AutoSize));

        var queryLabel = new Label { Text = "&Place name or postal code", AutoSize = true, TabIndex = 0 };
        var queryRow = new TableLayoutPanel { Dock = DockStyle.Fill, ColumnCount = 2, AutoSize = true, TabIndex = 1 };
        queryRow.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100));
        queryRow.ColumnStyles.Add(new ColumnStyle(SizeType.AutoSize));
        _query = new TextBox { AccessibleName = "Place name or postal code", Dock = DockStyle.Fill, TabIndex = 0 };
        _search = new Button { Text = "&Search", AutoSize = true, TabIndex = 1 };
        queryRow.Controls.Add(_query, 0, 0);
        queryRow.Controls.Add(_search, 1, 0);

        var resultsLabel = new Label { Text = "&Results", AutoSize = true, TabIndex = 2, Margin = new Padding(3, 10, 3, 0) };
        _results = new ListBox { AccessibleName = "Results", Dock = DockStyle.Fill, TabIndex = 3, IntegralHeight = false };
        _status = new Label { Text = "Type a place name and press Enter.", AutoSize = true, TabIndex = 4, Margin = new Padding(3, 6, 3, 6) };

        var buttons = new FlowLayoutPanel { FlowDirection = FlowDirection.RightToLeft, Dock = DockStyle.Fill, AutoSize = true, TabIndex = 5 };
        var cancel = new Button { Text = "Cancel", AutoSize = true, DialogResult = DialogResult.Cancel, TabIndex = 1 };
        _add = new Button { Text = "&Add", AutoSize = true, Enabled = false, TabIndex = 0 };
        buttons.Controls.Add(cancel);
        buttons.Controls.Add(_add);

        layout.Controls.Add(queryLabel, 0, 0);
        layout.Controls.Add(queryRow, 0, 1);
        layout.Controls.Add(resultsLabel, 0, 2);
        layout.Controls.Add(_results, 0, 3);
        layout.Controls.Add(_status, 0, 4);
        layout.Controls.Add(buttons, 0, 5);
        Controls.Add(layout);

        // Enter searches while typing and adds once a result is chosen.
        AcceptButton = _search;
        CancelButton = cancel;
        _results.Enter += (_, _) => AcceptButton = _add;
        _results.Leave += (_, _) => AcceptButton = _search;
        _results.SelectedIndexChanged += (_, _) => _add.Enabled = _results.SelectedIndex >= 0;
        _results.DoubleClick += (_, _) => Choose();
        _search.Click += async (_, _) => await SearchAsync();
        _add.Click += (_, _) => Choose();
        FormClosed += (_, _) => _searching?.Cancel();
    }

    private async Task SearchAsync()
    {
        var query = _query.Text.Trim();
        if (query.Length < 2)
        {
            SetStatus("Type at least two characters, then press Enter.");
            return;
        }

        _searching?.Cancel();
        _searching = new CancellationTokenSource();
        var token = _searching.Token;
        SetStatus($"Searching for {query}...");
        _search.Enabled = false;
        try
        {
            var found = await _client.SearchAsync(query, token);
            if (token.IsCancellationRequested) return;
            _results.BeginUpdate();
            _results.Items.Clear();
            foreach (var location in found)
            {
                _results.Items.Add(new ResultItem(location));
            }
            _results.EndUpdate();

            if (found.Count == 0)
            {
                SetStatus($"No places found for {query}.");
                _query.Focus();
                return;
            }
            SetStatus(found.Count == 1 ? "1 place found." : $"{found.Count} places found.");
            _results.SelectedIndex = 0;
            _results.Focus();
        }
        catch (OperationCanceledException)
        {
        }
        catch (Exception ex) when (ex is System.Net.Http.HttpRequestException or IOException or InvalidDataException or System.Runtime.Serialization.SerializationException)
        {
            SetStatus($"Couldn't search: {ex.Message}");
            _query.Focus();
        }
        finally
        {
            _search.Enabled = true;
        }
    }

    private void SetStatus(string text)
    {
        _status.Text = text;
        Announcer.Say(this, text);
    }

    private void Choose()
    {
        if (_results.SelectedItem is not ResultItem item) return;
        Chosen = item.Location;
        DialogResult = DialogResult.OK;
        Close();
    }

    private sealed class ResultItem(Location location)
    {
        public Location Location { get; } = location;
        public override string ToString() => Location.SearchResultText;
    }
}
