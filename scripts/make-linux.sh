#!/usr/bin/env bash
# Builds dist/Diggr-<version>-x86_64-linux.tar.gz on this Mac, in Docker.
#
# The binary is built in an x86_64 Debian 11 container, so it runs on glibc 2.31 and later
# (Debian 11, Ubuntu 20.04, Fedora 32 and newer). Crates are fetched on the host and the build
# runs offline, so no HTTPS is needed from inside the container (TLS-intercepting proxies break
# it); apt uses Debian's plain-HTTP archive, where Debian 11's packages went at its end of life.
# Then the binary is checked: no glibc symbol newer than 2.31, and `diggr --help` must run in a
# bare Debian 11 with only ALSA installed.
#
# Usage: scripts/make-linux.sh [--dry-run]   (--dry-run only says what it would build)
#   LINUX_IMAGE overrides the build image (default rust:<host toolchain major.minor>-bullseye).
set -euo pipefail
cd "$(dirname "$0")/.."

NAME=Diggr
VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' apps/native/Cargo.toml | head -1)
TOOLCHAIN=$(rustc --version | sed -n 's/^rustc \([0-9]*\.[0-9]*\).*/\1/p')
IMAGE=${LINUX_IMAGE:-rust:$TOOLCHAIN-bullseye}
GLIBC_MAX=2.31
DIR=$NAME-$VERSION
TARBALL=dist/$DIR-x86_64-linux.tar.gz
CARGO_HOME=${CARGO_HOME:-$HOME/.cargo}
# deb.debian.org still lists Debian 11's security updates but no longer serves them.
APT_ARCHIVE='sed -i "s|deb.debian.org|archive.debian.org|" /etc/apt/sources.list && apt-get -o Acquire::Check-Valid-Until=false update -qq'

echo "Linux x86_64 build of $NAME $VERSION in $IMAGE."
[[ "${1:-}" == "--dry-run" ]] && { echo "Dry run: would write $TARBALL."; exit 0; }
docker info >/dev/null 2>&1 || { echo "Docker isn't running; start Docker Desktop." >&2; exit 1; }

echo "==> Fetching crates on the host"
cargo fetch --locked

echo "==> Building, checking and packing in $IMAGE"
mkdir -p dist
docker run --rm --platform linux/amd64 \
    -v "$PWD:/src" -w /src \
    -v "$CARGO_HOME/registry:/usr/local/cargo/registry" \
    -v "$CARGO_HOME/git:/usr/local/cargo/git" \
    -e CARGO_TARGET_DIR=target/linux-x86_64 -e CARGO_PROFILE_RELEASE_STRIP=symbols \
    -e DIR="$DIR" -e TARBALL="$TARBALL" -e GLIBC_MAX="$GLIBC_MAX" \
    "$IMAGE" bash -euo pipefail -c "$APT_ARCHIVE"'
        apt-get install -y -qq --no-install-recommends pkg-config libasound2-dev desktop-file-utils >/dev/null
        cargo build --release --locked --offline -p diggr
        bin=target/linux-x86_64/release/diggr

        newest=$(readelf -V "$bin" | grep -o "GLIBC_[0-9.]*" | sed "s/GLIBC_//" | sort -uV | tail -1)
        [ "$(printf "%s\n%s\n" "$newest" "$GLIBC_MAX" | sort -V | tail -1)" = "$GLIBC_MAX" ] ||
            { echo "The binary needs glibc $newest, newer than $GLIBC_MAX." >&2; exit 1; }
        echo "glibc check passed: needs $newest (at most $GLIBC_MAX)."
        desktop-file-validate packaging/linux/diggr.desktop

        stage=$(mktemp -d)
        mkdir "$stage/$DIR"
        install -m 755 "$bin" "$stage/$DIR/diggr"
        install -m 755 packaging/linux/install.sh "$stage/$DIR/install.sh"
        install -m 644 packaging/linux/diggr.desktop "$stage/$DIR/diggr.desktop"
        install -m 644 assets/icon/icon-256.png "$stage/$DIR/diggr.png"
        tar -C "$stage" --owner=0 --group=0 -czf "$TARBALL" "$DIR"
    '

echo "==> Running it on a bare Debian 11"
docker run --rm --platform linux/amd64 -v "$PWD/dist:/dist:ro" debian:bullseye-slim bash -euo pipefail -c "
    $APT_ARCHIVE && apt-get install -y -qq --no-install-recommends libasound2 >/dev/null
    tar -C /tmp -xzf /$TARBALL
    /tmp/$DIR/diggr --help
"

echo "Done: $TARBALL ($(du -h "$TARBALL" | cut -f1))."
