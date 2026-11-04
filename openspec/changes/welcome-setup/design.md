## Context

- The account: Options ▸ Discogs… takes a personal access token (`discogs-intake`, "Discogs
  account"). Without one, a want opens the Connect to Discogs dialog until "Don't show this
  again" (`discogs-write`, "Connect to Discogs"); add to collection opens it every time.
- The extension "Diggr Bridge" is on the Chrome Web Store
  (`https://chromewebstore.google.com/detail/diggr-bridge/cineoflmmlmgdkefaegeocjgelnijfkk`).
  It pairs with a 6-digit code from Options ▸ Browser… (`browser-bridge`, "Pairing"); the app
  keeps a hash per paired browser. Nothing in the app or the README links the store listing.
- Pasting a Juno, Beatport, Traxsource or Bleep address says "‹store› pages are sent from the
  browser extension" and nothing else.
- On Windows, `ytdlp-onboarding` added a first-launch yt-dlp question.
- The maintainer's choices (2026-11-04): the welcome opens at every launch while something is
  open; users without a Discogs account (or Chrome) can say so once and stop being asked;
  empty-crate cards and locked-feature hints too; OAuth is its own change (`discogs-oauth`).

## Goals / Non-Goals

**Goals:**
- A new user learns in the first minute what the account and the extension add, and can set up
  each in a click or two.
- Nobody is asked twice about something they said no to.
- The welcome never delays launch or interrupts playback.

**Non-Goals:**
- Signing in without a token (`discogs-oauth`).
- Browsers other than Chromium ones (the extension is Chrome-only).
- Telemetry on who set up what: nothing leaves the machine.

## Decisions

### 1. Steps with three states, kept in the dig settings
Each step is `Open`, `Done` or `Declined`. Done is read from the app's state, never stored:
Discogs is done while a token is set; Browser while at least one browser is paired; Previews
(Windows) while a yt-dlp is found. Declined is stored in `settings.ron` (`welcome: { discogs,
browser, at_launch }`), so it survives restarts and fails soft: a broken file means "open".
Declining is undone by the step's own button in the panel ("Connect after all").

### 2. One panel, three ways in
The same egui window serves the welcome at launch, OPT ▸ Set up Diggr…, and the status line's
"‹n› of ‹m› set up". At launch it shows only open steps; opened by hand, it shows every step
with its state, so a declined one can be reopened.

### 3. At launch: after the first frame, only while something is open
The modal opens on the second frame (launch stays under 300 ms), when at least one step is open
and `at_launch` is on. "Don't show at startup" turns `at_launch` off; the status line and OPT
still open the panel. On Windows the Previews step replaces `ytdlp-onboarding`'s first-launch
question, using the same install and progress; the user-action question stays as it is.

### 4. Get the extension
Opens the store listing in the default browser (the existing browser seam), then shows the
pairing code inside the step, so installing and pairing happen in one place; the step turns
Done when the extension pairs. "I don't use Chrome" declines it.

### 5. Empty crates and locked features
An empty crate draws up to three cards inside the playlist area (Paste a Discogs link; Connect
Discogs; Get the extension), each hidden once its step is done or declined. Menu items that
need the account (collection, cart, wantlist sync) keep their place and, without a token, open
the panel at the Discogs step instead of doing nothing; with Discogs declined they say "needs a
Discogs account" in their tooltip.

### 6. Nudges
At most one a session, each once, never for a declined step:
- a pasted store page only the extension can send: the existing line plus a Get the extension
  button in the main window;
- the first send slowed by the anonymous rate (25 a minute): "Connecting Discogs doubles the
  speed", with Connect…;
- the existing Connect to Discogs dialog after a want: unchanged, except that a declined Discogs
  step gives the one-line message instead.

## Risks / Trade-offs

- **A modal at every launch annoys** → only open steps, "Don't show at startup", and every
  step can be declined; after that the status line is the only reminder.
- **Paired but the extension later removed** → the step stays Done (the app can't tell); the
  panel's Browser step still offers pairing again.
- **The store URL changes** → one constant, and the README link.
- **More UI in the playlist area** → cards only in an empty crate, drawn with the skin's colours.

## Migration Plan

Existing users with a token and a paired browser see nothing new except OPT ▸ Set up Diggr….
Those with "Don't show this again" ticked in the Connect dialog keep it; it maps to nothing in
the welcome (the Discogs step is still open there unless they decline it).

## Open Questions

- Should "I don't have an account" link to Discogs' sign-up page? (Proposed: yes.)
- Edge and Brave can install Chrome Web Store extensions: say so in the Browser step?
