#!/usr/bin/env bash
# Publishes the disk image built by scripts/make-dmg.sh as GitHub Release v<version>, with its
# SHA-256, after checking the tree is clean, the version isn't tagged yet and the image is signed.
#
# Usage: scripts/release.sh [--unsigned] [--notes-file FILE] [--dry-run]
#   --unsigned    publish dist/Diggr-<version>-unsigned.dmg (the normal case), with the Open
#                 Anyway steps appended to the notes
#   --notes-file  release notes; without it gh asks for them
#   --dry-run     run every check, then stop before tagging
set -euo pipefail
cd "$(dirname "$0")/.."

NAME=Diggr
VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' apps/native/Cargo.toml | head -1)
TAG=v$VERSION
UNSIGNED=0 DRY=0 NOTES=()
while (($#)); do
    case $1 in
        --unsigned) UNSIGNED=1 ;;
        --dry-run) DRY=1 ;;
        --notes-file) NOTES=(--notes-file "$2"); shift ;;
        *) echo "unknown option: $1" >&2; exit 2 ;;
    esac
    shift
done
fail() { echo "release: $*" >&2; exit 1; }

[[ -z "$(git status --porcelain)" ]] || fail "the working tree isn't clean; commit or stash first."
git rev-parse -q --verify "refs/tags/$TAG" >/dev/null && fail "$TAG already exists; bump the version in apps/native/Cargo.toml."
[[ -z "$(git ls-remote --tags origin "$TAG")" ]] || fail "$TAG already exists on origin."

if ((UNSIGNED)); then
    DMG=dist/$NAME-$VERSION-unsigned.dmg
    [[ -f $DMG ]] || fail "$DMG not found; run scripts/make-dmg.sh."
else
    DMG=dist/$NAME-$VERSION.dmg
    [[ -f $DMG ]] || fail "$DMG not found. Signed builds need DEVELOPER_ID and NOTARY_PROFILE; to publish an unsigned build for testing, pass --unsigned."
    spctl --assess --type open --context context:primary-signature "$DMG" ||
        fail "$DMG is not signed and notarized; rebuild it with scripts/make-dmg.sh."
fi

SUM=$DMG.sha256
(cd dist && shasum -a 256 "$(basename "$DMG")") >"$SUM"
echo "Ready: $TAG with $DMG ($(du -h "$DMG" | cut -f1)), sha256 $(cut -d' ' -f1 "$SUM")"
((DRY)) && { echo "Dry run: stopping before tagging."; exit 0; }

git tag -a "$TAG" -m "$NAME $VERSION"
git push origin "$TAG"
if ((UNSIGNED)); then
    notes=dist/$NAME-$VERSION-notes.md
    { [[ ${NOTES[1]:-} ]] && cat "${NOTES[1]}" && echo
      echo "**First open:** this build isn't signed by Apple, so macOS refuses it once. Drag Diggr"
      echo "to Applications, open it, then go to System Settings ▸ Privacy & Security and click"
      echo "**Open Anyway** next to Diggr. It opens normally from then on."; } >"$notes"
    NOTES=(--notes-file "$notes")
fi
gh release create "$TAG" "$DMG" "$SUM" --title "$NAME $VERSION" ${NOTES[@]+"${NOTES[@]}"}
