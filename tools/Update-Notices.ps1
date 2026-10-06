# Writes THIRD-PARTY-NOTICES.md: the software the Rust app's exe is built
# from, each with its licence text, as those licences ask of a program
# that ships them. The user guide's Software credits link to it, and the
# exe carries it beside the guide (build.rs). Run it again after changing
# the Rust dependencies or wxDragon's version, once a release build has
# downloaded wxWidgets' source (it reads the licence files there), with
# cargo-about installed: cargo install cargo-about --locked --features cli
#
#     powershell -NoProfile -ExecutionPolicy Bypass -File tools\Update-Notices.ps1
param([string]$Out = "THIRD-PARTY-NOTICES.md")

$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent
Set-Location $root
$wx = Join-Path $root "target\release\wxWidgets"
if (-not (Test-Path "$wx\include\wx\version.h")) { throw "No wxWidgets source at $wx; build the app in release first." }
$utf8 = New-Object Text.UTF8Encoding $false

function Read-Text([string]$path) { [IO.File]::ReadAllText((Join-Path $wx $path), $utf8).Replace("`r`n", "`n").Trim() }
function Version-Of([string]$path, [string[]]$names) {
    $text = Read-Text $path
    ($names | ForEach-Object { if ($text -match "#define\s+$_\s+(\d+)") { $Matches[1] } }) -join "."
}
function Fenced([string]$text) { '```' + "`n" + $text + "`n" + '```' }

# libjpeg's README is mostly about the library; its LEGAL ISSUES part is
# the licence, and asks for the line quoted before it.
$jpegReadme = Read-Text "src\jpeg\README"
$jpegLegal = [regex]::Match($jpegReadme, "(?s)LEGAL ISSUES\n=+\n(.*?)\n\n\nThe Unix configuration script").Groups[1].Value.Trim()
if (-not $jpegLegal) { throw "libjpeg's README has changed; find its LEGAL ISSUES part again." }

$wxVersion = Version-Of "include\wx\version.h" "wxMAJOR_VERSION", "wxMINOR_VERSION", "wxRELEASE_NUMBER"
$bundled = @(
    @{ Name = "Expat"; Url = "https://libexpat.github.io/"; Does = "reads XML"; Text = Read-Text "src\expat\expat\COPYING" },
    @{ Name = "libjpeg"; Url = "https://www.ijg.org/"; Does = "reads and writes JPEG images. This software is based in part on the work of the Independent JPEG Group"; Text = $jpegLegal },
    @{ Name = "libpng"; Url = "http://www.libpng.org/pub/png/libpng.html"; Does = "reads and writes PNG images"; Text = Read-Text "src\png\LICENSE" },
    @{ Name = "LibTIFF"; Url = "https://libtiff.gitlab.io/libtiff/"; Does = "reads and writes TIFF images"; Text = Read-Text "src\tiff\LICENSE.md" },
    @{ Name = "NanoSVG"; Url = "https://github.com/memononen/nanosvg"; Does = "draws SVG images, such as the icon in About"; Text = Read-Text "3rdparty\nanosvg\LICENSE.txt" },
    @{ Name = "PCRE2"; Url = "https://pcre2project.github.io/pcre2/"; Does = "matches regular expressions"; Text = Read-Text "3rdparty\pcre\LICENCE.md" },
    @{ Name = "zlib"; Url = "https://zlib.net/"; Does = "compresses data"; Text = Read-Text "src\zlib\LICENSE" }
)

$crates = Join-Path ([IO.Path]::GetTempPath()) "weatherspell-crates.md"
& cargo about generate --locked --fail -c tools/notices/about.toml -m crates/weatherspell/Cargo.toml -o $crates tools/notices/crates.hbs
if ($LASTEXITCODE -ne 0) { throw "cargo-about failed." }
$crateText = [IO.File]::ReadAllText($crates, $utf8).Replace("`r`n", "`n").Trim()
Remove-Item $crates

$parts = @(
    "# Third-party notices",
    "Weatherspell is built with the software below, each under its own licence, and this page gives each licence as it asks. Weatherspell's own licence and the credits for its weather and place data are in the user guide's [Credits and licences](USER_GUIDE.md#credits-and-licences).",
    "This page is written by ``tools/Update-Notices.ps1``.",
    "## wxWidgets",
    "Weatherspell's windows and controls come from [wxWidgets](https://www.wxwidgets.org/) $wxVersion, under the wxWindows Library Licence:",
    (Fenced (Read-Text "docs\licence.txt")),
    "wxWidgets brings these libraries with it, which are built into Weatherspell too."
)
foreach ($b in $bundled) {
    $parts += "### $($b.Name)"
    $parts += "[$($b.Name)]($($b.Url)) $($b.Does)."
    $parts += Fenced $b.Text
}
$parts += "## Rust libraries"
$parts += "Weatherspell is written in Rust, with these libraries from [crates.io](https://crates.io/), grouped by licence. Where a library offers a choice of licences, it's listed under the one Weatherspell uses."
$parts += $crateText

[IO.File]::WriteAllText((Join-Path $root $Out), (($parts -join "`n`n") + "`n"), $utf8)
"Wrote $Out ($((Get-Item (Join-Path $root $Out)).Length) bytes)."
