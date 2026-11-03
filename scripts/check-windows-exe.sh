#!/usr/bin/env bash
# Checks a built Diggr.exe: it must be a GUI program (no console window), and every DLL it imports
# must be one Windows provides, so a Windows 10 without the Visual C++ Redistributable runs it.
# Run by scripts/make-windows.sh on the Mac and by .github/workflows/windows-signed.yml on Windows
# (Git Bash), so both builds pass the same checks.
#
# Usage: scripts/check-windows-exe.sh <exe>
#   LLVM_OBJDUMP overrides the llvm-objdump used (default: the one on the PATH).
set -euo pipefail

EXE=${1:?usage: check-windows-exe.sh <exe>}
FORBIDDEN='^(vcruntime|msvcp|ucrtbase|vcomp|concrt)'
OBJDUMP=${LLVM_OBJDUMP:-llvm-objdump}
command -v "$OBJDUMP" >/dev/null || { echo "Missing llvm-objdump: brew install llvm" >&2; exit 1; }

echo "==> Checking $EXE"
# tr: llvm-objdump on Windows may end lines with CRLF.
headers=$("$OBJDUMP" -p "$EXE" | tr -d '\r')
imports=$(sed -n 's/^ *DLL Name: //p' <<<"$headers" | sort -u)
[[ -n $imports ]] || { echo "No imports found in $EXE: is it a Windows exe?" >&2; exit 1; }
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
