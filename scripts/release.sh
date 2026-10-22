#!/usr/bin/env bash
# Publishes the macOS disk image (scripts/make-dmg.sh) and the Linux tarball
# (scripts/make-linux.sh) as GitHub Release v<version>, with one SHA256SUMS for both, after
# checking the tree is clean, the version isn't tagged yet, both files exist and the image is signed.
#
# Usage: scripts/release.sh [--unsigned] [--notes-file FILE] [--dry-run]
#   --unsigned    publish dist/Diggr-<version>-unsigned.dmg (the normal case), with the macOS
#                 Open Anyway steps appended to the notes
#   --notes-file  release notes, followed by the install steps (without it, only those)
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
TARBALL=dist/$NAME-$VERSION-x86_64-linux.tar.gz
[[ -f $TARBALL ]] || fail "$TARBALL not found; run scripts/make-linux.sh."

SUMS=dist/$NAME-$VERSION-SHA256SUMS
(cd dist && shasum -a 256 "$(basename "$DMG")" "$(basename "$TARBALL")") >"$SUMS"
notes=dist/$NAME-$VERSION-notes.md
{ [[ ${NOTES[1]:-} ]] && cat "${NOTES[1]}" && echo
  if ((UNSIGNED)); then
      echo "**macOS, first open:** this build isn't signed by Apple, so macOS refuses it once. Drag"
      echo "Diggr to Applications, open it, then go to System Settings ▸ Privacy & Security and click"
      echo "**Open Anyway** next to Diggr. It opens normally from then on."
      echo
  fi
  echo "**Linux (x86_64):** extract the archive and run \`./install.sh\` in its folder to add Diggr to"
  echo "your applications menu (or just run \`./diggr\`). It needs ALSA and a Vulkan or OpenGL"
  echo "driver, which desktop distributions have."; } >"$notes"
NOTES=(--notes-file "$notes")
echo "Ready: $TAG with"
for f in "$DMG" "$TARBALL"; do echo "  $f ($(du -h "$f" | cut -f1))"; done
sed 's/^/  sha256 /' "$SUMS"
echo "Notes: $notes"
((DRY)) && { echo "Dry run: stopping before tagging."; exit 0; }

git tag -a "$TAG" -m "$NAME $VERSION"
git push origin "$TAG"
gh release create "$TAG" "$DMG" "$TARBALL" "$SUMS" --title "$NAME $VERSION" "${NOTES[@]}"
