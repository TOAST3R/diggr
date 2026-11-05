# Tasks

## 1. Findings (before any code)

- [x] 1.1 Create `findings.md` in this change with the measurements of 2026-11-05 on this Mac: one-file `yt-dlp_macos --version` 57.1 s and 49.2 s on an idle Mac; folder build 40.4 s, then 1.0 s and 0.7 s; the hung probe's stack (`dyld` `mapSegments` → `fcntl`); 17 orphaned probes, 76 `_MEI*` folders of 71 MB, `XprotectService` at 97 % CPU; release 2026.10.26 lists `yt-dlp_macos.zip` in `SHA2-256SUMS`; the zip holds `yt-dlp_macos` and `_internal/`, 162 regular files, no symlinks. Verify: the file exists and every number has its source
- [x] 1.2 Check that a folder build started once inside `.yt-dlp_macos.part/` and then renamed to `yt-dlp_macos/` starts in ≤ 2 s after the rename (design D2), with the scratch script that times runs and kills the group. Record the result in `findings.md`; if it is slow, revise D2 (warm up after the rename) before group 3
- [x] 1.3 If a reboot is possible, time the folder build's first start after one (design risk 1) and record it; otherwise write "not measured" in `findings.md`

## 2. Process groups (macOS and Linux)

- [x] 2.1 `proc::command` puts each helper in its own process group on Unix; `proc::stop(&mut Child)` kills the group and waits (on Windows, `Child::kill` as now); `libc` becomes a direct Unix dependency of `dig`. Verify: a unit test in `proc.rs` starts `sh -c 'sleep 30 & sleep 30'`, stops it, and within 1 s no process of that group is left (`kill(-pgid, 0)` fails)
- [x] 2.2 `YtDlp::version`, `YtDlp::run` and `fetch`'s cancel use `proc::stop`. Verify: a test in `fetcher.rs` uses a fake yt-dlp script that starts a child and hangs on `--version`; after `version()` returns `None`, the child is gone too
- [x] 2.3 At most one look at a time. Verify: a test in `tests/previews.rs` with a finder that blocks for longer than `recheck` and counts the calls running at the same time never sees more than 1 over 10 rechecks

## 3. The folder install (macOS)

- [x] 3.1 `tools.rs`: pull the unpack, check, stamp and swap out of `install_pin` into one folder install used by the pins and by macOS, with a `prepare` step that runs on the `.part` folder before the swap. Verify: the existing pin tests pass unchanged (`cargo test -p dig tools`)
- [x] 3.2 On macOS `ASSET` is `yt-dlp_macos.zip` and `SIZE_MB` is 54; the folder is `tools/yt-dlp_macos/`, the program `tools/yt-dlp_macos/yt-dlp_macos`; the install reads the tag first, unpacks, and checks that the program is there. Verify: `the_sum_is_read_for_exactly_this_systems_asset` expects the zip's line on macOS; a `FakeSource` zip release installs into the folder with no `.part` or `.old` left; a zip without the program is refused and leaves the old copy
- [x] 3.3 First start (design D2): `--version` on the `.part` copy, 180 s limit, stopped with `proc::stop`; its output must equal the tag, which is then written as the stamp; the progress reports a Preparing stage after the download. Verify: tests with fake zip releases whose program prints the tag (installed, stamped), prints another version (refused, old copy kept), or hangs, with a short limit set by the test (refused, nothing left running, no `.part`)
- [x] 3.4 `tools::installed(dir)` gives the app's copy and its stamped version without starting it; `update` uses the stamp as the current version. Verify: unit tests for present, no stamp, and no program; `updates_are_checked_once_a_day_and_only_install_newer_releases` passes on the folder layout
- [x] 3.5 Update `crates/dig/Cargo.toml`'s `zip` comment, the module doc of `tools.rs`, and the ignored `real_ytdlp_release_installs_and_runs` test to run the folder build. Verify: `cargo test -p dig -- --ignored real_ytdlp` passes on this Mac, and its second `--version` takes ≤ 2 s

## 4. Finding and replacing (scheduler)

- [x] 4.1 `Scheduler::look`: a user's yt-dlp is found through the finder as now; the app's copy through `tools::installed`, never started. Verify: a test in `tests/previews.rs` whose finder fails if it is given a path inside `tools/`, and the app's copy is still announced with its stamped version
- [x] 4.2 Old one-file copy (design D5): on macOS, `tools/yt-dlp` without the folder build starts the install without a question (once per session), sends `PreviewEvent::Preparing`, never starts the old file, and deletes it after success; a failure falls back to `NeedsYtDlp`. Verify: tests in `tests/previews.rs` for success (old file gone, previews start, the finder was never given `tools/yt-dlp`) and for failure
- [x] 4.3 Update `tests/previews.rs`'s managed-copy tests (`tools::program` paths) to the per-system layout. Verify: `cargo test -p dig --test previews` passes

## 5. The UI

- [x] 5.1 The question, OPT ▸ Discogs… and the welcome's Previews step show "Preparing yt-dlp… (the first start takes up to a minute)" during the Preparing stage; entries waiting meanwhile say "Preparing yt-dlp…" (a new `WaitKind`, read and written by the playlist file like the others). Verify: headless tests in `dig_tests.rs` for the modal's text during Preparing, and for an entry's status on the `Preparing` event; the playlist round-trip test covers the new status
- [x] 5.2 `dig_tests.rs`'s check that the requested asset is `dig::tools::ASSET` still passes, and the sizes shown come from `SIZE_MB`. Verify: `cargo test -p ui`

## 6. Docs and checks

- [x] 6.1 README: the cache layout (`tools/yt-dlp_macos/`), the macOS size, the new test areas and the test count; CHANGELOG `## Unreleased`: previews on a Mac start in about a second instead of never, the update happens by itself, and leftover `_MEI…` folders in the temporary folder can be deleted (written with the `changelog` skill). Verify: the README test count matches `cargo test --workspace`'s total
- [x] 6.2 On this Mac, with the old one-file copy in `tools/`, run `cargo run --release -p diggr`, send a crate, and watch: "Preparing yt-dlp…", then previews download; afterwards `ps` shows no `yt-dlp` once downloads end and no new `_MEI*` folder in `$TMPDIR`. Record the timings in `findings.md`
- [x] 6.3 `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`, `cargo check -p audio -p platform --target wasm32-unknown-unknown` and `cargo xwin check --workspace --all-targets --target x86_64-pc-windows-msvc` all pass

## Workflow follow-up

- Sync the `preview-fetch` delta into `openspec/specs/` and archive the change after review.
- Consider the same folder build for Linux (`yt-dlp_linux.zip`) as a separate change.
