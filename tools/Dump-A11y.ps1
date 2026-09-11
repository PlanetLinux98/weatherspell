<#
Dumps what a screen reader is handed by a running Weatherspell window.

The MSAA (IAccessible) tree comes first: it is what NVDA and JAWS read for
WinForms 4.8 controls, because WinForms implements IAccessible itself
(AccessibleName, AccessibleRole, value, states). The UI Automation tree
follows for comparison; on this runtime UIA sees WinForms controls only
through the HWND bridge, so it reports less than MSAA does and must not be
taken as the screen-reader view.

    powershell -NoProfile -File tools\Dump-A11y.ps1 [-ProcessName Weatherspell] [-Depth 12] [-NoUia]

Launch the app first. Windows PowerShell 5.1 is enough: oleacc and
UIAutomationClient are part of Windows and .NET Framework.
#>
param(
    [string]$ProcessName = "Weatherspell",
    [int]$Depth = 12,
    [switch]$NoUia
)

$proc = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $proc) {
    Write-Error "No running $ProcessName process found. Launch the app first."
    exit 1
}

# Every visible top-level window of the process: a modal dialog is its own
# window, and it is usually the one under test.
Add-Type -TypeDefinition @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

public static class TopWindows
{
    delegate bool EnumProc(IntPtr hwnd, IntPtr lParam);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc proc, IntPtr lParam);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr hwnd);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowText(IntPtr hwnd, StringBuilder text, int max);

    public static IntPtr[] ForProcess(int pid)
    {
        var found = new List<IntPtr>();
        EnumWindows((hwnd, l) =>
        {
            uint owner;
            GetWindowThreadProcessId(hwnd, out owner);
            if (owner == pid && IsWindowVisible(hwnd)) found.Add(hwnd);
            return true;
        }, IntPtr.Zero);
        return found.ToArray();
    }

    public static string Title(IntPtr hwnd)
    {
        var sb = new StringBuilder(256);
        GetWindowText(hwnd, sb, 256);
        return sb.ToString();
    }
}
"@
$windows = [TopWindows]::ForProcess($proc.Id)
if ($windows.Count -eq 0) {
    Write-Error "$ProcessName is running but has no visible window yet."
    exit 1
}

# The walk lives entirely in C#: an IAccessible handed back to PowerShell
# degrades to a bare __ComObject and cannot be passed into oleacc again.
Add-Type -ReferencedAssemblies Accessibility -TypeDefinition @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
using Accessibility;

public static class Msaa
{
    [DllImport("oleacc.dll")]
    static extern int AccessibleObjectFromWindow(IntPtr hwnd, uint id, ref Guid iid,
        [MarshalAs(UnmanagedType.IUnknown)] out object ppv);

    [DllImport("oleacc.dll")]
    static extern int AccessibleChildren(IAccessible container, int start, int count,
        [Out] object[] children, out int obtained);

    [DllImport("oleacc.dll", CharSet = CharSet.Unicode)]
    static extern uint GetRoleText(uint role, StringBuilder text, uint max);

    [DllImport("oleacc.dll", CharSet = CharSet.Unicode)]
    static extern uint GetStateText(uint state, StringBuilder text, uint max);

    static Guid IID_IAccessible = new Guid("618736E0-3C3D-11CF-810C-00AA00389B71");
    const uint OBJID_WINDOW = 0x00000000;
    const int STATE_SYSTEM_INVISIBLE = 0x8000;

    public static string[] Dump(IntPtr hwnd, int maxDepth)
    {
        object o;
        AccessibleObjectFromWindow(hwnd, OBJID_WINDOW, ref IID_IAccessible, out o);
        var lines = new List<string>();
        Walk((IAccessible)o, 0, 0, maxDepth, lines);
        return lines.ToArray();
    }

    static void Walk(IAccessible acc, object child, int indent, int maxDepth, List<string> lines)
    {
        if (indent > maxDepth) return;
        lines.Add(Describe(acc, child, indent));
        if (!(child is int) || (int)child != 0) return;

        int count;
        try { count = acc.accChildCount; } catch { return; }
        if (count <= 0) return;
        var kids = new object[count];
        int got;
        AccessibleChildren(acc, 0, count, kids, out got);
        for (int i = 0; i < got; i++)
        {
            var kid = kids[i];
            var kidAcc = kid as IAccessible;
            // Invisible subtrees (hidden controls, the frame's own chrome) are
            // skipped the way a screen reader skips them.
            if (kidAcc != null)
            {
                if (IsInvisible(kidAcc, 0)) continue;
                Walk(kidAcc, 0, indent + 1, maxDepth, lines);
            }
            else if (kid is int)
            {
                if (IsInvisible(acc, kid)) continue;
                Walk(acc, kid, indent + 1, maxDepth, lines);
            }
        }
    }

