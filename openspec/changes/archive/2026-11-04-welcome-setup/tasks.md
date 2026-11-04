## 1. Steps and their states

- [x] 1.1 Dig settings: `welcome: { discogs, browser, at_launch }` (RON, `#[serde(default)]`: open, open, true); a broken file leaves every step open. Unit tests
- [x] 1.2 One function per step's state: Discogs Done while a token is set, Browser Done while the bridge has a paired browser (expose that from `dig::bridge`), Previews (Windows, guided) Done while yt-dlp is found; Declined from the settings. Unit tests

## 2. The setup panel

- [x] 2.1 A `welcome` module in `crates/ui/src/app/`: the panel (modal at launch, window on demand), each step with its text and buttons from the spec, Done/Declined shown with a way back; Escape closes
- [x] 2.2 Discogs step: Connect… opens Options ▸ Discogs… with the token field focused; "I don't have an account" declines and shows the link to create one; Later
- [x] 2.3 Browser step: Get the extension opens the store listing through the browser seam (a constant for the URL), then shows the pairing code from the bridge in the step; "I don't use Chrome" declines; Done as soon as a browser pairs
- [x] 2.4 Previews step (Windows, guided): reuses `ytdlp-onboarding`'s install and progress; the separate first-launch yt-dlp modal is no longer opened at launch
- [x] 2.5 At launch: opened on the second frame when a step is open and `at_launch` is on; "Don't show at startup". Headless tests: new user, everything set up, turned off, declined steps not listed; `--startup-time` still under 300 ms
- [x] 2.6 On demand: OPT ▸ Set up Diggr…, labelled "Set up Diggr… (‹n› of ‹m›)" while a step is open (the status line is a passing tooltip, so the count lives in the menu; decided 2026-11-04). Headless test

## 3. In the rest of the app

- [x] 3.1 Empty crates: the empty-crate hint (already in the skin's font, with store links) shows SET UP THE EXTENSION and CONNECT YOUR DISCOGS while their step is open, each opening the setup panel; without the panel it behaves as before. Headless test
- [x] 3.2 Locked features: add to collection, the Discogs cart and wantlist/collection sync open the panel at the Discogs step without a token, or say "needs a Discogs account" when it's declined; the Connect to Discogs dialog gives the one-line message when declined (the `discogs-write` delta). Headless tests
- [x] 3.3 Nudges: once a session, never for a declined step: a pasted Juno/Beatport/Traxsource/Bleep page gets Get the extension; the first anonymous send slowed by the rate limit says connecting doubles the speed. Headless tests

## 4. Docs and checks

- [x] 4.1 README: the Chrome Web Store link in place of Load unpacked (keep Load unpacked as the way to try an unreleased version), the setup panel and its steps, the test count
- [x] 4.2 CHANGELOG `## Unreleased` with the `changelog` skill
- [x] 4.3 diggr.es: already links the store listing on every page that mentions the extension (checked 2026-11-04; the site is out of this change's scope)
- [x] 4.4 The four checks from AGENTS.md and `cargo xwin check`
