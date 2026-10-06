# Checks that the Rust app is still one exe with nothing beside it (see
# NOTES.md, "One exe"): no DLL or config file in its folder, and nothing
# imported but Windows' own DLLs. The Visual C++ runtime is turned away by
# name, since a build machine has it in System32 like any system DLL.
#
#     powershell -NoProfile -ExecutionPolicy Bypass -File tools\Check-SingleExe.ps1 [-Path target\release\weatherspell.exe]
param([string]$Path = "target\release\weatherspell.exe")

$ErrorActionPreference = "Stop"
$exe = Get-Item $Path

$beside = Get-ChildItem $exe.DirectoryName -File | Where-Object { $_.Extension -in ".dll", ".config" }
if ($beside) { throw "Beside the exe: $($beside.Name -join ', ')" }

$vs = & "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" -latest -products * -property installationPath
$dumpbin = Get-ChildItem "$vs\VC\Tools\MSVC\*\bin\Hostx64\x64\dumpbin.exe" | Select-Object -First 1
if (-not $dumpbin) { throw "dumpbin was not found under $vs" }
$imports = & $dumpbin.FullName /nologo /dependents $exe.FullName |
    ForEach-Object { if ($_ -match '^\s+(\S+\.dll)\s*$') { $Matches[1] } } |
    Sort-Object -Unique

$runtime = '^(vcruntime|msvcp|ucrtbase|concrt|vccorlib|api-ms-win-crt-)'
$foreign = $imports | Where-Object {
    $_ -match $runtime -or -not ($_ -match '^(api|ext)-ms-' -or (Test-Path (Join-Path "$env:SystemRoot\System32" $_)))
}
"$($exe.Name): $([math]::Round($exe.Length / 1MB, 1)) MB, imports $($imports -join ', ')"
if ($foreign) { throw "Not Windows' own: $($foreign -join ', ')" }
"One exe, Windows' own DLLs only."
