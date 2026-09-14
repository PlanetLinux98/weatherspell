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
}

internal static class Native
{
    public const int WM_GETOBJECT = 0x003D;
    public const int OBJID_CLIENT = -4;
    public const int UiaRootObjectId = -25;
}
