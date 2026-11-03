## 1. Spike (on Windows, before any code)

- [x] 1.1 Create `findings.md` in this change with one line per spike check, plus the facts checked on 2026-11-03 (yt-dlp.exe unsigned; the Python binaries and deno.exe signed; zip app needs Python ≥ 3.11)
- [x] 1.2 On a Windows PC (done on Windows 11, Smart App Control off): the embeddable Python and the zip app; `--version`; a YouTube m4a without a runtime and with Deno, and a Bandcamp mp3
- [x] 1.3 On the same PC, time 5 starts and 5 previews with `python.exe yt-dlp` and with `yt-dlp.exe`; record the medians
- [ ] 1.4 (moved to the pre-release: no machine with it on) On a Windows 11 with Smart App Control on: Python, its `.pyd` files and `deno.exe` run, a preview plays
- [x] 1.5 Decide in `findings.md`: go on with the design

## 2. Running yt-dlp as a program plus arguments

- [x] 2.1 `preview/fetcher.rs`: `YtDlp { program, lead }`; `YtDlp::at` recognises the managed layout and splits it into Python, the zip app and the Deno arguments; every start (fetch, version, searches, reads) goes through one `command()` that puts `lead` first; tests with the layout made in a test folder, and that a user's `yt-dlp.exe` is never split

## 3. Windows managed install

- [ ] 3.1 `zip` crate (`dig`, deflate only) and one `unpack` helper that refuses entries outside the target folder; tests with zips made in the test, including a `../` entry
- [ ] 3.2 `tools.rs`: `MANAGED` includes Windows x86_64; the Windows asset is the zip app; pinned Python and Deno (version, address, SHA-256); `SIZE_MB` is the Windows total; `ToolSource::fetch` for fixed addresses, with errors that name the host
- [ ] 3.3 Install on Windows: Python and Deno each downloaded, checked against their pin, unpacked into `tools/.‹name›.part/` and swapped in with a version stamp; then the zip app as on other systems; progress over the whole; a failure at any step leaves the previous copy and no partial folder. Tests with `FakeSource` for match, mismatch, a missing file, a zip without its exe, and an older stamp
- [ ] 3.4 Update on Windows: the zip app daily as on other systems; Python or Deno replaced when the pinned version differs from the stamp; ignored tests that download the real Python and Deno and check their pins

## 4. The guided modal (Windows)

- [ ] 4.1 `DigSetup::guided_ytdlp` (true on Windows builds); without it, the question works exactly as before this change, and the existing tests stay unchanged
- [ ] 4.2 With it: the question's text (size, source, own files play without it, the Python note), Install / Not now, Escape; "Installing yt-dlp: N%"; "Previews are ready" then closed after 2 s; the failure with Try again
- [ ] 4.3 First launch: `ytdlp_setup_asked` in the dig settings; the UI asks the scheduler to look as soon as previews start, and opens the modal if nothing is found; either answer sets the flag. Headless test
- [ ] 4.4 Triggers: play or arm of a waiting entry that needs yt-dlp, sends from the extension, pastes that make a crate, a click on the "needs yt-dlp" icon; not samples, background prefetch or refreshes. Headless tests: Not now, then paste and play reopen it; the icon action reopens it; Install then plays the armed entry
- [ ] 4.5 No `brew` text on Windows or Linux: one hint per system, a test for each

## 5. Outdated detection (Windows)

- [ ] 5.1 Scheduler (`Config::outdated_check`, set with the guided setup): count consecutive `Failed` downloads on different clips, reset on success, ignore `Limited`, `Unreachable` and samples; at 3, force an update check of the app's own copy (at most once an hour) or report the user's version. Tests in `tests/previews.rs`
- [ ] 5.2 `tools::days_old` and the UI: for a user's yt-dlp older than 60 days, "may be out of date" with where to get a newer one, once a session. Tests

## 6. Docs

- [ ] 6.1 `packaging/windows/README.txt`: Diggr offers to install yt-dlp the first time previews are needed; a yt-dlp on the PATH is still used
- [ ] 6.2 `README.md`: the managed copy on Windows (Python, zip app, Deno, sizes, `tools/` layout), the guided modal, the outdated message, the release checklist line for the pins, the test count
- [ ] 6.3 `CHANGELOG.md` `## Unreleased` with the `changelog` skill

## 7. Check

- [ ] 7.1 The four checks from AGENTS.md and `cargo xwin check`
- [ ] 7.2 A test zip from `scripts/make-windows.sh`; on a Windows PC without yt-dlp: first launch → modal → Install → a Discogs record's preview plays, no console window, no "DASH m4a" playback problem; record in `findings.md`
