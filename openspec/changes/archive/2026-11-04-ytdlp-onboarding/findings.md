# Findings: ytdlp-onboarding

## Checked on the Mac, 2026-11-03 (yt-dlp 2026.10.26, Python 3.15.2, Deno 2.9.8)

| Check | Result |
|---|---|
| `yt-dlp.exe` signed? | **No** (`osslsigncode`: no signature). 22 MB, PyInstaller one-file. |
| `python-3.15.2-embed-amd64.zip` signed? | **Yes**: 34 binaries, 32 by the Python Software Foundation, 2 (C runtime) by Microsoft. 13.1 MB zip, 25 MB unpacked. `python315._pth` isolates it. |
| `yt-dlp` zip app | 2.9 MB, in `SHA2-256SUMS` (hash matched), needs Python ≥ 3.11, holds the `yt-dlp-ejs` scripts. |
| Zip app on a bare Python (`-I -S`, no site-packages), Bandcamp `mp3-128` | **Works**: `bc.mp3`, 6.4 MB. |
| Same, YouTube m4a, with Deno available | **Works**: format 140, solved YouTube's JS challenge with Deno. |
| Same, YouTube m4a, `--no-js-runtimes --no-cache-dir` (as a fresh Windows would be) | **Works today, deprecated**: "YouTube extraction without a JS runtime has been deprecated, and some formats may be missing"; it fell back to another player client. |
| Without ffmpeg | Works; yt-dlp warns "writing DASH m4a" (it can't fix up the container). Not checked yet whether symphonia plays every such file. |
| Deno's Windows build signed? | **Yes**, by Deno Land Inc. But 40 MB zip, 94 MB unpacked. |
| Homebrew Python's TLS behind Zscaler | Fails (`CERTIFICATE_VERIFY_FAILED`) until given the keychain roots (`SSL_CERT_FILE`). Expected not to happen on Windows, where Python reads the system store; to confirm in 1.2. |

## Consequence for the design

YouTube now needs a JavaScript runtime for yt-dlp to stay reliable. This is not specific to
Windows or to the zip app: the app's managed `yt-dlp_macos` and `yt-dlp_linux` don't include one
either, so every managed copy (and most user installs without Deno) is on the deprecated path
today. Decided 2026-11-03: not solved in this change; a follow-up
change adds a runtime for all three systems (design.md, Risks).

## On Windows 11 Enterprise 25H2 (build 26200), Smart App Control off, 2026-11-03

Run by a tester from the shared checklist (all commands as published there).

| Check | Result |
|---|---|
| Downloads and SHA-256 (Python zip, zip app, `yt-dlp.exe`, Deno zip) | all four OK |
| Signatures as Windows sees them | `python.exe`, `python315.dll`, `_ssl.pyd`: Valid, Python Software Foundation; `deno.exe`: Valid, Deno Land Inc.; `yt-dlp.exe`: NotSigned |
| 1.2 zip app on the embeddable Python (unpacked with Expand-Archive) | `--version` prints 2026.10.26 |
| 1.2 YouTube m4a, no runtime | works, with the "deprecated" warning (as on the Mac) |
| 1.2 YouTube m4a, with Deno | works: "Solving JS challenges using deno", no runtime warning |
| 1.2 Bandcamp mp3-128 | works, 6.4 MB |
| 1.2 TLS | no certificate errors on a corporate Windows (Python uses the Windows store) |
| 1.3 start (`--version`), median of 5 | Python 1.14 s, `yt-dlp.exe` 3.24 s |
| 1.3 one YouTube preview with Deno, cache on, median of 5 | Python 5.04 s, `yt-dlp.exe` 6.36 s |
| 1.4 Smart App Control on | **not tested**: no machine with it on (this one is off, and can't be turned back on). Windows trusts every signature the design relies on; confirmation moves to the pre-release (a tester with Smart App Control on) |
| 1.5 decision | **go**: the design holds; Python is faster to start than `yt-dlp.exe` and every program it starts is signed |

Still open from the Mac: whether the player plays every "DASH m4a" file yt-dlp writes without
ffmpeg (all three YouTube files here were format 140, the same as on the Mac). Checked in 7.2.

## JavaScript runtimes (2026-11-03; moved here from the dropped `ytdlp-js-runtime` change)

| Runtime | Version | Windows download | Windows signature |
|---|---|---|---|
| Deno | 2.9.8 | 40 MB zip, `deno.exe` 94 MB; SHA-256 `cba84b26e0df5ae7f74fc66b86a95b866872a86292ebd84fa1e959927edd93bf` | Deno Land Inc. (Valid on Windows 11) |
| Node | 26.13.0 LTS | 39 MB zip, `node.exe` 95 MB | OpenJS Foundation |
| QuickJS-ng | 0.18.0 | 1.8 MB | none |

Decided: Deno, on Windows only (2026-11-04); macOS and Linux stay as they are.