    static bool IsInvisible(IAccessible acc, object child)
    {
        try
        {
            var state = acc.get_accState(child);
            return state is int && ((int)state & STATE_SYSTEM_INVISIBLE) != 0;
        }
        catch { return false; }
    }

    static string Describe(IAccessible acc, object child, int indent)
    {
        var sb = new StringBuilder();
        sb.Append(' ', indent * 2);
        sb.Append(RoleText(Get(() => acc.get_accRole(child))));
        sb.Append(" \"").Append(Str(Get(() => acc.get_accName(child)))).Append('"');

        var value = Str(Get(() => acc.get_accValue(child)));
        if (value.Length > 0)
        {
            value = System.Text.RegularExpressions.Regex.Replace(value, @"\s+", " ").Trim();
            if (value.Length > 80) value = value.Substring(0, 77) + "...";
            sb.Append(" value=\"").Append(value).Append('"');
        }

        var state = StateText(Get(() => acc.get_accState(child)));
        if (state.Length > 0) sb.Append(" [").Append(state).Append(']');

        var key = Str(Get(() => acc.get_accKeyboardShortcut(child)));
        if (key.Length > 0) sb.Append(" key=").Append(key);

        var desc = Str(Get(() => acc.get_accDescription(child)));
        if (desc.Length > 0) sb.Append(" desc=\"").Append(desc).Append('"');

        var help = Str(Get(() => acc.get_accHelp(child)));
        if (help.Length > 0) sb.Append(" help=\"").Append(help).Append('"');

        return sb.ToString();
    }

    static object Get(Func<object> get)
    {
        try { return get(); } catch { return null; }
    }

    static string Str(object o) { return o == null ? "" : o.ToString(); }

    static string RoleText(object role)
    {
        if (!(role is int)) return Str(role);
        var sb = new StringBuilder(128);
        GetRoleText((uint)(int)role, sb, 128);
        return sb.ToString();
    }

    static string StateText(object state)
    {
        if (!(state is int)) return "";
        uint bits = (uint)(int)state;
        var parts = new List<string>();
        for (int i = 0; i < 32; i++)
        {
            uint bit = 1u << i;
            if ((bits & bit) == 0) continue;
            var sb = new StringBuilder(64);
            GetStateText(bit, sb, 64);
            parts.Add(sb.ToString());
        }
        return string.Join(", ", parts.ToArray());
    }
}
"@

foreach ($hwnd in $windows) {
    '== MSAA (what NVDA reads for WinForms controls): "' + [TopWindows]::Title($hwnd) + '" =='
    [Msaa]::Dump($hwnd, $Depth)
    ''
}

if (-not $NoUia) {
    Add-Type -AssemblyName UIAutomationClient
    Add-Type -AssemblyName UIAutomationTypes
    $walker = [System.Windows.Automation.TreeWalker]::ControlViewWalker

    function Format-Uia($el, $indent) {
        $c = $el.Current
        $type = $c.ControlType.ProgrammaticName -replace '^ControlType\.', ''
        $line = ('  ' * $indent) + $type + ' "' + $c.Name + '"'
        if ($c.AccessKey)      { $line += ' key=' + $c.AccessKey }
        if ($c.AcceleratorKey) { $line += ' accel=' + $c.AcceleratorKey }
        if ($c.HelpText)       { $line += ' help="' + $c.HelpText + '"' }
        $flags = @()
        if ($c.IsKeyboardFocusable) { $flags += 'focusable' }
        if (-not $c.IsEnabled)      { $flags += 'disabled' }
        if ($c.HasKeyboardFocus)    { $flags += 'FOCUSED' }
        if ($flags) { $line += ' [' + ($flags -join ', ') + ']' }
        $line
    }

    function Walk-Uia($el, $indent) {
        if ($indent -gt $Depth) { return }
        Format-Uia $el $indent
        $child = $walker.GetFirstChild($el)
        while ($child) {
            Walk-Uia $child ($indent + 1)
            $child = $walker.GetNextSibling($child)
        }
    }

    foreach ($hwnd in $windows) {
        '== UI Automation (HWND bridge view; secondary): "' + [TopWindows]::Title($hwnd) + '" =='
        Walk-Uia ([System.Windows.Automation.AutomationElement]::FromHandle($hwnd)) 0
        ''
    }

    $focused = [System.Windows.Automation.AutomationElement]::FocusedElement
    if ($focused) {
        ''
        'Keyboard focus: ' + (Format-Uia $focused 0).Trim()
    }
}
