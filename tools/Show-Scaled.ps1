<#
Shows each form at a larger font, as the Windows "Text size" accessibility
setting would, and screenshots it: a check that layouts grow with the text
instead of clipping it. Loads the built exe as an assembly (the forms are
internal, so reflection reaches them) with WEATHERSPELL_FONT_POINTS set, so
the forms are built with the large font from the start; no display setting
is changed.

    powershell -NoProfile -ExecutionPolicy Bypass -File tools\Show-Scaled.ps1 [-PointSize 14] [-OutDir <folder>]

The main form is built against a temporary settings file with one location,
so it fetches a real forecast while shown.
#>
param(
    [float]$PointSize = 14,
    [string]$OutDir = (Join-Path $env:TEMP "weatherspell-scaled"),
    [string]$Exe = (Join-Path $PSScriptRoot "..\src\Weatherspell\bin\Debug\net48\Weatherspell.exe")
)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public static class ScaledWin {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
}
"@
[ScaledWin]::SetProcessDPIAware() | Out-Null
New-Item -ItemType Directory -Force $OutDir | Out-Null

# The forms read this at construction (Scaling.DeveloperFont), which is
# when a real Text size setting would already be in effect; setting the font
# afterwards would force a re-layout and hide measurement bugs.
$env:WEATHERSPELL_FONT_POINTS = $PointSize
$asm = [System.Reflection.Assembly]::LoadFrom((Resolve-Path $Exe))

function Snap($form, $name, $seconds) {
    $form.Show()
    $end = (Get-Date).AddSeconds($seconds)
    while ((Get-Date) -lt $end) { [System.Windows.Forms.Application]::DoEvents(); Start-Sleep -Milliseconds 50 }
    $r = New-Object ScaledWin+RECT
    [ScaledWin]::GetWindowRect($form.Handle, [ref]$r) | Out-Null
    $bmp = New-Object System.Drawing.Bitmap ($r.R - $r.L), ($r.B - $r.T)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($r.L, $r.T, 0, 0, $bmp.Size)
    $g.Dispose()
    $path = Join-Path $OutDir "$name-$PointSize.png"
    $bmp.Save($path)
    $form.Close()
    "$name at $PointSize pt: $($r.R - $r.L)x$($r.B - $r.T) -> $path"
}

# Constructors are invoked explicitly: PowerShell's overload resolution for
# Activator.CreateInstance wraps a string argument the wrong way.
# ($args would shadow PowerShell's automatic variable, hence ctorArgs.)
function New-Internal($typeName, [object[]]$ctorArgs) {
    $type = $asm.GetType($typeName)
    if ($ctorArgs.Count -eq 0) { return [Activator]::CreateInstance($type, $true) }
    $types = [type[]]($ctorArgs | ForEach-Object { $_.GetType() })
    $ctor = $type.GetConstructor([System.Reflection.BindingFlags]"Public,NonPublic,Instance", $null, $types, $null)
    return $ctor.Invoke($ctorArgs)
}

$client = New-Internal "Weatherspell.Weather.OpenMeteo.OpenMeteoClient" @()
$search = New-Internal "Weatherspell.Weather.LocationSearch" @($client)
$dialog = New-Internal "Weatherspell.FindLocationDialog" @($search)
Snap $dialog "find-location" 2

# The alert dialog takes a list of paragraphs; the single constructor is
# invoked directly since a string[] is not the exact parameter type.
$alertCtor = $asm.GetType("Weatherspell.AlertDialog").GetConstructors([System.Reflection.BindingFlags]"Public,NonPublic,Instance")[0]
$alertText = [string[]]@(
    "Frost advisory from Environment Canada, in effect until 6:30 am tomorrow.",
    "Area: Gander and vicinity.",
    "Issued 10:35 pm today.",
    "Yellow level, moderate impact, high confidence.",
    "Areas of frost are expected.",
    "Locations: Deer Lake - Humber Valley, Buchans and the interior, Grand-Falls-Windsor and vicinity, Green Bay - White Bay, Gander and vicinity and Terra Nova.")
$alert = $alertCtor.Invoke([object[]]@("Frost advisory", $alertText, "https://weather.gc.ca/"))
Snap $alert "alert" 2

$settingsPath = Join-Path $OutDir "settings.json"
'{"version":1,"lastLocation":0,"locations":[{"name":"Toronto","region":"Ontario","country":"Canada","latitude":43.65,"longitude":-79.38,"timeZoneId":"America/Toronto","notifyAlerts":true}]}' |
    Set-Content $settingsPath -Encoding UTF8
$store = New-Internal "Weatherspell.Settings.SettingsStore" @([string]$settingsPath)
$main = New-Internal "Weatherspell.MainForm" @($store)
Snap $main "main" 8
