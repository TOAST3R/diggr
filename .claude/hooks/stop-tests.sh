#!/usr/bin/env bash
# Stop hook: if Rust code changed in the working tree, run the tests of the changed crates
# and of every crate that depends on them before the session stops. A failure blocks the
# stop (exit 2) and hands the test output back to Claude. A tree that already passed is
# not tested again.
#
# Plain `cargo test`, not nextest: nextest starts a process per test, and on machines
# with an endpoint-security agent that scans each exec it ran 1.6x slower here. Sessions
# sharing the checkout take turns: two suites at once starve each other's
# timing-sensitive tests.
set -u

input=$(cat)
cd "${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel)}" || exit 0

paths=('*.rs' '*.toml' 'Cargo.lock' '*.ron')
changed=$( { git diff HEAD --name-only -- "${paths[@]}"; git ls-files --others --exclude-standard -- "${paths[@]}"; } | sort -u)
[ -z "$changed" ] && exit 0

# Fingerprint of the changed code, so an unchanged tree isn't retested.
fp=$( { git diff HEAD -- "${paths[@]}"; git ls-files --others --exclude-standard -z -- "${paths[@]}" | xargs -0 shasum 2>/dev/null; } | shasum | cut -d' ' -f1)
state="$(git rev-parse --git-common-dir)/claude-stop-tests"
[ "$(cat "$state.ok" 2>/dev/null)" = "$fp" ] && exit 0

# Claude already continued once after a failure on this exact tree: let it stop.
if echo "$input" | grep -q '"stop_hook_active": *true' && [ "$(cat "$state.fail" 2>/dev/null)" = "$fp" ]; then
  exit 0
fi

# Changed files to packages. Anything outside one crate (root manifest, lockfile, a
# stray file) means the whole workspace.
crates=""
while IFS= read -r f; do
  case "$f" in
    crates/*/*) c=${f#crates/}; c=${c%%/*} ;;
    apps/native/*) c=diggr ;;
    assets/*) c=ui ;;
    *) crates=all; break ;;
  esac
  case " $crates " in *" $c "*) ;; *) crates="$crates $c" ;; esac
done <<< "$changed"

# One run at a time per checkout; a lock older than 20 minutes is from a killed run.
lock="$state.lock"
for _ in $(seq 1 600); do
  mkdir "$lock" 2>/dev/null && break
  [ -n "$(find "$lock" -maxdepth 0 -mmin +20 2>/dev/null)" ] && rmdir "$lock" 2>/dev/null
  sleep 1
done
trap 'rmdir "$lock" 2>/dev/null' EXIT

if [ "$crates" = all ]; then
  cmd=(cargo test --workspace --no-fail-fast)
else
  # Add the workspace crates that depend on a selected one until nothing new comes in.
  graph=$(cargo metadata --no-deps --format-version 1 --offline 2>/dev/null |
    jq -r '.packages[] | "\(.name) \([.dependencies[] | select(.path) | .name] | join(" "))"')
  grew=1
  while [ $grew = 1 ]; do
    grew=0
    while read -r pkg deps; do
      case " $crates " in *" $pkg "*) continue ;; esac
      for d in $deps; do
        case " $crates " in *" $d "*) crates="$crates $pkg"; grew=1; break ;; esac
      done
    done <<< "$graph"
  done
  cmd=(cargo test --no-fail-fast)
  for c in $crates; do cmd+=(-p "$c"); done
fi

out=$("${cmd[@]}" 2>&1)
if [ $? -eq 0 ]; then
  echo "$fp" > "$state.ok"
  exit 0
fi
echo "$fp" > "$state.fail"
{
  echo "\`${cmd[*]}\` failed after code changes. Fix the failures before stopping:"
  echo "$out" | grep -E '^test .* FAILED|^error(\[|:)|panicked at' | head -60
  echo "..."
  echo "$out" | tail -40
} >&2
exit 2
