using Weatherspell.Settings;
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
            using var form = new FindLocationDialog(new OpenMeteoClient());
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
}
