#!/bin/sh
# Type-checks the app for the Mac (Apple silicon) on Linux, with clippy,
# as tools/Check-Mac.ps1 does on Windows, so a Mac compile error shows up
# here instead of after a push. Nothing is built: wxdragon-sys's docs mode
# (DOCS_RS) skips compiling wxWidgets, and ring's C code and the bindings
# are checked against clang's own headers plus a stand-in for Apple's
# TargetConditionals.h. Needs the target (rustup target add
# aarch64-apple-darwin), clang and llvm-ar. Only CI's Mac job proves the
# app links and runs, and runs the Mac's tests.
set -eu
cd "$(dirname "$0")/../.."

builtins=$(clang -print-resource-dir)/include
archiver=$(command -v llvm-ar || ls /usr/bin/llvm-ar-* | sort -V | tail -1)
stub=$(mktemp -d)
trap 'rm -rf "$stub"' EXIT
printf '#define %s\n' 'TARGET_OS_MAC 1' 'TARGET_OS_OSX 1' 'TARGET_OS_IPHONE 0' \
    'TARGET_OS_IOS 0' 'TARGET_OS_SIMULATOR 0' > "$stub/TargetConditionals.h"

# No C library headers for the Mac here, and Linux's must stay out of it:
# ring has a mode without them, and clang's own stand alone when asked.
alone="-ffreestanding -nostdlibinc"
DOCS_RS=1 \
    CC_aarch64_apple_darwin=clang \
    AR_aarch64_apple_darwin=$archiver \
    CFLAGS_aarch64_apple_darwin="-I$stub -DRING_CORE_NOSTDLIBINC=1 $alone" \
    BINDGEN_EXTRA_CLANG_ARGS_aarch64_apple_darwin="-I$builtins $alone" \
    cargo clippy --target aarch64-apple-darwin -p weatherspell -p wx-accessibility \
    --all-targets -- -D warnings
