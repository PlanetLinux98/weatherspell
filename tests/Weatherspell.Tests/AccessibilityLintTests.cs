using Weatherspell.Settings;
using Weatherspell.Weather;
using Weatherspell.Weather.OpenMeteo;
using Xunit;

namespace Weatherspell.Tests;

public class AccessibilityLintTests
{
    [Fact]
    public void MainForm_passes_the_accessibility_lint()
    {
        var failures = Sta.Run(() =>
        {
            // A store pointing nowhere, so the test never reads real settings.
            using var form = new MainForm(new SettingsStore(Path.Combine(Path.GetTempPath(), "weatherspell-lint", "settings.json")));
            return AccessibilityLint.Check(form).ToList();
        });

        Assert.True(failures.Count == 0, string.Join(Environment.NewLine, failures));
    }

    [Fact]
    public void FindLocationDialog_passes_the_accessibility_lint()
    {
        var failures = Sta.Run(() =>
        {
            using var form = new FindLocationDialog(new LocationSearch(new OpenMeteoClient()));
            return AccessibilityLint.Check(form).ToList();
        });

        Assert.True(failures.Count == 0, string.Join(Environment.NewLine, failures));
    }

    [Fact]
    public void AlertDialog_passes_the_accessibility_lint()
    {
        var failures = Sta.Run(() =>
        {
            using var form = new AlertDialog("Frost advisory", ["Frost advisory from Environment Canada, in effect until 6:30 am tomorrow.", "Areas of frost are expected."], "https://weather.gc.ca/");
            return AccessibilityLint.Check(form).ToList();
        });

        Assert.True(failures.Count == 0, string.Join(Environment.NewLine, failures));
    }

    // Guards the lint itself: an unlabelled input, a renamed button and a
    // duplicate tab index must each be reported, or a green run means nothing.
    [Fact]
    public void Lint_reports_the_rules_it_enforces()
    {
        var failures = Sta.Run(() =>
        {
            using var form = new Form { Text = "Probe" };
            form.Controls.Add(new TextBox { Name = "city", TabIndex = 0 });
            form.Controls.Add(new Button { Name = "go", Text = "Refresh", AccessibleName = "Fetch the forecast now", TabIndex = 0 });
            return AccessibilityLint.Check(form).ToList();
        });

        Assert.Contains(failures, f => f.Contains("\"city\"") && f.Contains("no AccessibleName"));
        Assert.Contains(failures, f => f.Contains("\"go\"") && f.Contains("does not start with the visible text"));
        Assert.Contains(failures, f => f.Contains("tab index 0 is shared"));
    }

    [Fact]
    public void Lint_reports_menu_items_without_a_mnemonic_or_sharing_one()
    {
        var failures = Sta.Run(() =>
        {
            using var form = new Form { Text = "Probe" };
            var file = new MenuItem("&File");
            file.MenuItems.Add(new MenuItem("&Refresh"));
            file.MenuItems.Add(new MenuItem("-"));
            file.MenuItems.Add(new MenuItem("&Rename"));
            file.MenuItems.Add(new MenuItem("Exit	Alt+F4"));
            form.Menu = new MainMenu([file]);
            return AccessibilityLint.Check(form).ToList();
        });

        Assert.Contains(failures, f => f.Contains("\"Rename\"") && f.Contains("mnemonic r is also \"Refresh\"'s"));
        Assert.Contains(failures, f => f.Contains("\"Exit\"") && f.Contains("no mnemonic"));
        Assert.Equal(2, failures.Count);
    }

    // The exceptions must be deliberate: a tag on the control, not a quiet
    // drift. Tagged controls pass; untagged ones with the same names fail.
    [Fact]
    public void Tagged_exceptions_are_accepted()
    {
        var failures = Sta.Run(() =>
        {
            using var form = new Form { Text = "Probe" };
            form.Controls.Add(new TextBox { Name = "search", AccessibleName = "Search", Tag = "a11y:unlabelled", TabIndex = 0 });
            form.Controls.Add(new Button { Name = "go", Text = "Go", AccessibleName = "Fetch the forecast", Tag = "a11y:custom-name", TabIndex = 1 });
            return AccessibilityLint.Check(form).ToList();
        });

        Assert.Empty(failures);
    }
}
