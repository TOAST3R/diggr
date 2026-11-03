## Why

The Windows zip is unsigned. On Windows 10, and on Windows 11 with Smart App Control off, that
costs one "More info ▸ Run anyway". On a Windows 11 with Smart App Control on (new PCs, clean
installs, resets), an unsigned exe is blocked with no way past it, and that group grows as
Windows 10 machines are replaced. SignPath Foundation signs open-source projects for free, and
Diggr is now GPL-3.0-or-later in a public repository, so it can apply.

## What Changes

- A GitHub Actions workflow builds `Diggr.exe` on a GitHub-hosted Windows runner from a pushed
  commit, runs the same exe checks as the Mac build (Windows-provided DLLs only, GUI subsystem),
  packs the zip, and submits it to SignPath, which signs the exe inside it with the SignPath
  Foundation certificate. SignPath only signs builds it can trace to the project's own CI, so
  the signed build cannot come from the Mac.
- `scripts/make-windows.sh --signed` starts that workflow for the current pushed commit, waits for
  it, and downloads the signed zip into `dist/` under the usual name. Without `--signed` it
  cross-builds unsigned on the Mac as today (local testing, and a fallback).
- The exe checks move out of `make-windows.sh` into `scripts/check-windows-exe.sh`, so the Mac
  build and the CI build run the same checks.
- `scripts/release.sh` checks the Windows zip's signature and refuses an unsigned zip unless it
  is told `--windows-unsigned`, the way it already refuses an unsigned disk image without
  `--unsigned`.
- The first-open steps change: Smart App Control lets the signed exe through; SmartScreen may
  still say "Windows protected your PC" (publisher SignPath Foundation) until the certificate has
  a download history, so More info ▸ Run anyway stays in `README.txt`, the release notes and the
  README, worded as "may".
- The README gets the code-signing policy section SignPath Foundation asks signed projects to
  publish.
- Human steps, outside the repository: apply to SignPath Foundation, set up the project and
  signing policy there, and add its API token and organization ID as repository secrets.

## Capabilities

### New Capabilities
- `windows-signing`: the signed Windows build: where it is built, how it is signed, how the
  signature is checked, and what the maintainer has to set up once.

### Modified Capabilities
- `release`: the Windows first-open requirement now describes a signed exe (Smart App Control
  passes, SmartScreen may still show once); the one-command-build requirement gains the
  `--signed` path; publishing refuses an unsigned Windows zip without an explicit option. These
  build on the Windows requirements the `windows-port` change adds, so `windows-port` is archived
  first.

## Impact

- New: `.github/workflows/windows-signed.yml` (the repository's first workflow),
  `scripts/check-windows-exe.sh`.
- Changed: `scripts/make-windows.sh`, `scripts/release.sh`, `packaging/windows/README.txt`,
  `README.md` (build section, first-open steps, code-signing policy), `CHANGELOG.md`.
- New outside tools: `gh` (already used by `release.sh`) for starting and downloading the
  workflow run; `osslsigncode` (Homebrew) on the Mac to check the signature.
- Accounts and secrets: a SignPath Foundation project; `SIGNPATH_API_TOKEN` (secret) and
  `SIGNPATH_ORGANIZATION_ID` (variable) on `TOAST3R/diggr`. No secret is kept in the repository.
- No app code changes. The macOS and Linux builds are unchanged.
- Depends on `windows-port`: its Windows test (section 7) should pass unsigned first, so a
  failure afterwards points at signing.
