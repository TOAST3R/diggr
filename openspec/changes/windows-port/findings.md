# Findings: windows-port

What was done and what happened, per check. A failure says whether it was fixed in this change
or left as a known gap, and why. Rows marked **open** need the Windows 10 laptop (tasks 7.x).

## Build tooling (Mac)

```sh
rustup target add x86_64-pc-windows-msvc
cargo install cargo-xwin --locked          # cargo-xwin 0.23.1
brew install llvm lld                      # clang-cl, llvm-objdump, llvm-rc / lld-link
export PATH="/opt/homebrew/opt/llvm/bin:/opt/homebrew/opt/lld/bin:$PATH"
cargo xwin build --release --locked -p diggr --target x86_64-pc-windows-msvc
```

- Homebrew's `llvm` does not include the linker: `lld` (for `lld-link`) is a separate formula.
- `cargo-xwin` downloaded Microsoft's CRT on first use (52 s) and asked nothing; the CRT and SDK
  stay under Microsoft's license terms.
- `ring` (the one crate that compiles C for this target) built **without `nasm`**.
- `llvm-rc` is present in Homebrew's `llvm` (the icon resource compiler).
- Release build: 2 min 55 s clean. `target/x86_64-pc-windows-msvc/release/diggr.exe`, 29.3 MB
  unstripped, PE32+ x86-64, console subsystem (before task 2.6).

## Checks

| Check | Result | Notes |
|---|---|---|
| Cross-build | pass | see above |
| Imports (`llvm-objdump -p`) | pass | system DLLs only, no `VCRUNTIME*`, `MSVCP*`, `ucrtbase`: advapi32, api-ms-win-core-synch-l1-2-0, api-ms-win-core-winrt-error-l1-1-0, bcrypt, bcryptprimitives, combase, dwmapi, dxgi, gdi32, imm32, kernel32, mmdevapi, ntdll, ole32, oleaut32, opengl32, setupapi, shell32, shlwapi, user32, uxtheme, ws2_32 |
| `cargo xwin check` / clippy (Windows target) | see below | |
| Launch (`--startup-time`) | **open** | |
| GUI (window, skin, icon) | **open** | |
| Local playback, `--bench` (start / seek / underruns) | **open** | |
| A preview with yt-dlp (no console flash, first-preview time) | **open** | |
| Settings save and reload, awkward paths | **open** | |
| Bridge and firewall prompt | **open** | |

## Fixed from the Mac

- The first cross-build printed hundreds of `LNK4099` lines (the CRT's `.pdb` files aren't
  shipped); silenced with `/ignore:4099` for the Windows target in `.cargo/config.toml`.
- Of the five `Command::new` sites I had counted for yt-dlp, three are production code (all in
  `preview/fetcher.rs`); the two in `tools.rs` are in tests. All production starts now go
  through `dig::proc::command`, which sets `CREATE_NO_WINDOW`, and a test fails if another
  appears.
- The Windows build found an `unused_mut` the Mac build can't see (the Unix-only yt-dlp
  folders); fixed.

## Known gaps

None yet.
