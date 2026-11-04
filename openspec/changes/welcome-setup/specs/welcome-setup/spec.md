## ADDED Requirements

### Requirement: Setup steps
The app SHALL track these setup steps, each Open, Done or Declined:
- **Discogs**: Done while a Discogs account is connected.
- **Browser**: Done while at least one browser is paired with the browser bridge.
- **Previews** (Windows x86_64 only): Done while a yt-dlp is found.

A step SHALL be Declined only when the user says so ("I don't have an account" for Discogs, "I don't use Chrome" for Browser); Declined SHALL be stored in the dig settings and survive restarts, and a settings file that can't be read SHALL leave every step Open. A Declined step SHALL never be asked about again (at launch, in nudges, in empty-crate cards) until the user reopens it from the setup panel.

#### Scenario: No Discogs account
- **WHEN** the user chooses "I don't have an account" and restarts the app twice
- **THEN** no launch, nudge or card asks to connect Discogs again, and the setup panel shows the Discogs step as declined with a way to connect after all

#### Scenario: Paired from the extension
- **WHEN** the Browser step is open and the extension pairs with the app
- **THEN** the step shows as Done without restarting the app

### Requirement: Welcome at launch
At every launch where at least one step is Open and "Don't show at startup" isn't ticked, the app SHALL open the setup panel as a modal after the first frame, listing only the Open steps: Previews first (Windows), then Discogs, then Browser. Launch SHALL still reach its first frame within 300 ms. The Discogs step SHALL say what the account adds (wantlist and collection sync, the Discogs cart, faster sends) and offer Connect…, "I don't have an account" (with a link to create one on discogs.com) and Later. The Browser step SHALL say what the extension adds (sending any Discogs, Bandcamp, Juno, Beatport, Traxsource or Bleep page with one click) and offer Get the extension, "I don't use Chrome" and Later. The modal SHALL offer "Don't show at startup" and Close (or Escape). No modal SHALL open at launch when every step is Done or Declined.

#### Scenario: A new user
- **WHEN** the app is opened for the first time, with no token and no paired browser
- **THEN** after the player appears, the welcome lists the Discogs and Browser steps (and, on Windows without yt-dlp, Previews first)

#### Scenario: Everything set up
- **WHEN** a token is set and a browser is paired (and, on Windows, yt-dlp is found)
- **THEN** no welcome opens at launch

#### Scenario: Turned off
- **WHEN** the user ticks "Don't show at startup" and restarts the app with steps still open
- **THEN** no welcome opens, and OPT ▸ Set up Diggr… still says how many steps are set up

### Requirement: Setup panel on demand
OPT ▸ Set up Diggr… SHALL open the same panel at any time, listing every step with its state; a Declined step SHALL offer to set it up after all. While a step is Open, the menu item SHALL read "Set up Diggr… (‹n› of ‹m›)", ‹n› the steps Done and ‹m› the steps this setup has.

#### Scenario: Changing one's mind
- **WHEN** the user declined Discogs and later opens OPT ▸ Set up Diggr…
- **THEN** the Discogs step shows as declined with Connect…, and connecting makes it Done

### Requirement: Getting the extension
Get the extension SHALL open the Chrome Web Store listing of Diggr Bridge (`https://chromewebstore.google.com/detail/diggr-bridge/cineoflmmlmgdkefaegeocjgelnijfkk`) in the default browser, and the Browser step SHALL then show the pairing code and how to enter it in the extension, so the user installs and pairs without leaving the panel. The README SHALL link the same listing.

#### Scenario: Install and pair
- **WHEN** the user clicks Get the extension, installs it, and enters the code shown in the step
- **THEN** the step turns Done and the panel says a browser was paired

### Requirement: Empty crates show what to do
An empty crate SHALL show, in place of its list, a card to paste a Discogs link, a card to connect Discogs and a card to get the extension; the Discogs and extension cards SHALL be hidden while their step is Done or Declined. Clicking a card SHALL open the setup panel at its step (or, for pasting, say how to paste).

#### Scenario: Empty crate, nothing set up
- **WHEN** the user creates a new crate with no token and no paired browser
- **THEN** the crate shows the three cards

#### Scenario: Everything set up
- **WHEN** a token is set and a browser is paired
- **THEN** an empty crate shows only the paste card

### Requirement: Locked features say what unlocks them
Actions that need a Discogs account (add to collection, the Discogs cart, syncing the wantlist and the collection) SHALL, without one, open the setup panel at the Discogs step instead of failing silently, unless the step is Declined, in which case their tooltip SHALL say "needs a Discogs account".

#### Scenario: The cart without an account
- **WHEN** the user without a token chooses an action that adds to the Discogs cart
- **THEN** the setup panel opens at the Discogs step

### Requirement: Nudges when they pay off
The app SHALL nudge at most once per session, each nudge at most once, never for a Declined step and never during a modal: pasting a page only the extension can send SHALL add Get the extension to the existing message; the first send slowed by the anonymous rate limit SHALL say that connecting Discogs doubles the speed, with Connect….

#### Scenario: A Juno page pasted
- **WHEN** the user pastes a Juno address with the Browser step Open
- **THEN** the main window says Juno pages are sent from the browser extension, with Get the extension

#### Scenario: Declined
- **WHEN** the same happens with the Browser step Declined
- **THEN** only the existing message shows
