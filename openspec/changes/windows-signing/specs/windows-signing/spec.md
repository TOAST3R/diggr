## ADDED Requirements

### Requirement: Windows exe built in CI
A GitHub Actions workflow, `.github/workflows/windows-signed.yml`, started by hand (`workflow_dispatch`) for one commit, SHALL build `diggr.exe` on a GitHub-hosted Windows runner for `x86_64-pc-windows-msvc`, with the Rust version AGENTS.md names, `--release --locked` and symbols stripped. It SHALL run `scripts/check-windows-exe.sh` on the exe (the same imports and GUI-subsystem checks the Mac build runs) and pack the zip with `scripts/pack-windows-zip.sh`, the same script the Mac build uses, so both zips have the same name, folder, files and line endings. A failed check SHALL fail the run before anything is submitted for signing.

#### Scenario: A clean run
- **WHEN** the workflow is started for a pushed commit at version 0.1.5
- **THEN** it produces `Diggr-0.1.5-x86_64-windows.zip` holding `Diggr-0.1.5/Diggr.exe`, `README.txt` and `LICENSE.txt`, and logs the imports and subsystem checks as passed

#### Scenario: A runtime DLL leaks in
- **WHEN** the CI-built exe imports `VCRUNTIME140.dll`
- **THEN** the run fails at the check and no signing request is sent

### Requirement: Signed by SignPath Foundation
The workflow SHALL submit the zip to SignPath with SignPath's GitHub action, using the repository secret `SIGNPATH_API_TOKEN` and the repository variables for the organization, project and signing policy. SignPath SHALL sign `Diggr.exe` inside the zip with the SignPath Foundation certificate and return the zip. The release-signing policy SHALL need the maintainer's manual approval for each request. The workflow SHALL check the returned exe with `Get-AuthenticodeSignature` (status `Valid`, signer containing `SignPath Foundation`) and upload the signed zip as the run's artifact. When the SignPath variables are not set, the workflow SHALL skip signing, upload the unsigned zip, and say so. No secret SHALL be kept in the repository.

#### Scenario: Approved request
- **WHEN** the maintainer approves the signing request in SignPath
- **THEN** the run's artifact is the zip whose `Diggr.exe` has a valid signature from SignPath Foundation, within 15 minutes of approval

#### Scenario: Not set up yet
- **WHEN** the workflow runs on a repository without the SignPath variables
- **THEN** it uploads the unsigned zip and its log says signing was skipped

#### Scenario: Request refused or timed out
- **WHEN** the signing request is denied or not approved in time
- **THEN** the run fails and uploads no zip

### Requirement: Signed zip fetched into dist
`scripts/make-windows.sh --signed` SHALL check that `HEAD` is pushed to `origin`, start the workflow for `HEAD`'s commit, wait for that run, download its artifact and place it as `dist/Diggr-‹version›-x86_64-windows.zip`, then verify it as `release.sh` does. It SHALL fail, writing no zip, when `HEAD` isn't pushed, when the run's commit isn't `HEAD`, when the run fails, or when the exe isn't signed by SignPath Foundation. Without `--signed`, the script SHALL cross-build the unsigned zip on the Mac as before.

#### Scenario: Release build
- **WHEN** `scripts/make-windows.sh --signed` runs on a pushed, clean `HEAD` and the request is approved
- **THEN** `dist/` has the signed zip for the current version, and the script prints the signer

#### Scenario: Unpushed commit
- **WHEN** `HEAD` is not on `origin`
- **THEN** the script stops before starting the workflow and says to push

### Requirement: Code-signing policy published
Before the first signed release, the project's download page SHALL say that Windows code signing is provided by SignPath Foundation, and a code-signing policy page SHALL give the attribution "Free code signing provided by SignPath.io, certificate by SignPath Foundation", the team roles and a privacy statement (kept on the diggr.es site, change `signpath-code-signing` there). The README SHALL link to that page from its build section.

#### Scenario: Application
- **WHEN** the SignPath Foundation application asks for the download URL
- **THEN** `https://diggr.es/code-signing/` is live with the download buttons and the attribution
