## Context

- The working name is spread across ~190 lines: four separate folder-name literals (`ui::settings`, `analysis::cache`, `visuals::library`, a temp fallback in `ui::app::digging`), seven env vars with the old prefix, the desktop package and binary, the app type, `dig::APP_NAME`, the User-Agent, the extension manifest, the window title (which reads the old product's name) and the eframe app id, plus comments and docs that describe the look by naming the old product.
- The app is installed on one Mac only. Its config folder holds crates, the Discogs token, the extension pairing (`dig/bridge.ron`) and sellers; its cache holds 606 MB of previews, scores and covers.
- The GitHub repo is already `TOAST3R/diggr` (renamed on 2026-10-08; GitHub redirects the old URL).
- `macos-release` (not started) plans the bundle id, URL scheme and DMG name "until the rebrand".

## Goals / Non-Goals

**Goals:**
- One name, Diggr, in everything the user sees, everything on disk and everything sent out.
- No occurrence of the old name in any tracked file, enforced by a test.
- The single existing install keeps all its data, through a one-time manual move.

**Non-Goals:**
- Migration code for old folders (it would have to name them).
- Rewriting git history.
- A new skin, icon or visual identity.
- Renaming the `apps/native` directory or other crate folders that don't carry the old name.

## Decisions

### 1. One folder name, in `platform`

`platform::APP_DIR` is `"Diggr"` on macOS and Windows and `"diggr"` elsewhere (XDG convention). Config, cache and visuals folders and the dig temp fallback all use it instead of their own string literal. `platform` stays wasm-portable: it's a `&str` constant behind `cfg!(target_os)`.

*Alternative:* `dirs`-style lowercase everywhere. Rejected: macOS apps show their names capitalised in Application Support.

### 2. Manual move, no migration code

The repo can't mention the old folder name, and there is one install, so the move is three commands, run once with the app closed (see Migration Plan). Without them the app starts empty but nothing is lost: the old folders stay where they were.

*Alternative:* move at launch if only the old folder exists. Rejected: it keeps the old name in the code forever for one machine.

### 3. Names per layer

| Layer | Name |
|---|---|
| Product, window title, extension, `APP_NAME`, docs | `Diggr` |
| Folders | `Diggr` (macOS, Windows), `diggr` (Linux) |
| Package and binary | `diggr` |
| Env vars | `DIGGR_CONFIG_DIR`, `DIGGR_CACHE_DIR`, `DIGGR_DISCOGS_TOKEN`, `DIGGR_FRAME_STATS`, `DIGGR_AUTO_FULLSCREEN`, `DIGGR_AUTO_QUIT_SECS`, `DIGGR_VISUAL_BENCH` |
| Rust type | `DiggrApp` |
| eframe app id | `diggr` |
| User-Agent | `Diggr/‹version› +https://github.com/TOAST3R/diggr` |
| Test temp root | `<temp>/diggr-tests` |
| Bundle id, URL scheme (`macos-release`) | `io.github.toast3r.diggr`, `diggr://` |

The skin's title bar draws text in its uppercase bitmap font, so it shows DIGGR wherever the name appears there.

### 4. Describing the look without naming a product

Comments, README, AGENTS.md, `openspec/config.yaml` and specs that said "<old name> 2.x-style" say "classic 2000s desktop player" (or "the classic skinned player") instead. Behaviour notes that compared to the old player ("as in <old name>: the playing track plays on") keep the behaviour and drop the comparison. The EQ's "classic band centres" and presets keep their values.

### 5. Archived changes are edited as text

Archived proposals, designs, specs and tasks are prose; nothing reads them at runtime. Each occurrence is replaced the same way as in living docs (binary, folders, env vars, descriptions). Their meaning doesn't change. Git history still has the originals.

### 6. A test keeps the repo clean

`apps/native/tests/no_old_name.rs` lists tracked files with `git ls-files` and fails if any text file contains the old name, case-insensitive. The name is assembled from two halves in the test, so the test itself doesn't contain it. When `git` isn't available (a source tarball), the test skips, like the GPU tests do without an adapter.

## Risks / Trade-offs

- **[Forgetting the manual move]** The app would open with no crates or token. → The data is still in the old folders; the README's "Where things are kept" names only the new ones, and the PR description lists the commands.
- **[Muscle memory: the old `cargo run -p …`]** fails after the change. → README and AGENTS.md give the new commands; cargo suggests nothing, so the PR says it plainly.
- **[Extension pairing]** The extension stores the app name it last saw. → It updates on the next hello; reloading the unpacked extension picks up the new manifest name. The pairing secret lives in `dig/bridge.ron`, which moves with the config folder.
- **[Local checkout path]** Renaming the local checkout orphans this project's Claude Code memory and forces a full rebuild (`target/` has absolute paths). → Rename the memory folder at the same time (Migration Plan step 4); the rebuild is one-off.
- **[Archived text drifts from what was merged then]** → Only names change; requirements, numbers and decisions stay.

## Migration Plan

After the change is merged, on the one machine, with Diggr closed:

1. Rename the old config folder in `~/Library/Application Support/` to `Diggr`.
2. Rename the old cache folder in `~/Library/Caches/` to `Diggr`.
3. In `chrome://extensions`, reload the unpacked extension.
4. From a terminal outside Claude Code, rename the checkout to `~/code-ai/diggr`, and this project's folder under `~/.claude-personal/projects/` to match its new path.

The exact commands are in the PR description; tracked files don't spell the old names.

Rollback: revert the PR and move the folders back.
