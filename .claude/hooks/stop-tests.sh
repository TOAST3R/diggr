#!/usr/bin/env bash
# Stop hook: if Rust code changed in the working tree, run `cargo test --workspace`
# before the session stops. A failure blocks the stop (exit 2) and hands the test
# output back to Claude. A tree that already passed is not tested again.
set -u

input=$(cat)
cd "${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel)}" || exit 0

paths=('*.rs' '*.toml' 'Cargo.lock' '*.ron')
changed=$( { git diff HEAD --name-only -- "${paths[@]}"; git ls-files --others --exclude-standard -- "${paths[@]}"; } | sort -u)
[ -z "$changed" ] && exit 0

# Fingerprint of the changed code, so an unchanged tree isn't retested.
fp=$( { git diff HEAD -- "${paths[@]}"; git ls-files --others --exclude-standard -z -- "${paths[@]}" | xargs -0 shasum 2>/dev/null; } | shasum | cut -d' ' -f1)
state="$(git rev-parse --git-dir)/claude-stop-tests"
[ "$(cat "$state.ok" 2>/dev/null)" = "$fp" ] && exit 0

# Claude already continued once after a failure on this exact tree: let it stop.
if echo "$input" | grep -q '"stop_hook_active": *true' && [ "$(cat "$state.fail" 2>/dev/null)" = "$fp" ]; then
  exit 0
fi

out=$(cargo test --workspace 2>&1)
if [ $? -eq 0 ]; then
  echo "$fp" > "$state.ok"
  exit 0
fi
echo "$fp" > "$state.fail"
{
  echo "cargo test --workspace failed after code changes. Fix the failures before stopping:"
  echo "$out" | grep -E '^(test .* FAILED|failures:|---- |error(\[|:)|thread .* panicked)' | head -60
  echo "..."
  echo "$out" | tail -40
} >&2
exit 2
