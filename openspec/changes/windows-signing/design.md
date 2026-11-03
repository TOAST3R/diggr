## Context

- `windows-port` cross-builds `Diggr.exe` on the Mac with `cargo-xwin`, checks it (only DLLs that
  Windows provides, GUI subsystem), packs `Diggr-‹version›-x86_64-windows.zip` and publishes it
  unsigned through `scripts/release.sh`. Its Windows 10 test (section 7) is still open.
- Smart App Control on Windows 11 blocks unsigned exes outright when it is on. A signature from
  a trusted certificate lets the exe through. SmartScreen is separate: it warns about files with
  little download history, signed or not, until reputation builds.
- SignPath Foundation gives open-source projects a free certificate and signing service. Its
  terms: OSI licence and public repository, no proprietary components, only artifacts built from
  the project's own source, MFA for every team member on SignPath and on GitHub. The certificate
  names SignPath Foundation as the publisher. Signing requests come from CI, and SignPath checks
  that the artifact came from the project's workflow run (origin verification).
- The repository has no GitHub Actions workflows yet. Releases are made on the Mac.
- One maintainer. One Windows 10 laptop to test on, no Windows 11 machine for certain.

## Goals / Non-Goals

**Goals:**
- A Windows zip whose `Diggr.exe` is Authenticode-signed by the SignPath Foundation certificate,
  published through the existing release flow.
- The same checks on the CI-built exe as on the Mac-built one, from one script.
- A release can't publish an unsigned Windows zip by accident.
- The Mac cross-build keeps working, for local testing and as the fallback while SignPath
  approval is pending or if it is refused.

**Non-Goals:**
- Running the test suite in CI, or building macOS and Linux in CI.
- An installer, MSIX or the Microsoft Store (the other free route; revisit if SignPath refuses).
- Getting rid of the SmartScreen warning: that needs download reputation, which time gives.
- arm64 Windows.

## Decisions

### Build on a GitHub-hosted Windows runner with the MSVC toolchain
SignPath signs only what it can trace to the project's CI, so the signed exe can't be the one
built on the Mac. The workflow runs on `windows-latest` with Rust's own `x86_64-pc-windows-msvc`
toolchain and the runner's Visual Studio build tools, not `cargo-xwin`: it's the native path, and
`.cargo/config.toml`'s static C runtime applies to it unchanged. The toolchain version is pinned
to the version AGENTS.md names (1.98, as `dtolnay/rust-toolchain` input), and the build
uses `--locked`.

Alternative: run `cargo-xwin` on a Linux runner. Rejected: it signs the same way but adds the
Microsoft CRT download and a cross toolchain to CI for no gain, and Windows runners are free for
public repositories.

### Start the workflow by hand, for one commit
The workflow is `workflow_dispatch` only, with no tag trigger. `release.sh` creates the tag after
every file is in `dist/`, so a tag-triggered build would run too late. `make-windows.sh --signed`
checks that `HEAD` is pushed, starts the workflow for `HEAD`'s commit, finds the run for that
commit, watches it (`gh run watch`), downloads its signed artifact (`gh run download`) and places
it as `dist/Diggr-‹version›-x86_64-windows.zip`. The run's commit has to equal `HEAD`, or the
script fails: the zip must come from the commit about to be tagged.

Alternative: a tag-triggered workflow that uploads to the GitHub Release itself. Rejected: it
splits publishing between the Mac and CI, and the release would exist for minutes without its
Windows file.

### Submit the zip, sign the exe inside it
CI packs the zip exactly as `make-windows.sh` does (same folder name, `README.txt` and
`LICENSE.txt` with CRLF), uploads it as the unsigned artifact, and submits it with SignPath's
GitHub action (`signpath/github-action-submit-signing-request`). The SignPath artifact
configuration signs `*/Diggr.exe` inside the zip and returns the zip. One artifact in, one out,
and the packing stays in one place.

The packing commands move into `scripts/pack-windows-zip.sh`, run by both `make-windows.sh` and
the workflow (Git Bash is on the Windows runner), so the two zips can't drift apart.

### One check script for both builds
`scripts/check-windows-exe.sh <exe>` holds the imports and subsystem checks now inline in
`make-windows.sh`. It uses `llvm-objdump` (Homebrew's on the Mac, the runner's LLVM on Windows;
`LLVM_OBJDUMP` overrides it). With `--signed` it also checks the signature (below).

### Check the signature with `osslsigncode`
On the Mac, `osslsigncode verify` (Homebrew) reads Authenticode signatures. `release.sh`
extracts `Diggr.exe` from the zip into a temp folder and requires that it verifies and that the
signer's subject contains `SignPath Foundation`. In CI, the same check runs on the signed zip
before the artifact is uploaded, with `Get-AuthenticodeSignature` (status `Valid`), which needs
nothing installed.

### `release.sh`: signed Windows by default, `--windows-unsigned` to override
Today `--unsigned` means the disk image. Windows gets its own flag, so a Mac unsigned release
doesn't silently allow an unsigned Windows one: without `--windows-unsigned`, an unsigned or
wrongly signed zip stops the release before tagging. The release notes pick the Windows
first-open text to match.

### Secrets live in GitHub, the signing policy in SignPath
`SIGNPATH_API_TOKEN` is a repository secret; `SIGNPATH_ORGANIZATION_ID` and the project and
policy slugs are repository variables. The workflow reads them; nothing secret is in the
repository. The SignPath release-signing policy is set to need manual approval by the maintainer
for each request, which SignPath Foundation asks for and which makes each signature a decision.

## Risks / Trade-offs

- **SignPath refuses the project, or approval takes weeks** → the Mac unsigned path keeps
  working (`--windows-unsigned`); a refusal moves the Store (MSIX) route up, as a new change.
- **SmartScreen still warns** → expected until reputation builds; the docs say "may" and keep
  the Run anyway steps. Not treated as a failure.
- **Smart App Control still blocks the signed exe** → not expected (it trusts signed code), but
  it can't be confirmed without a Windows 11 with it on. The first signed release is a
  pre-release; if a tester reports a block, it is a finding and the Store route moves up.
- **The CI exe differs from the Mac one** (MSVC linker versus lld, another `rustc` build) → the
  same checks run on both, and the Windows test of section 7 is repeated on the CI-built exe.
- **SignPath's GitHub action needs access granted** (it has been a preview) → asked for in the
  application; the workflow can't be finished until it is.
- **Publisher shows "SignPath Foundation", not Diggr** → a condition of the free certificate;
  said in the README's code-signing policy.
- **Manual approval slows a release** → a few minutes per release, by design.
- **The workflow can be started by anyone with write access** → only the maintainer has it; the
  manual approval in SignPath stops a run nobody meant.

## Migration Plan

1. Apply to SignPath Foundation and wait (human step). Until then, releases stay unsigned with
   `--windows-unsigned`.
2. Land the scripts and the workflow. The workflow's signing step is skipped while the SignPath
   variables are unset, so the unsigned CI zip can be compared with the Mac one early.
3. Once approved and configured, make the next release with `--signed`, as a pre-release, and
   test it on the Windows 10 laptop (and a Windows 11 if available).
4. Promote it, and make signed the normal path in the README.

Rollback: release with the Mac build and `--windows-unsigned`.

## Open Questions

- Is the SignPath GitHub action generally available, or still preview-only?
- Exact SignPath artifact configuration for a PE file nested in a zip (their docs, at setup).
- Does SignPath Foundation want the code-signing policy in the README, or on a separate page?
- Is there a Windows 11 with Smart App Control on to test the first signed release on?
