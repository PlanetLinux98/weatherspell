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

# The AppImage's runtime, which linux.yml downloads and appimagetool puts
# in front of the app: its texts are kept in tools/notices/appimage for
# the release named in its release file (tag, then commit), and the
# libraries are the ones its Makefile links (its own LICENSE leaves out
# mimalloc). Versions from the runtime's install-dependencies.sh and
# Alpine's packages for the release it builds on (3.21; zlib's version
# there has moved since, so none is given).
$runtimeDir = Join-Path $root "tools\notices\appimage"
function Read-Runtime([string]$name) { [IO.File]::ReadAllText((Join-Path $runtimeDir $name), $utf8).Replace("`r`n", "`n").Trim() }
$tag, $commit = (Read-Runtime "release") -split " "
$pinned = [regex]::Match([IO.File]::ReadAllText((Join-Path $root ".github\workflows\linux.yml"), $utf8), "type2-runtime/releases/download/([^/]+)/").Groups[1].Value
if ($pinned -ne $tag) { throw "linux.yml uses the AppImage runtime $pinned, but tools\notices\appimage holds $tag's licences; refresh them." }
# The runtime's own licence is its first part; the rest lists libraries.
$runtimeLicence = [regex]::Match((Read-Runtime "type2-runtime-LICENSE"), "(?s)^.*?SOFTWARE\.").Value
$runtimeParts = @(
    @{ Name = "musl"; Url = "https://musl.libc.org/"; Version = "1.2.5"; Does = "is the C library"; File = "musl-COPYRIGHT" },
    @{ Name = "libfuse"; Url = "https://github.com/libfuse/libfuse"; Version = "3.15.0"; Does = "lets the AppImage be mounted as a folder. It is under the GNU Lesser General Public Licence, version 2.1. Its source is at [libfuse 3.15.0](https://github.com/libfuse/libfuse/releases/tag/fuse-3.15.0), and the one change the runtime makes to it, with everything needed to build the runtime again, is in [the runtime's source](https://github.com/AppImage/type2-runtime/tree/$commit). Both are also attached to each Weatherspell release that includes the AppImage, as ``appimage-runtime-source.tar.gz``"; File = "libfuse-LGPL2.txt" },
    @{ Name = "squashfuse"; Url = "https://github.com/vasi/squashfuse"; Version = "0.5.2"; Does = "reads the AppImage's files"; File = "squashfuse-LICENSE" },
    @{ Name = "Zstandard"; Url = "https://facebook.github.io/zstd/"; Version = "1.5.6"; Does = "decompresses them (used under its BSD licence)"; File = "zstd-LICENSE" },
    @{ Name = "zlib"; Url = "https://zlib.net/"; Version = ""; Does = "decompresses them too"; File = "zlib-LICENSE" },
    @{ Name = "mimalloc"; Url = "https://github.com/microsoft/mimalloc"; Version = "2.1.7"; Does = "manages the runtime's memory"; File = "mimalloc-LICENSE" }
)
$parts += "## The AppImage's runtime (Linux)"
$parts += "On Linux, Weatherspell comes as an AppImage, which begins with AppImage's [type2 runtime](https://github.com/AppImage/type2-runtime) (release $tag): a small program, separate from Weatherspell's own, that opens the AppImage and starts the app inside. It is under the MIT licence:"
$parts += Fenced $runtimeLicence
$parts += "The runtime is built with these libraries."
foreach ($r in $runtimeParts) {
    $parts += "### $($r.Name)"
    $parts += ("[$($r.Name)]($($r.Url)) $($r.Version) $($r.Does)." -replace "  ", " ")
    $parts += Fenced (Read-Runtime $r.File)
}

[IO.File]::WriteAllText((Join-Path $root $Out), (($parts -join "`n`n") + "`n"), $utf8)
"Wrote $Out ($((Get-Item (Join-Path $root $Out)).Length) bytes)."
