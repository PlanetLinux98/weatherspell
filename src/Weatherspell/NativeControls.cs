namespace Weatherspell;

// Two controls with the accessibility Windows gives them itself, in place
// of the objects WinForms 4.7.3+ answers WM_GETOBJECT with. Found with NVDA
// on 2026-09-14 and Narrator on 2026-09-25 (NOTES.md):
//
// - NVDA speaks a collapsed combo box's new value from the MSAA value
//   change, but drops MSAA events from any window that advertises a UIA
//   provider unless the window class is on its Win32 list; WinForms'
//   "COMBOBOX" is not, so arrowing through a collapsed combo was silent.
//   Not answering the UIA root request leaves the combo a plain Win32
//   combo box to every screen reader.
// - The text to be read is a ReadingBox, below.
internal sealed class NativeComboBox : ComboBox
{
    protected override void WndProc(ref Message m)
    {
        if (m.Msg == Native.WM_GETOBJECT && (int)(long)m.LParam == Native.UiaRootObjectId)
        {
            m.Result = IntPtr.Zero;
            return;
        }
        base.WndProc(ref m);
    }
}

// The text a user reads, the forecast and an alert's details: read-only
// plain text in a RichTextBox on msftedit's RICHEDIT50W rather than a
// TextBox (#23). On this runtime a WinForms TextBox reaches UI Automation
// only as an edit with a value and no text pattern, so Narrator read the
// whole forecast on focus and nothing on the arrows; RICHEDIT50W answers
// UI Automation itself as a document with a text pattern, while NVDA reads
// it exactly as it read the TextBox. msftedit.dll is part of Windows, and
// RICHEDIT50W is what .NET 10's RichTextBox uses by default. WinForms'
// own RichTextBox here is RichEdit20W, which is no better than the TextBox.
//
// RichEdit counts a line break as one character, so text for it is built
// with "\n" (SectionLayout), and offsets match TextLength and the caret.
internal sealed class ReadingBox : RichTextBox
{
    private static readonly bool Msftedit = Native.LoadLibrary("msftedit.dll") != IntPtr.Zero;

    public ReadingBox()
    {
        ReadOnly = true;
        // "open-meteo.com" in the Sources line is not a link to follow.
        DetectUrls = false;
        ScrollBars = RichTextBoxScrollBars.Vertical;
        WordWrap = true;
        // RichEdit has no menu of its own, where an edit control offers Copy
        // and Select All on a right click, Shift+F10 or the Applications key.
        var copy = new MenuItem("&Copy\tCtrl+C", (_, _) => Copy());
        var selectAll = new MenuItem("Select &All\tCtrl+A", (_, _) => SelectAll());
        ContextMenu = new ContextMenu([copy, selectAll]);
        ContextMenu.Popup += (_, _) => copy.Enabled = SelectionLength > 0;
    }

    protected override CreateParams CreateParams
    {
        get
        {
            var cp = base.CreateParams;
            if (Msftedit) cp.ClassName = "RICHEDIT50W";
            return cp;
        }
    }

    // WinForms' own MSAA object for the box cannot identify itself to NVDA
    // (IAccIdentity), so a nudge of the mouse read the whole text; the
    // control's own answer does, and NVDA reads the paragraph under the
    // pointer (NOTES.md). UI Automation's request still reaches RichEdit.
    protected override void WndProc(ref Message m)
    {
        if (m.Msg == Native.WM_GETOBJECT && (int)(long)m.LParam == Native.OBJID_CLIENT)
        {
            DefWndProc(ref m);
            return;
        }
        base.WndProc(ref m);
    }

    // RichEdit sets text against the border, where the edit control left a
    // little room; a quarter of a line on each side, so it scales with the
    // font.
    protected override void OnHandleCreated(EventArgs e)
    {
        base.OnHandleCreated(e);
        var margin = Math.Max(2, Font.Height / 4);
        Native.SendMessage(Handle, Native.EM_SETMARGINS, (IntPtr)(Native.EC_LEFTMARGIN | Native.EC_RIGHTMARGIN), (IntPtr)(margin | (margin << 16)));
    }

    // Select All and Copy here rather than in RichEdit: after Shift+F10
    // opens the menu, RichEdit never sees Shift let go and reads the next
    // Ctrl+A as Ctrl+Shift+A, so the keys did nothing until Shift was
    // pressed again. The key data here is the keyboard's real state.
    protected override bool ProcessCmdKey(ref Message msg, Keys keyData)
    {
        switch (keyData)
        {
            case Keys.Control | Keys.A:
                SelectAll();
                return true;
            case Keys.Control | Keys.C:
            case Keys.Control | Keys.Insert:
                if (SelectionLength > 0) Copy();
                return true;
        }
        return base.ProcessCmdKey(ref msg, keyData);
    }

    // Enter belongs to the form: it opens an alert's details in the
    // forecast and closes the details dialog, as it did in the TextBox.
    protected override bool IsInputKey(Keys keyData) =>
        (keyData & Keys.KeyCode) != Keys.Enter && base.IsInputKey(keyData);

    // New text with the caret at the given offset and the view where the
    // reader left it: setting Text scrolls to the top, so the first visible
    // line is put back, and the caret is brought into view only if it was
    // in view before (someone reading with the mouse wheel stays put).
    public void ReplaceText(string text, int caret)
    {
        var top = FirstVisibleLine;
        var caretLine = GetLineFromCharIndex(SelectionStart);
        var caretShown = caretLine >= top && caretLine < top + VisibleLines;
        Text = text;
        SelectionStart = Math.Min(Math.Max(caret, 0), TextLength);
        SelectionLength = 0;
        var scroll = top - FirstVisibleLine;
        if (scroll != 0) Native.SendMessage(Handle, Native.EM_LINESCROLL, IntPtr.Zero, (IntPtr)scroll);
        if (caretShown) ScrollToCaret();
    }

    private int FirstVisibleLine => (int)Native.SendMessage(Handle, Native.EM_GETFIRSTVISIBLELINE, IntPtr.Zero, IntPtr.Zero);

    private int VisibleLines => Math.Max(1, ClientSize.Height / Math.Max(1, Font.Height));
}

internal static class Native
{
    public const int WM_GETOBJECT = 0x003D;
    public const int OBJID_CLIENT = -4;
    public const int UiaRootObjectId = -25;
    public const int EM_GETFIRSTVISIBLELINE = 0x00CE;
    public const int EM_LINESCROLL = 0x00B6;
    public const int EM_SETMARGINS = 0x00D3;
    public const int EC_LEFTMARGIN = 1;
    public const int EC_RIGHTMARGIN = 2;

    [System.Runtime.InteropServices.DllImport("user32.dll")]
    public static extern IntPtr SendMessage(IntPtr hWnd, int msg, IntPtr wParam, IntPtr lParam);

    [System.Runtime.InteropServices.DllImport("kernel32.dll", CharSet = System.Runtime.InteropServices.CharSet.Unicode)]
    public static extern IntPtr LoadLibrary(string name);

    public const int SW_RESTORE = 9;

    [System.Runtime.InteropServices.DllImport("user32.dll")]
    public static extern bool IsIconic(IntPtr hWnd);

    [System.Runtime.InteropServices.DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);

    [System.Runtime.InteropServices.DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);
}
