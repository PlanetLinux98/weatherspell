namespace Weatherspell;

// Two standard controls with the classic accessibility Windows gives every
// Win32 edit and combo box, in place of the objects WinForms 4.7.3+ answers
// WM_GETOBJECT with itself. Found with NVDA on 2026-09-14 (NOTES.md):
//
// - NVDA speaks a collapsed combo box's new value from the MSAA value
//   change, but drops MSAA events from any window that advertises a UIA
//   provider unless the window class is on its Win32 list; WinForms'
//   "COMBOBOX" is not, so arrowing through a collapsed combo was silent.
//   Not answering the UIA root request leaves the combo a plain Win32
//   combo box to every screen reader.
// - NVDA reads the text under the mouse pointer through its edit-control
//   support only when it can identify the object under the pointer as the
//   window's client object (IAccIdentity), which WinForms' objects do not
//   implement; it then fell back to the control's name plus its entire
//   value, and a nudge of the mouse read the whole forecast. The standard
//   edit proxy identifies itself, and names the control from the label
//   before it, which every text box here has.
//
// Each is one message on one control; nothing else about them changes.
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

internal sealed class NativeTextBox : TextBox
{
    protected override void WndProc(ref Message m)
    {
        if (m.Msg == Native.WM_GETOBJECT && (int)(long)m.LParam == Native.OBJID_CLIENT)
        {
            DefWndProc(ref m);
            return;
        }
        base.WndProc(ref m);
    }

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

    [System.Runtime.InteropServices.DllImport("user32.dll")]
    public static extern IntPtr SendMessage(IntPtr hWnd, int msg, IntPtr wParam, IntPtr lParam);

    public const int SW_RESTORE = 9;

    [System.Runtime.InteropServices.DllImport("user32.dll")]
    public static extern bool IsIconic(IntPtr hWnd);

    [System.Runtime.InteropServices.DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);

    [System.Runtime.InteropServices.DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);
}
