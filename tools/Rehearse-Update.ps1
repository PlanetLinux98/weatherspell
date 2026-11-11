# Check for Updates, rehearsed with a screen reader before any newer
# release exists: a copy of the built app in a scratch folder, a made-up
# "version 99.0.0" for it to find (the same exe with a few bytes after its
# end, so it runs and its checksum differs), and a small web server on
# this PC only (127.0.0.1) that hands them out. The copy runs on scratch
# data with one made-up location, so the real settings are never touched;
# no copy of Weatherspell may be running already.
#
# It stays running, serving, until the copy is closed (a relaunched copy
# counts) or Ctrl+C. After Relaunch Now the new copy finds "99.0.0" again,
# since it is the same build: that is the rehearsal, not a fault.
#
# The address variable it sets is read only by previews and builds between
# tags (window.rs, update_feed).
#
#     powershell -NoProfile -ExecutionPolicy Bypass -File tools\Rehearse-Update.ps1 [-Slow] [-WrongChecksum] [-Automatic] [-Protected] [-Path target\release\weatherspell.exe] [-Out folder]
#
# -Slow           the download takes about 20 seconds, so its progress is
#                 heard and Cancel can be tried.
# -WrongChecksum  the release's checksum does not match: Install refuses.
# -Automatic      "Check for updates automatically" starts turned on.
# -Protected      the copy goes under Program Files instead, to try
#                 installing where an administrator's permission is
#                 needed. Windows asks for that permission twice: once now,
#                 to put the copy there, and once when Install is chosen.
#                 The folder (Program Files\Weatherspell Rehearsal) stays
#                 afterwards and takes an administrator to remove.
param(
    [switch]$Slow,
    [switch]$WrongChecksum,
    [switch]$Automatic,
    [switch]$Protected,
    [string]$Path = "target\release\weatherspell.exe",
    [string]$Out = (Join-Path $env:TEMP "weatherspell-update-rehearsal"),
    [int]$Port = 8765
)

$ErrorActionPreference = "Stop"
if (Get-Process weatherspell -ErrorAction SilentlyContinue) { throw "Weatherspell is running; close it first." }
$built = (Get-Item $Path).FullName
$utf8 = New-Object Text.UTF8Encoding $false

$serve = Join-Path $Out "serve"
$data = Join-Path $Out "data"
$appFolder = if ($Protected) { Join-Path $env:ProgramFiles "Weatherspell Rehearsal" } else { Join-Path $Out "app" }
$app = Join-Path $appFolder "Weatherspell.exe"
New-Item -ItemType Directory -Force $serve, $data | Out-Null

# The "new version", its checksum list and the release list.
$new = Join-Path $serve "Weatherspell.exe"
Copy-Item $built $new -Force
$tail = [Text.Encoding]::ASCII.GetBytes("`nrehearsal copy`n")
$file = [IO.File]::Open($new, [IO.FileMode]::Append)
$file.Write($tail, 0, $tail.Length)
$file.Close()
$size = (Get-Item $new).Length
$sum = (Get-FileHash $new -Algorithm SHA256).Hash.ToLower()
if ($WrongChecksum) { $sum = "0" * 64 }
[IO.File]::WriteAllText((Join-Path $serve "SHA256SUMS"), "$sum  Weatherspell.exe`n", $utf8)
$notes = "A rehearsal of Check for Updates, not a real release.\r\n\r\n" +
    "### Added\r\n" +
    "- **Check for Updates** in the Help menu shows what's new and installs it (#11).\r\n" +
    "- Settings can check for updates automatically.\r\n\r\n" +
    "### Fixed\r\n" +
    "- Nothing: these lines are here to be read with the arrow keys.\r\n"
$base = "http://127.0.0.1:$Port"
$list = '[{"tag_name":"v99.0.0","draft":false,"prerelease":false,' +
    '"html_url":"https://github.com/PlanetLinux98/weatherspell/releases","body":"' + $notes + '","assets":[' +
    '{"name":"Weatherspell.exe","size":' + $size + ',"browser_download_url":"' + $base + '/Weatherspell.exe"},' +
    '{"name":"SHA256SUMS","size":83,"browser_download_url":"' + $base + '/SHA256SUMS"}]}]'
