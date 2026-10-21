---
name: changelog
description: Add user-facing entries to CHANGELOG.md after any change people using Diggr would notice (a feature, a key, a behaviour, a fix), and cut a release section when releasing. Use when finishing or applying a change (including /opsx:apply), before committing it, and when preparing release notes or landing-page copy.
---

# Changelog

`CHANGELOG.md` at the repo root is the one list of what changed for users. Its entries go onto
the landing page (`LANDING.md`) and into the GitHub release notes as they are, so write them
as landing copy, not as commit messages.

## When

- After every change a user would notice: add its entries to `## Unreleased` in the same
  commit as the change. Skip internal-only work (refactors, tests, CI, specs, tooling).
- When applying an OpenSpec change, add the entries once its tasks are done.
- When releasing, cut the section (below).

## How to write an entry

```markdown
- **Headline in a few words.** One or two plain sentences: what the user can do now, and the
  key or place to find it.
```

- Put it under `### New` (something new), `### Improved` (better at something it did),
  `### Changed` (works differently: say what replaced the old way), or `### Fixed`. Create a
  heading only when it has entries, in that order.
- Speak to the user ("you", or the action itself), present tense, no code names, file paths,
  crate names, spec names, PR numbers or tool identifiers.
- Name keys as the help panel does (Space, Shift+Space, ⌘V written as Cmd+V).
- One entry per thing a user would tell a friend about; merge small related bits.
- If a later change in the same Unreleased section alters an entry, edit that entry instead of
  adding a contradicting one.
- Match the voice of `LANDING.md`: short, concrete, a little warm, never hype.

## Cutting a release

1. Read the version from `apps/native/Cargo.toml`.
2. Rename `## Unreleased` to `## <version> — <YYYY-MM-DD>` and put a fresh empty
   `## Unreleased` above it.
3. Write that section's body to a notes file for `scripts/release.sh --notes-file`:
   ```sh
   awk '/^## [0-9]/{n++} n==1 && !/^## /' CHANGELOG.md > dist/notes.md
   ```
4. Offer to work the New and Improved entries into `LANDING.md` where they fit; the user
   decides what goes on the page.
