#!/usr/bin/env bash
# Packs Diggr.exe into <out.zip>: one folder, named after the zip without -x86_64-windows.zip
# (Diggr-<version>), holding Diggr.exe, README.txt (the first-open steps) and the GPL as
# LICENSE.txt, both text files with CRLF. Fails, leaving no zip, when it is over 25 MB.
# Run by scripts/make-windows.sh on the Mac and by .github/workflows/windows-signed.yml on Windows
# (Git Bash, where 7-Zip stands in for zip), so both zips are the same.
#
# Usage: scripts/pack-windows-zip.sh <exe> <out.zip>
set -euo pipefail

EXE=${1:?usage: pack-windows-zip.sh <exe> <out.zip>}
ZIP=${2:?usage: pack-windows-zip.sh <exe> <out.zip>}
ROOT=$(cd "$(dirname "$0")/.." && pwd)
DIR=$(basename "$ZIP" -x86_64-windows.zip)
MAX_BYTES=25000000

echo "==> Packing $ZIP"
mkdir -p "$(dirname "$ZIP")"
ZIP=$(cd "$(dirname "$ZIP")" && pwd)/$(basename "$ZIP")
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/$DIR"
cp "$EXE" "$work/$DIR/Diggr.exe"
# Notepad on older Windows 10 builds wants CRLF. tr first: a Windows checkout may already have it.
tr -d '\r' <"$ROOT/packaging/windows/README.txt" | sed 's/$/\r/' >"$work/$DIR/README.txt"
tr -d '\r' <"$ROOT/LICENSE" | sed 's/$/\r/' >"$work/$DIR/LICENSE.txt"
rm -f "$ZIP"
if command -v zip >/dev/null; then
    (cd "$work" && zip -q -r -9 -X "$ZIP" "$DIR")
else
    (cd "$work" && 7z a -bd -bso0 -tzip -mx=9 "$ZIP" "$DIR")
fi
bytes=$(wc -c <"$ZIP" | tr -d ' ')
((bytes <= MAX_BYTES)) || { echo "$ZIP is $bytes bytes, over the $MAX_BYTES limit." >&2; rm -f "$ZIP"; exit 1; }
echo "packed: $bytes bytes."