[IO.File]::WriteAllText((Join-Path $serve "releases.json"), $list, $utf8)

$settings = '{"version":1,"locations":[' +
    '{"name":"Peterborough","region":"Ontario","country":"Canada","latitude":44.30012,"longitude":-78.31623,"nickname":"Home"}' +
    '],"lastLocation":0,"checkUpdatesAutomatically":' + $(if ($Automatic) { "true" } else { "false" }) + '}'
[IO.File]::WriteAllText((Join-Path $data "settings.json"), $settings, $utf8)

if ($Protected) {
    Write-Host "Windows will ask for an administrator's permission to put the copy under Program Files."
    $copy = "New-Item -ItemType Directory -Force '$appFolder' | Out-Null; Copy-Item '$built' '$app' -Force"
    Start-Process powershell -Verb RunAs -Wait -WindowStyle Hidden -ArgumentList "-NoProfile", "-Command", $copy
    if (-not (Test-Path $app)) { throw "The copy was not put under Program Files." }
} else {
    New-Item -ItemType Directory -Force $appFolder | Out-Null
    Copy-Item $built $app -Force
}

function Copies {
    Get-Process | Where-Object { $_.Path -and $_.Path.StartsWith($appFolder, [StringComparison]::OrdinalIgnoreCase) }
}

# One request at a time is plenty: the app asks for the list, the
# checksums, then the file. Each answer closes its connection.
function Answer($client) {
    try {
        $stream = $client.GetStream()
        $stream.ReadTimeout = 5000
        $reader = New-Object IO.StreamReader $stream, ([Text.Encoding]::ASCII), $false, 4096, $true
        $request = $reader.ReadLine()
        while ($true) { $line = $reader.ReadLine(); if ([string]::IsNullOrEmpty($line)) { break } }
        $name = if ($request -match '^GET /([A-Za-z0-9._-]+) ') { $Matches[1] } else { "" }
        $file = Join-Path $serve $name
        if ($name -and (Test-Path $file -PathType Leaf)) {
            $bytes = [IO.File]::ReadAllBytes($file)
            $head = [Text.Encoding]::ASCII.GetBytes("HTTP/1.1 200 OK`r`nContent-Length: $($bytes.Length)`r`nConnection: close`r`n`r`n")
            $stream.Write($head, 0, $head.Length)
            $piece = 256KB
            for ($at = 0; $at -lt $bytes.Length; $at += $piece) {
                $stream.Write($bytes, $at, [Math]::Min($piece, $bytes.Length - $at))
                if ($Slow -and $name -like "*.exe") { Start-Sleep -Milliseconds 300 }
            }
            Write-Host "  sent $name"
        } else {
            $head = [Text.Encoding]::ASCII.GetBytes("HTTP/1.1 404 Not Found`r`nContent-Length: 0`r`nConnection: close`r`n`r`n")
            $stream.Write($head, 0, $head.Length)
        }
    } catch {
        # The app's Cancel closes the connection mid-file: nothing to do.
    } finally {
        $client.Close()
    }
}

$listener = New-Object Net.Sockets.TcpListener ([Net.IPAddress]::Loopback), $Port
$listener.Start()
try {
    $env:WEATHERSPELL_DATA = $data
    $env:WEATHERSPELL_UPDATE_FEED = "$base/releases.json"
    $process = Start-Process $app -PassThru
    $env:WEATHERSPELL_DATA = $null
    $env:WEATHERSPELL_UPDATE_FEED = $null
    Write-Host "Weatherspell's rehearsal copy is running from $appFolder."
    Write-Host "Choose Help, Check for Updates. Close the copy to end the rehearsal."
    $gone = 0
    while ($gone -lt 20) {
        if ($listener.Pending()) {
            Answer $listener.AcceptTcpClient()
            continue
        }
        Start-Sleep -Milliseconds 250
        # A relaunch leaves a moment with no copy: five seconds without one
        # is the end.
        if (Copies) { $gone = 0 } else { $gone++ }
    }
    Write-Host "The rehearsal copy has closed."
}
finally {
    $listener.Stop()
}
