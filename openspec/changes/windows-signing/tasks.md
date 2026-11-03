## 1. SignPath Foundation (maintainer, outside the repository)

- [x] 1.1 Turn on MFA for GitHub (TOAST3R) and, once the account exists, for SignPath
- [x] 1.2 Publish the diggr.es code-signing page (diggr_es change `signpath-code-signing`) and give `https://diggr.es/code-signing/` as the download URL in the application
- [ ] 1.3 Send the application (OSI licence GPL-3.0-or-later, repository `TOAST3R/diggr`) and ask for the GitHub Actions integration in it; record the answer and dates in `findings.md` in this change
- [ ] 1.4 After approval: create the project, the artifact configuration (zip with `*/Diggr.exe` signed) and the release-signing policy with manual approval in SignPath; link the GitHub repository as a trusted build system
- [ ] 1.5 Add `SIGNPATH_API_TOKEN` as a repository secret and the organization ID, project slug and policy slug as repository variables

## 2. Shared Windows scripts

- [x] 2.1 Move the imports and subsystem checks from `make-windows.sh` into `scripts/check-windows-exe.sh <exe>` (`LLVM_OBJDUMP` still overrides the tool); `make-windows.sh` calls it and behaves as before
- [x] 2.2 Move the packing into `scripts/pack-windows-zip.sh <exe> <out.zip>` (folder name, `README.txt` and `LICENSE.txt` with CRLF, the 25 MB check); `make-windows.sh` calls it; confirm the Mac-built zip lists the same files as before
- [x] 2.3 Make both scripts run under Git Bash on Windows (no `stat -f`, no BSD-only flags) and check it with `shellcheck`

## 3. Workflow

- [x] 3.1 Write `.github/workflows/windows-signed.yml`: `workflow_dispatch` only, `windows-latest`, Rust 1.98 with the msvc target, `cargo build --release --locked -p diggr`, then the two shared scripts, then upload the unsigned zip as an artifact
- [x] 3.2 Add the SignPath step (SignPath's submit-signing-request action, waiting for completion), skipped with a log line when the variables are unset; then `Get-AuthenticodeSignature` on the extracted exe (Valid, signer SignPath Foundation) and upload the signed zip
- [ ] 3.3 Run it once before approval (signing skipped); compare the CI zip's file list and the exe's imports with the Mac zip's; record differences in `findings.md`

## 4. Mac side

- [x] 4.1 `make-windows.sh --signed`: check `HEAD` is pushed, `gh workflow run windows-signed.yml --ref <branch>`, find the run whose `headSha` is `HEAD`, `gh run watch --exit-status`, `gh run download` into a temp folder, move the zip into `dist/`; fail on a mismatch or failure with no zip left behind
- [x] 4.2 Signature check (shared by `make-windows.sh --signed` and `release.sh`): extract `Diggr.exe`, `osslsigncode verify`, signer contains `SignPath Foundation`; `brew install osslsigncode` named in the error when it's missing
- [x] 4.3 `release.sh`: `--windows-unsigned` option; without it, the signature check stops the release before tagging; the Windows release-notes text picks the signed or unsigned wording
- [x] 4.4 `release.sh --dry-run` with an unsigned zip stops at the signature check, and with `--windows-unsigned` passes

## 5. Docs

- [x] 5.1 `packaging/windows/README.txt`: signed by SignPath Foundation; Windows may still ask once (More info ▸ Run anyway); Windows 11 with Smart App Control supported
- [x] 5.2 `README.md`: build section (`--signed`, the workflow, secrets and variables, `--windows-unsigned`), the first-open steps, a link to `https://diggr.es/code-signing/`
- [x] 5.3 `CHANGELOG.md` `## Unreleased`: the Windows build is signed and opens with Smart App Control on (with the `changelog` skill)

## 6. First signed release

- [ ] 6.1 Bump the patch version, `make-windows.sh --signed`, approve the request, then `release.sh --unsigned --prerelease` (ask the maintainer before tagging)
- [ ] 6.2 On a Windows machine: download in a browser, extract, open; note SmartScreen's text and publisher, and Properties ▸ Digital Signatures; on Windows 11 with Smart App Control on if one is available. Record in `findings.md`
- [ ] 6.3 After confirmation, promote the release (ask first) and update diggr.es with the `release-version` skill's first-signed-release step
- [ ] 6.4 Confirm `findings.md` has no unrecorded failure, then archive (after `windows-port` is archived)
