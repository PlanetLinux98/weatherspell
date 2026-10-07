# Type-checks the app for the Mac (Apple Silicon) on Windows, with clippy,
# so a Mac compile error shows up here instead of after a push. Nothing is
# built: wxdragon-sys's docs mode (DOCS_RS) skips compiling wxWidgets, and
# ring's C code is checked against clang's own headers plus a stand-in for
# Apple's TargetConditionals.h. Needs the target (rustup target add
# aarch64-apple-darwin) and Visual Studio's clang (the C++ Clang tools).
# Only CI's Mac job proves the app links and runs.

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'

$vs = & "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" -latest -products * -property installationPath
$llvm = Join-Path $vs 'VC\Tools\Llvm\x64'
if (-not (Test-Path "$llvm\bin\clang.exe")) {
    throw "Visual Studio's clang is not installed ($llvm)."
}
$builtins = Get-ChildItem "$llvm\lib\clang\*\include" -Directory | Select-Object -Last 1

$stub = Join-Path $env:TEMP 'weatherspell-mac-stub'
New-Item -ItemType Directory -Force $stub | Out-Null
$conditionals = "#define TARGET_OS_MAC 1`n#define TARGET_OS_OSX 1`n#define TARGET_OS_IPHONE 0`n#define TARGET_OS_IOS 0`n#define TARGET_OS_SIMULATOR 0`n"
[IO.File]::WriteAllText((Join-Path $stub 'TargetConditionals.h'), $conditionals)

$env:DOCS_RS = '1'
$env:CC_aarch64_apple_darwin = "$llvm\bin\clang.exe"
$env:AR_aarch64_apple_darwin = "$llvm\bin\llvm-ar.exe"
# No C library headers for the Mac here; ring has a mode without them.
$env:CFLAGS_aarch64_apple_darwin = "-I$stub -DRING_CORE_NOSTDLIBINC=1"
$env:BINDGEN_EXTRA_CLANG_ARGS_aarch64_apple_darwin = "-I`"$($builtins.FullName)`""
try {
    Push-Location $root
    & $cargo clippy --target aarch64-apple-darwin -p weatherspell -p wx-accessibility --all-targets -- -D warnings
    $code = $LASTEXITCODE
}
finally {
    Pop-Location
    foreach ($name in 'DOCS_RS', 'CC_aarch64_apple_darwin', 'AR_aarch64_apple_darwin',
        'CFLAGS_aarch64_apple_darwin', 'BINDGEN_EXTRA_CLANG_ARGS_aarch64_apple_darwin') {
        Set-Item "env:$name" $null
    }
}
exit $code
