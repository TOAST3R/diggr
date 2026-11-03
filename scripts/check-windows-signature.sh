#!/usr/bin/env bash
# Checks that Diggr.exe inside a Windows zip carries a valid Authenticode signature from SignPath
# Foundation (the free code-signing certificate, see https://diggr.es/code-signing/). Run by
# scripts/make-windows.sh --signed and scripts/release.sh. Prints the signer.
#
# Usage: scripts/check-windows-signature.sh <zip>
set -euo pipefail

ZIP=${1:?usage: check-windows-signature.sh <zip>}
command -v osslsigncode >/dev/null || { echo "Missing osslsigncode: brew install osslsigncode" >&2; exit 1; }

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
unzip -q -j "$ZIP" '*/Diggr.exe' -d "$work" || { echo "No Diggr.exe in $ZIP." >&2; exit 1; }
ca=()
# Homebrew's openssl keeps the trusted roots here; osslsigncode's built-in default may not exist.
if prefix=$(brew --prefix 2>/dev/null) && [[ -f $prefix/etc/ca-certificates/cert.pem ]]; then
    ca=(-CAfile "$prefix/etc/ca-certificates/cert.pem")
fi
if ! out=$(osslsigncode verify ${ca[@]+"${ca[@]}"} -in "$work/Diggr.exe" 2>&1); then
    echo "Diggr.exe in $ZIP has no valid signature:" >&2
    sed 's/^/  /' <<<"$out" | tail -5 >&2
    exit 1
fi
signer=$(grep -m1 -E 'Subject:.*SignPath Foundation' <<<"$out" | sed 's/^[[:space:]]*Subject:[[:space:]]*//') ||
    { echo "Diggr.exe in $ZIP is signed, but not by SignPath Foundation." >&2; exit 1; }
echo "signature check passed: $signer"
