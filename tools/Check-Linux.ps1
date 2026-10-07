# Type-checks the app for Linux (x86_64, GTK) on Windows, with clippy, so
# a Linux compile error shows up here instead of after a push. Nothing is
# built: wxdragon-sys's docs mode (DOCS_RS) skips compiling wxWidgets, and
# ring's C code is checked against clang's own headers only. Needs the
# target (rustup target add x86_64-unknown-linux-gnu) and Visual Studio's
# clang (the C++ Clang tools). Only a build on Linux proves the app links
# and runs.

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'

$vs = & "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" -latest -products * -property installationPath
$llvm = Join-Path $vs 'VC\Tools\Llvm\x64'
if (-not (Test-Path "$llvm\bin\clang.exe")) {
    throw "Visual Studio's clang is not installed ($llvm)."
}
$builtins = Get-ChildItem "$llvm\lib\clang\*\include" -Directory | Select-Object -Last 1

$env:DOCS_RS = '1'
$env:CC_x86_64_unknown_linux_gnu = "$llvm\bin\clang.exe"
$env:AR_x86_64_unknown_linux_gnu = "$llvm\bin\llvm-ar.exe"
# No C library headers for Linux here; ring has a mode without them. Its
# x86 intrinsics pull in mm_malloc.h, which wants stdlib.h and which ring
# never uses: its include guard keeps it out.
$env:CFLAGS_x86_64_unknown_linux_gnu = '-DRING_CORE_NOSTDLIBINC=1 -D__MM_MALLOC_H'
$env:BINDGEN_EXTRA_CLANG_ARGS_x86_64_unknown_linux_gnu = "-I`"$($builtins.FullName)`""
try {
    Push-Location $root
    & $cargo clippy --target x86_64-unknown-linux-gnu -p weatherspell -p wx-accessibility --all-targets -- -D warnings
    $code = $LASTEXITCODE
}
finally {
    Pop-Location
    foreach ($name in 'DOCS_RS', 'CC_x86_64_unknown_linux_gnu', 'AR_x86_64_unknown_linux_gnu',
        'CFLAGS_x86_64_unknown_linux_gnu', 'BINDGEN_EXTRA_CLANG_ARGS_x86_64_unknown_linux_gnu') {
        Set-Item "env:$name" $null
    }
}
exit $code
