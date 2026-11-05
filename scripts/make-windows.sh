#!/usr/bin/env bash
# Builds dist/Diggr-<version>-x86_64-windows.zip on this Mac: Diggr.exe cross-built for Windows 10
# and later, with a README.txt that gives the first-open steps and the GPL as LICENSE.txt.
#
# The exe is built with cargo-xwin (LLVM's clang-cl, lld-link, llvm-lib and llvm-rc, Microsoft's
# CRT fetched once) for x86_64-pc-windows-msvc, with the C runtime linked in (.cargo/config.toml).
# Then the exe is checked: it must be a GUI program (no console window), and every DLL it imports
# must be one Windows provides, so a Windows 10 without the Visual C++ Redistributable runs it.
# The zip must be at most 25 MB.
#
# Needs: `cargo install cargo-xwin --locked`, `rustup target add x86_64-pc-windows-msvc`, and
# LLVM: Homebrew's llvm and lld (`brew install llvm lld`, found without being on the PATH), or,
# where Homebrew doesn't install (Intel Macs), conda-forge's on the PATH (see README).
#
# Usage: scripts/make-windows.sh [--dry-run]   (--dry-run only says what it would build)
#   LLVM_OBJDUMP overrides the llvm-objdump used for the checks (the tests of this script use it).
set -euo pipefail
cd "$(dirname "$0")/.."

NAME=Diggr
VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' apps/native/Cargo.toml | head -1)
TARGET=x86_64-pc-windows-msvc
DIR=$NAME-$VERSION
ZIP=dist/$DIR-x86_64-windows.zip
MAX_BYTES=25000000
EXE=target/$TARGET/release/diggr.exe
FORBIDDEN='^(vcruntime|msvcp|ucrtbase|vcomp|concrt)'

echo "Windows x86_64 build of $NAME $VERSION."
[[ "${1:-}" == "--dry-run" ]] && { echo "Dry run: would write $ZIP."; exit 0; }

if command -v brew >/dev/null 2>&1; then
    for prefix in llvm lld; do
        dir=$(brew --prefix "$prefix" 2>/dev/null) && [[ -d "$dir/bin" ]] && PATH="$dir/bin:$PATH"
    done
fi
export PATH
for tool in clang-cl lld-link llvm-lib llvm-rc; do
    command -v "$tool" >/dev/null || {
        echo "Missing $tool: on Apple Silicon \`brew install llvm lld\`; on Intel, see README, Build a release" >&2
        exit 1
    }
done
cargo xwin --version >/dev/null 2>&1 || { echo "Missing: cargo install cargo-xwin --locked" >&2; exit 1; }
OBJDUMP=${LLVM_OBJDUMP:-llvm-objdump}
command -v "$OBJDUMP" >/dev/null || { echo "Missing llvm-objdump (it comes with LLVM)" >&2; exit 1; }
rustup target add "$TARGET" >/dev/null 2>&1

echo "==> Building $TARGET"
CARGO_PROFILE_RELEASE_STRIP=symbols cargo xwin build --release --locked -p diggr --target "$TARGET"

echo "==> Checking $EXE"
headers=$("$OBJDUMP" -p "$EXE")
imports=$(sed -n 's/^ *DLL Name: //p' <<<"$headers" | sort -u)
bad=$(grep -i -E "$FORBIDDEN" <<<"$imports" || true)
if [[ -n $bad ]]; then
    echo "The exe imports DLLs that a clean Windows 10 may not have:" >&2
    sed 's/^/  /' <<<"$bad" >&2
    echo "Link the C runtime statically (.cargo/config.toml, +crt-static)." >&2
    exit 1
fi
echo "imports check passed: $(wc -l <<<"$imports" | tr -d ' ') DLLs, all provided by Windows."
grep -q "(Windows GUI)" <<<"$headers" ||
    { echo "The exe is not a GUI program: it would open a console window." >&2; exit 1; }
echo "subsystem check passed: Windows GUI."

echo "==> Packing $ZIP"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/$DIR"
cp "$EXE" "$work/$DIR/$NAME.exe"
# Notepad on older Windows 10 builds wants CRLF.
sed 's/$/\r/' packaging/windows/README.txt >"$work/$DIR/README.txt"
sed 's/$/\r/' LICENSE >"$work/$DIR/LICENSE.txt"
mkdir -p dist
rm -f "$ZIP"
(cd "$work" && zip -q -r -9 -X "$OLDPWD/$ZIP" "$DIR")
bytes=$(stat -f%z "$ZIP")
((bytes <= MAX_BYTES)) || { echo "$ZIP is $bytes bytes, over the $MAX_BYTES limit." >&2; rm -f "$ZIP"; exit 1; }
unzip -l "$ZIP"

cat <<EOF
Done: $ZIP ($(du -h "$ZIP" | cut -f1)).

Unsigned: Windows SmartScreen stops the first open of a downloaded copy. Publish it with
scripts/release.sh --unsigned (add --prerelease until it is confirmed on a Windows machine).
To open it: More info, then Run anyway, once.
EOF
