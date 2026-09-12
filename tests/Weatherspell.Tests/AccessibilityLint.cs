namespace Weatherspell.Tests;

// Mechanical checks for the accessibility rules that can be judged without a
// screen reader. Passing here is necessary, not sufficient: NVDA still has the
// final say on how the window actually reads.
internal static class AccessibilityLint
{
    // Controls that show their own text, which is therefore their name.
    private static readonly Type[] SelfLabelled =
    [
        typeof(Button), typeof(CheckBox), typeof(RadioButton), typeof(LinkLabel),
        typeof(GroupBox), typeof(TabPage),
    ];

    // Controls with no visible text of their own: the name must come from a
    // visible label, and AccessibleName must carry that label's text.
    private static readonly Type[] NeedsLabel =
    [
        typeof(TextBox), typeof(RichTextBox), typeof(MaskedTextBox), typeof(ComboBox),
        typeof(ListBox), typeof(CheckedListBox), typeof(ListView), typeof(TreeView),
        typeof(NumericUpDown), typeof(DateTimePicker), typeof(TrackBar), typeof(DataGridView),
    ];

    // Opt-outs, each a deliberate line in the code with a comment saying why:
    // a control whose name has no visible label, and a control whose name
    // must differ from its visible text.
    private const string UnlabelledTag = "a11y:unlabelled";
    private const string CustomNameTag = "a11y:custom-name";

    public static IEnumerable<string> Check(Form form)
    {
        if (string.IsNullOrWhiteSpace(form.Text))
        {
            yield return "Form has no title text.";
        }

        var labels = AllControls(form).OfType<Label>()
            .Where(l => l is not LinkLabel)
            .Select(l => StripMnemonic(l.Text).TrimEnd(':'))
            .Where(t => t.Length > 0)
            .ToHashSet(StringComparer.Ordinal);

        foreach (var control in AllControls(form))
        {
            var where = Describe(control);
            var type = control.GetType();

            if (SelfLabelled.Any(t => t.IsAssignableFrom(type)))
            {
                var visible = StripMnemonic(control.Text);
                if (visible.Length == 0)
                {
                    yield return $"{where}: no visible text.";
                }
                else if (!string.IsNullOrEmpty(control.AccessibleName)
                         && !control.AccessibleName!.StartsWith(visible, StringComparison.Ordinal)
                         && !HasTag(control, CustomNameTag))
                {
                    yield return $"{where}: AccessibleName \"{control.AccessibleName}\" does not start with the visible text \"{visible}\".";
                }
            }

            if (NeedsLabel.Any(t => t.IsAssignableFrom(type)))
            {
                var name = control.AccessibleName;
                if (string.IsNullOrWhiteSpace(name))
                {
                    yield return $"{where}: no AccessibleName; set it to the visible label's text.";
                }
                else if (!labels.Contains(name!) && !HasTag(control, UnlabelledTag))
                {
                    yield return $"{where}: AccessibleName \"{name}\" matches no visible Label on the form.";
                }
            }
        }

        foreach (var container in AllControls(form).Prepend(form))
        {
            // Not filtered on Visible: it reports false for every control of a
            // form that has never been shown, which is the case under test.
            var stops = container.Controls.Cast<Control>()
                .Where(c => c.TabStop)
                .ToList();
            var duplicates = stops.GroupBy(c => c.TabIndex).Where(g => g.Count() > 1);
            foreach (var group in duplicates)
            {
                var names = string.Join(", ", group.Select(Describe));
                yield return $"{Describe(container)}: tab index {group.Key} is shared by {names}; set an explicit tab order.";
            }
        }

        foreach (var item in AllToolStripItems(form))
        {
            if (item is ToolStripSeparator)
            {
                continue;
            }
            if (string.IsNullOrWhiteSpace(StripMnemonic(item.Text)) && string.IsNullOrWhiteSpace(item.AccessibleName))
            {
                yield return $"ToolStripItem \"{item.Name}\" ({item.GetType().Name}): no text and no AccessibleName.";
            }
        }
    }

    private static bool HasTag(Control control, string tag) =>
        control.Tag is string tags && tags.Contains(tag);

    private static string StripMnemonic(string? text)
    {
        // "&&" is a literal ampersand; a lone "&" marks the mnemonic letter.
        const string literal = "\u0001";
        return (text ?? string.Empty).Replace("&&", literal).Replace("&", string.Empty).Replace(literal, "&").Trim();
    }

    private static string Describe(Control c) =>
        $"{c.GetType().Name} \"{(string.IsNullOrEmpty(c.Name) ? StripMnemonic(c.Text) : c.Name)}\"";

    private static IEnumerable<Control> AllControls(Control root)
    {
        foreach (Control child in root.Controls)
        {
            yield return child;
            foreach (var grandchild in AllControls(child))
            {
                yield return grandchild;
            }
        }
    }

    private static IEnumerable<ToolStripItem> AllToolStripItems(Control root)
    {
        foreach (var strip in AllControls(root).OfType<ToolStrip>())
        {
            foreach (var item in Flatten(strip.Items))
            {
                yield return item;
            }
        }
    }

    private static IEnumerable<ToolStripItem> Flatten(ToolStripItemCollection items)
    {
        foreach (ToolStripItem item in items)
        {
            yield return item;
            if (item is ToolStripDropDownItem dropDown)
            {
                foreach (var nested in Flatten(dropDown.DropDownItems))
                {
                    yield return nested;
                }
            }
        }
    }
}
