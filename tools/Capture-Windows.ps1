# Pictures of each window of the Rust app at a large font, as a large
# Windows Text size gives it, to check that nothing is cut off or pushed
# out of its window (0.1's Show-Scaled did the same for the C# app). The
# app runs offline with two made-up locations in a scratch folder, so the
# real settings are never touched; no copy of it may be running already.
#
#     powershell -NoProfile -ExecutionPolicy Bypass -File tools\Capture-Windows.ps1 [-PointSize 18] [-Path target\release\weatherspell.exe] [-Out folder]
param(
    [int]$PointSize = 18,
    [string]$Path = "target\release\weatherspell.exe",
    [string]$Out = (Join-Path $env:TEMP "weatherspell-capture")
)

$ErrorActionPreference = "Stop"
if (Get-Process weatherspell -ErrorAction SilentlyContinue) { throw "Weatherspell is running; close it first." }
$app = (Get-Item $Path).FullName

Add-Type -AssemblyName System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public static class CaptureWin32 {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, int m, IntPtr w, IntPtr l);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowEx(IntPtr parent, IntPtr after, string cls, string title);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, System.Text.StringBuilder s, int max);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    public delegate bool EnumProc(IntPtr h, IntPtr l);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc p, IntPtr l);
    // The visible top-level windows of one process, by title.
    public static System.Collections.Generic.Dictionary<string, IntPtr> Windows(uint pid) {
        var found = new System.Collections.Generic.Dictionary<string, IntPtr>();
        EnumWindows((h, l) => {
            uint p; GetWindowThreadProcessId(h, out p);
            if (p == pid && IsWindowVisible(h)) {
                var title = new System.Text.StringBuilder(256); GetWindowText(h, title, 256);
                found[title.ToString()] = h;
            }
            return true;
        }, IntPtr.Zero);
        return found;
    }
}
"@
[void][CaptureWin32]::SetProcessDPIAware()

$data = Join-Path $Out "data"
New-Item -ItemType Directory -Force $data | Out-Null
$settings = '{"version":1,"locations":[' +
    '{"name":"Peterborough","region":"Ontario","country":"Canada","latitude":44.30012,"longitude":-78.31623,"nickname":"Home"},' +
    '{"name":"Saint-Pierre-de-la-Riviere-du-Sud","region":"Quebec","country":"Canada","latitude":46.92,"longitude":-70.68}' +
    '],"lastLocation":0}'
[IO.File]::WriteAllText((Join-Path $data "settings.json"), $settings, (New-Object Text.UTF8Encoding $false))

function Find-Window([string]$title) {
    for ($i = 0; $i -lt 50; $i++) {
        $windows = [CaptureWin32]::Windows([uint32]$process.Id)
        if ($windows.ContainsKey($title)) { Start-Sleep -Milliseconds 400; return $windows[$title] }
        Start-Sleep -Milliseconds 100
    }
    throw "No window titled $title appeared."
}

# The window as it draws itself, whatever covers it on the screen, so
# nothing has to come to the front.
function Save-Picture([IntPtr]$window, [string]$name) {
    $r = New-Object CaptureWin32+RECT
    [void][CaptureWin32]::GetWindowRect($window, [ref]$r)
    $bitmap = New-Object Drawing.Bitmap ($r.R - $r.L), ($r.B - $r.T)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $dc = $graphics.GetHdc()
    # PW_RENDERFULLCONTENT
    [void][CaptureWin32]::PrintWindow($window, $dc, 2)
    $graphics.ReleaseHdc($dc)
    $file = Join-Path $Out "$name.png"
    $bitmap.Save($file)
    $graphics.Dispose(); $bitmap.Dispose()
    $file
}

function Close-Window([IntPtr]$window) {
    [void][CaptureWin32]::PostMessage($window, 0x10, [IntPtr]::Zero, [IntPtr]::Zero)
    Start-Sleep -Milliseconds 500
}

$env:WEATHERSPELL_DATA = $data
$env:WEATHERSPELL_OFFLINE = "1"
$env:WEATHERSPELL_FONT_POINTS = "$PointSize"
$process = Start-Process $app -PassThru
$env:WEATHERSPELL_DATA = $null
$env:WEATHERSPELL_OFFLINE = $null
$env:WEATHERSPELL_FONT_POINTS = $null
try {
    $frame = Find-Window "Weatherspell Preview"
    Start-Sleep -Seconds 2
    Save-Picture $frame "main"
    # The menu commands' ids (src/window.rs), and the dialog each opens.
    foreach ($dialog in @(
            @{ Id = 6006; Title = "Add Location"; Name = "add-location" },
            @{ Id = 5022; Title = "Settings"; Name = "settings" },
            @{ Id = 5014; Title = "About Weatherspell"; Name = "about" })) {
        [void][CaptureWin32]::PostMessage($frame, 0x111, [IntPtr]$dialog.Id, [IntPtr]::Zero)
        $window = Find-Window $dialog.Title
        Save-Picture $window $dialog.Name
        Close-Window $window
    }
    [void][CaptureWin32]::PostMessage($frame, 0x111, [IntPtr]6005, [IntPtr]::Zero)
    $manage = Find-Window "Manage Locations"
    Save-Picture $manage "manage-locations"
    # F2 in the list: Edit Location for the first location.
    $list = [CaptureWin32]::FindWindowEx($manage, [IntPtr]::Zero, "ListBox", [NullString]::Value)
    [void][CaptureWin32]::PostMessage($list, 0x100, [IntPtr]0x71, [IntPtr]1)
    $edit = Find-Window "Edit Location"
    Save-Picture $edit "edit-location"
    Close-Window $edit
    Close-Window $manage
}
finally {
    # Its own copy, on the scratch folder: every window closed, dialogs
    # and all, until it has gone.
    for ($i = 0; $i -lt 10 -and -not $process.HasExited; $i++) {
        foreach ($window in [CaptureWin32]::Windows([uint32]$process.Id).Values) { Close-Window $window }
    }
    if (-not $process.HasExited) { Write-Warning "The copy it started (process $($process.Id)) did not close." }
}
