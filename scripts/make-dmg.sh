#!/usr/bin/env bash
# Builds dist/Diggr.app (one universal binary) and its drag-to-install disk image.
#
# With DEVELOPER_ID ("Developer ID Application: … (TEAMID)") and NOTARY_PROFILE (a keychain
# profile from `xcrun notarytool store-credentials`) set, the app and the image are signed with
# the hardened runtime, notarized and stapled (needs a paid Apple account). Without them, the
# normal case, the app is signed ad hoc and the image is named …-unsigned.dmg.
#
# Usage: scripts/make-dmg.sh [--dry-run]   (--dry-run only says which path it would take)
set -euo pipefail
cd "$(dirname "$0")/.."

NAME=Diggr
VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' apps/native/Cargo.toml | head -1)
APP=dist/$NAME.app
TARGETS=(aarch64-apple-darwin x86_64-apple-darwin)

if [[ -n "${DEVELOPER_ID:-}" && -n "${NOTARY_PROFILE:-}" ]]; then
    SIGNED=1
    DMG=dist/$NAME-$VERSION.dmg
    echo "Signed build of $NAME $VERSION with \"$DEVELOPER_ID\", notarized with profile \"$NOTARY_PROFILE\"."
else
    SIGNED=0
    DMG=dist/$NAME-$VERSION-unsigned.dmg
    missing=()
    [[ -z "${DEVELOPER_ID:-}" ]] && missing+=(DEVELOPER_ID)
    [[ -z "${NOTARY_PROFILE:-}" ]] && missing+=(NOTARY_PROFILE)
    echo "Unsigned build of $NAME $VERSION (${missing[*]} not set)."
fi
[[ "${1:-}" == "--dry-run" ]] && { echo "Dry run: would write $APP and $DMG."; exit 0; }

# Fails the build unless Apple accepted the submission (notarytool exits 0 for "Invalid" too).
notarize() {
    local log
    log=$(xcrun notarytool submit "$1" --keychain-profile "$NOTARY_PROFILE" --wait)
    echo "$log"
    grep -q "status: Accepted" <<<"$log" || { echo "Notarization of $1 failed." >&2; exit 1; }
    xcrun stapler staple "$2"
}

echo "==> Building ${TARGETS[*]}"
export MACOSX_DEPLOYMENT_TARGET=11.0 CARGO_PROFILE_RELEASE_STRIP=symbols
slices=()
for target in "${TARGETS[@]}"; do
    rustup target add "$target" >/dev/null 2>&1
    cargo build --release --locked -p diggr --target "$target"
    slices+=("target/$target/release/diggr")
done

echo "==> Bundling $APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
lipo -create "${slices[@]}" -output "$APP/Contents/MacOS/diggr"
sed -e "s/@NAME@/$NAME/g" -e "s/@VERSION@/$VERSION/g" packaging/macos/Info.plist.in >"$APP/Contents/Info.plist"
plutil -lint -s "$APP/Contents/Info.plist"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
iconset=$work/AppIcon.iconset
mkdir "$iconset"
for size in 16 32 128 256 512; do
    sips -z $size $size assets/icon/icon-1024.png --out "$iconset/icon_${size}x${size}.png" >/dev/null
    sips -z $((size * 2)) $((size * 2)) assets/icon/icon-1024.png --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$APP/Contents/Resources/AppIcon.icns"

if ((SIGNED)); then
    echo "==> Signing and notarizing $APP"
    codesign --force --options runtime --timestamp --sign "$DEVELOPER_ID" "$APP/Contents/MacOS/diggr"
    codesign --force --options runtime --timestamp --sign "$DEVELOPER_ID" "$APP"
    ditto -c -k --keepParent "$APP" "$work/$NAME.zip"
    notarize "$work/$NAME.zip" "$APP"
else
    # Apple Silicon runs nothing without at least an ad-hoc signature.
    codesign --force --sign - "$APP"
fi

echo "==> Making $DMG"
mkdir "$work/stage"
cp -R "$APP" "$work/stage/"
ln -s /Applications "$work/stage/Applications"
rm -f "$DMG"
# LZMA (opens on macOS 10.15+; the app needs 11): about 17 MB, where zlib's UDZO is about 25.
hdiutil create -volname "$NAME" -srcfolder "$work/stage" -format ULMO -ov "$DMG" >/dev/null

if ((SIGNED)); then
    codesign --timestamp --sign "$DEVELOPER_ID" "$DMG"
    notarize "$DMG" "$DMG"
    spctl --assess --type execute -v "$APP"
    spctl --assess --type open --context context:primary-signature -v "$DMG"
    echo "Done: $DMG ($(du -h "$DMG" | cut -f1)), signed and notarized."
else
    cat <<EOF
Done: $DMG ($(du -h "$DMG" | cut -f1)).

Unsigned: publish it with scripts/release.sh --unsigned. To open it on another Mac: drag
Diggr to Applications, open it once (macOS refuses), then System Settings ▸ Privacy &
Security ▸ "Open Anyway" next to Diggr.
EOF
fi
