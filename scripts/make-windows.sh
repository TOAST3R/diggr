#!/usr/bin/env bash
# Builds dist/Diggr-<version>-x86_64-windows.zip: Diggr.exe for Windows 10 and later, with a
# README.txt that gives the first-open steps and the GPL as LICENSE.txt.
#
# Without --signed, the exe is cross-built on this Mac with cargo-xwin (clang-cl and lld from
# Homebrew's llvm and lld, Microsoft's CRT fetched once) for x86_64-pc-windows-msvc, with the C
# runtime linked in (.cargo/config.toml), checked by scripts/check-windows-exe.sh (GUI program,
# only DLLs Windows provides) and packed by scripts/pack-windows-zip.sh. The zip is unsigned.
#
# With --signed, the zip comes from .github/workflows/windows-signed.yml instead, run for HEAD:
# SignPath signs only builds made by the project's own CI. HEAD must be pushed; the script starts
# the run, waits for it (and for the signing request to be approved in SignPath), downloads the
# signed zip and checks its signature with scripts/check-windows-signature.sh.
#
# Needs: `cargo install cargo-xwin --locked`, `brew install llvm lld`,
# `rustup target add x86_64-pc-windows-msvc`; with --signed only `gh` and
# `brew install osslsigncode`.
#
# Usage: scripts/make-windows.sh [--signed] [--dry-run]   (--dry-run only says what it would build)
#   LLVM_OBJDUMP overrides the llvm-objdump used for the checks.
set -euo pipefail
cd "$(dirname "$0")/.."

NAME=Diggr
VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' apps/native/Cargo.toml | head -1)
TARGET=x86_64-pc-windows-msvc
DIR=$NAME-$VERSION
ZIP=dist/$DIR-x86_64-windows.zip
EXE=target/$TARGET/release/diggr.exe
WORKFLOW=windows-signed.yml
SIGNED=0 DRY=0
while (($#)); do
    case $1 in
        --signed) SIGNED=1 ;;
        --dry-run) DRY=1 ;;
        *) echo "unknown option: $1" >&2; exit 2 ;;
    esac
    shift
done
fail() { echo "make-windows: $*" >&2; exit 1; }

if ((SIGNED)); then
    echo "Signed Windows x86_64 build of $NAME $VERSION, from CI."
    ((DRY)) && { echo "Dry run: would fetch the signed zip for $(git rev-parse --short HEAD) into $ZIP."; exit 0; }
    command -v gh >/dev/null || fail "missing gh: brew install gh"
    command -v osslsigncode >/dev/null || fail "missing osslsigncode: brew install osslsigncode"
    head=$(git rev-parse HEAD)
    branch=$(git symbolic-ref --short -q HEAD) || fail "HEAD is detached; check out the branch to build."
    [[ "$(git ls-remote origin "refs/heads/$branch" | cut -f1)" == "$head" ]] ||
        fail "HEAD isn't pushed: git push origin $branch, then run this again."
    since=$(date -u +%Y-%m-%dT%H:%M:%SZ)
    echo "==> Starting $WORKFLOW for $branch at ${head:0:7}"
    gh workflow run "$WORKFLOW" --ref "$branch"
    run=
    for _ in $(seq 30); do
        sleep 4
        run=$(gh run list --workflow "$WORKFLOW" --branch "$branch" --event workflow_dispatch --limit 10 \
            --json databaseId,headSha,createdAt \
            --jq "[.[] | select(.headSha == \"$head\" and .createdAt >= \"$since\")][0].databaseId // empty")
        [[ -n $run ]] && break
    done
    [[ -n $run ]] || fail "no $WORKFLOW run for ${head:0:7} showed up (was $branch pushed again meanwhile?)."
    echo "==> Waiting for run $run (approve the signing request in SignPath)"
    gh run watch "$run" --exit-status >/dev/null || fail "run $run failed: gh run view $run --log-failed"
    work=$(mktemp -d)
    trap 'rm -rf "$work"' EXIT
    gh run download "$run" --name windows-signed --dir "$work" ||
        fail "run $run has no signed zip; is SignPath set up (README, Build a release)?"
    [[ -f "$work/$(basename "$ZIP")" ]] || fail "run $run's zip isn't $(basename "$ZIP"): $(ls "$work")"
    scripts/check-windows-signature.sh "$work/$(basename "$ZIP")"
    mkdir -p dist
    mv "$work/$(basename "$ZIP")" "$ZIP"
    unzip -l "$ZIP"
    echo "Done: $ZIP ($(du -h "$ZIP" | cut -f1)), signed by SignPath Foundation. Publish it with scripts/release.sh."
    exit 0
fi

echo "Windows x86_64 build of $NAME $VERSION."
((DRY)) && { echo "Dry run: would write $ZIP."; exit 0; }

for prefix in llvm lld; do
    dir=$(brew --prefix "$prefix" 2>/dev/null) || { echo "Missing: brew install $prefix" >&2; exit 1; }
    PATH="$dir/bin:$PATH"
done
export PATH
cargo xwin --version >/dev/null 2>&1 || { echo "Missing: cargo install cargo-xwin --locked" >&2; exit 1; }
rustup target add "$TARGET" >/dev/null 2>&1

echo "==> Building $TARGET"
CARGO_PROFILE_RELEASE_STRIP=symbols cargo xwin build --release --locked -p diggr --target "$TARGET"

scripts/check-windows-exe.sh "$EXE"
scripts/pack-windows-zip.sh "$EXE" "$ZIP"
unzip -l "$ZIP"

cat <<EOF
Done: $ZIP ($(du -h "$ZIP" | cut -f1)).

Unsigned: Windows SmartScreen stops the first open of a downloaded copy (More info, then Run
anyway), and Windows 11 with Smart App Control on blocks it. For a release, build it signed with
scripts/make-windows.sh --signed; to publish this one, pass --windows-unsigned to scripts/release.sh.
EOF
