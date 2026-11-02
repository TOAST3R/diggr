## Context

Diggr builds and runs on macOS and x86_64 Linux. For Windows there is groundwork (`APP_DIR`
capitalised, `%APPDATA%` / `%LOCALAPPDATA%` in the README, `Cmd` mapped to `Ctrl`, permission
code behind `cfg(unix)`) but no Windows build has ever been made. Nothing in the repo is
`cfg(windows)`, there is no `windows_subsystem`, no icon resource, and the yt-dlp spawns would
each flash a console.

Constraints from the maintainer:

- Everything is built on one Mac. No Windows build machine, no CI.
- The only Windows machine is one Windows 10 laptop, without Rust installed, so `cargo test`
  cannot run there. The maintainer tests by downloading a GitHub Release in its browser and
  reporting back, which is also how a user would get it (the download carries the "from the
  internet" mark, so SmartScreen shows).
- Windows 11 should work, but is not tested. Windows 10 is the tested version.
- Unsigned for now. Signing is a later change.

What the dependency tree says about cross-building (read from `Cargo.lock` and `cargo tree`,
not yet built): for `x86_64-pc-windows-msvc`, `ring` (through ureq's rustls) is the only crate
that compiles C. wgpu is built with DX12, Vulkan and GL; cpal uses WASAPI; `rfd`, `arboard`
and `webbrowser` use the Win32 APIs.

## Goals / Non-Goals

**Goals:**
- A `diggr.exe` for Windows 10 x86_64, cross-built from the Mac, that starts, plays a local
  file and digs a preview, within the existing latency targets.
- A fresh Windows 10 needs nothing installed to run it.
- Findings from a real run, written down, before any fix beyond the known ones is designed.
- Nothing changes on macOS, Linux or the wasm check.

**Non-Goals:**
- Signing, an installer, and the README's Windows download link before the release is confirmed.
- A managed yt-dlp for Windows (decided after the spike; see Open Questions).
- A Windows test run in CI or on the laptop; native arm64; Windows 11 testing; an installer.
- Relaxing a latency target silently. A miss is a finding that needs a decision.

## Decisions

**1. Cross-build with `cargo-xwin` for the msvc target, not mingw.**
The msvc ABI is what Rust's Windows support and wgpu/cpal are tested against, and cargo-xwin
drives clang-cl and lld from Homebrew's `llvm` with Microsoft's CRT and SDK fetched once (the
maintainer accepts Microsoft's license on first use). `x86_64-pc-windows-gnu` with mingw would
avoid that download but is the less common configuration for these crates, and Homebrew's
mingw doesn't give the same system-library headers. If `ring` wants `nasm` for its assembly on
this target, it is one more `brew install`; the first build says.

**2. Static C runtime (`+crt-static`) in `.cargo/config.toml`, for the Windows target only.**
Without it, `diggr.exe` imports `VCRUNTIME140.dll`, and a Windows 10 without the Visual C++
Redistributable fails at launch with a dialog. It is the Windows counterpart of the Linux
build's glibc 2.31 ceiling. The check is `llvm-objdump -p diggr.exe`: every imported DLL must
be a system one (`KERNEL32`, `USER32`, `api-ms-win-*`, and so on); `VCRUNTIME*`, `MSVCP*` and
`ucrtbase` are failures. Done by hand in the spike, in a script in `windows-release`.

**3. Hide the console with a parent-console attach.**
Until the fix the exe is a console-subsystem binary, and the spike's first cross-build (copied
by hand or run from the Mac's own checks) keeps it so logs show. The released exe has no
console: `#![windows_subsystem = "windows"]` (release
builds only, so `cargo run` in a development build still has its console) plus, at the start
of `main`, `AttachConsole(ATTACH_PARENT_PROCESS)` so `diggr.exe --help` from a terminal still
prints. Alternative considered: two binaries (`diggr.exe` GUI, `diggr-cli.exe`); rejected, the
usage string and the release check both say `diggr`, and `app-identity` says one binary. Known
limit of the attach approach: from `cmd.exe` the prompt returns before the output ends, so the
output can interleave with the next prompt. Acceptable for the CLI modes, which are developer
tools. It also keeps a console-less release diagnosable: started from PowerShell, the exe
prints what it logs there, so the maintainer can run it that way on the laptop and report it.

**4. `CREATE_NO_WINDOW` on every yt-dlp spawn.**
`preview/fetcher.rs` has three `Command::new` calls and `tools.rs` two. A GUI-subsystem parent
spawning a console child makes Windows open a console window per spawn; previews spawn
constantly. One small helper in `dig` builds the `Command` (setting
`std::os::windows::process::CommandExt::creation_flags(0x0800_0000)` on Windows only) and the
five sites use it, so a sixth cannot forget. Not a new dependency: the flag is in std.

**5. The icon is a resource linked into the exe, generated from the mark.**
`icon-gen` already renders every PNG from `assets/icon/logo-mark.svg` and a test fails on
drift; it also writes `assets/icon/diggr.ico` (16, 32, 48, 256 px, PNG-compressed entries),
with the same test. `apps/native/build.rs` links it with the `embed-resource` crate, which can
use `llvm-rc` when cross-compiling (to be confirmed with Homebrew's llvm; if `llvm-rc` is
missing the fallback is a first Windows build with the generic icon, recorded as a known gap).
The build script is a no-op on every other target, so macOS and Linux builds do not change.
The window icon at runtime is already set from the 256 px PNG.

**6. Measure with the instruments that exist.**
`--startup-time` for launch, `--bench <file>` for start, seek and underruns, the same as the
latency spec. The laptop run records the numbers in the change (a findings file in the change
folder), so a later Windows 11 or second-machine run can be compared. A miss is written down
with the number; the decision (smaller buffer, exclusive mode, a documented Windows target) is
made then, not assumed.

**7. Compile-level coverage from the Mac instead of tests.**
`cargo xwin check --workspace --all-targets` and `cargo xwin clippy ... -D warnings` for the
Windows target catch type and `cfg` errors, including in test code. They cannot catch what
fails at runtime (a path separator, a rename onto an open file). Those surface on the laptop
or are accepted as a known gap; a Windows test run in CI is a separate decision, not made
here.

**8. A zip with `Diggr.exe` and `README.txt`, built by `scripts/make-windows.sh`.**
`Diggr-<version>-x86_64-windows.zip` holds one folder `Diggr-<version>/` (like the tarball)
with `Diggr.exe` (capitalised, so Task Manager shows the app's name) and a `README.txt` with
the first-open steps, the data folders and the yt-dlp note. The script follows
`make-linux.sh`'s layout: version from `apps/native/Cargo.toml`, `--dry-run`, a build, the
imports check (decision 2), the pack with the macOS-provided `zip`, a size check against 25 MB.
Alternatives considered: a bare `.exe` (the browser challenges a downloaded `.exe` more, and
there is no room for a note), and an NSIS installer (harder to debug from a Mac, more often
flagged by antivirus; revisit with signing).

**9. `release.sh` takes a third file and a `--prerelease` option.**
It looks for `dist/Diggr-<version>-x86_64-windows.zip`, stops before tagging when it is
missing and names `scripts/make-windows.sh`, puts all three files in one `SHA256SUMS`, and
appends a "Windows 10 and later (x86_64)" section to the notes next to macOS's and Linux's.
`--prerelease` passes `--prerelease` to `gh release create` (which also keeps it from becoming
`releases/latest`). Promoting after confirmation is `gh release edit v<version>
--prerelease=false --latest`, which the tasks do only when the maintainer says it works.

**10. v0.1.4 carries all three platforms.**
`release.sh` already refuses to tag unless every platform's file exists, and keeping that rule
is simpler than a Windows-only exception, so 0.1.4 is built for macOS (`make-dmg.sh`), Linux
(`make-linux.sh`, Docker) and Windows from the same commit. The version is bumped in
`apps/native/Cargo.toml` (and `Cargo.lock`), and `CHANGELOG.md`'s `## Unreleased` becomes
`## 0.1.4 — <date>`, with a Windows entry written as landing-page copy (the changelog skill).
The currently unreleased Spanish record stores entry ships in it.

**11. A pre-release until the laptop confirms.**
An untested Windows exe must not become `releases/latest`: the README's download links point
there, and macOS and Linux visitors would get an unconfirmed version. A pre-release is public
and downloadable without a GitHub login (a draft is not), does not become latest, and can be
deleted if the build is bad. This departs from the release spec's rule that an unsigned release
is not a pre-release; that rule exists so `releases/latest` leads to unsigned releases, and it
holds again once the release is promoted. The maintainer downloads the zip in the laptop's
browser, extracts it and runs it (so the real first-open path with SmartScreen is exercised),
then reports back; the report fills `findings.md`. A tag is never reused: a fix after a failed
test ships as the next patch version.

## Risks / Trade-offs

- **WASAPI shared mode may miss start < 30 ms or give underruns** → measure first (decision 6);
  if it misses, the options are a smaller buffer request, exclusive mode, or a stated Windows
  target; each is a spec decision, not a code tweak.
- **wgpu may not open a window on the laptop's GPU/driver** → wgpu's other backends are
  compiled in; the spike records which one was chosen and, if it fails, forces another with
  `WGPU_BACKEND` to separate a driver problem from a code problem.
- **Cross-building could fail on `ring` or the resource compiler** → known candidates (`nasm`,
  `llvm-rc`) are named; each failure becomes a finding with its fix in the build notes.
- **Only runtime Windows behaviour the spike exercises gets checked** → the spike's script
  covers startup, playback, a preview and a settings save; paths with spaces and non-ASCII
  characters in the user name are added by hand (a second folder on the laptop) because they
  are the usual Windows breakage; the rest is a known gap.
- **`yt-dlp.exe` may be slow to start per spawn** (I believe it is a one-file bundle that
  unpacks on every run; not measured) → the spike times a preview; the result decides whether
  a managed copy, or a recommendation to use a different yt-dlp build, is worth a later change.
- **Antivirus / SmartScreen on an unsigned exe** → on Windows 10 SmartScreen offers
  "Run anyway"; Smart App Control does not exist there. Not solved here; `windows-release`
  documents it.
- **No Windows 11 test** → stated in the spec as untested; the APIs used are the same.
- **A broken first release costs a version number** (tags aren't reused) → the Mac-side checks
  (cross-check, clippy, the imports check, the zip's size and contents) run before anything is
  published, the release is a pre-release, and the fix ships as 0.1.5.
- **Publishing is outward-facing and hard to undo** (a pushed tag, a public release) → the apply
  step asks the maintainer before pushing the tag, and says what it will publish and as what.
- **0.1.4 re-publishes macOS and Linux files for a Windows test** → they are real builds of the
  same commit, checked as for any release; as a pre-release they are not what "Download for
  Mac" gets until promotion.
- **SmartScreen on the downloaded zip** → expected and documented; Windows 10 has no Smart App
  Control, so "More info ▸ Run anyway" is available. If the laptop turns out to be blocked
  harder than that, it is a finding and signing moves up.

## Open Questions

- Which backend does wgpu choose on the laptop, and does it matter for the first frame (300 ms)?
- Does `--bench` meet 30 ms / 50 ms / 0 underruns on WASAPI? (Decides whether a latency
  change follows this one.)
- Does the yt-dlp lookup ("usual install folders") find `yt-dlp.exe` on the PATH and at a path
  set in OPT ▸ Discogs…? It must; the spike checks.
- Is a managed `yt-dlp.exe` worth a change of its own, given its start time and size?
- Does the browser bridge (127.0.0.1) raise a Windows Defender Firewall prompt? (The macOS
  release spec promises no permission prompts; the Windows release should keep that promise,
  or document it.)
