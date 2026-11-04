# welcome-setup Specification

## Purpose
Shows a new user what the Discogs account, the browser extension and (on Windows) previews add, and sets each up in a click or two: a panel at launch while a step is open, the same panel from OPT ▸ Set up Diggr…, and hints where a step would help. Only the user's "no" is kept, so nobody is asked again about what they don't have.
## Requirements
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
At every launch where at least one step is Open and "Don't show at startup" isn't ticked, the app SHALL open the setup panel as a modal after the first frame, listing only the Open steps: Previews first (Windows), then Discogs, then Browser. Launch SHALL still reach its first frame within 300 ms. The Discogs step SHALL say what the account adds (wantlist and collection sync, the Discogs cart, faster sends) and offer Connect…, "I don't have an account" (with a link to create one on discogs.com) and Later. The Browser step SHALL say what the extension adds (sending any Discogs, Bandcamp, Juno, Beatport, Traxsource or Bleep page with one click) and offer Get the extension, "I don't use Chrome" and Later. The modal SHALL offer "Don't show at startup" and Close (or Escape). No modal SHALL open at launch when every step is Done or Declined. When Connect… takes the user to Options ▸ Discogs…, the panel SHALL open again when that dialog closes, listing the steps still Open.

#### Scenario: A new user
- **WHEN** the app is opened for the first time, with no token and no paired browser
- **THEN** after the player appears, the welcome lists the Discogs and Browser steps (and, on Windows without yt-dlp, Previews first)

#### Scenario: Back after connecting
- **WHEN** the user clicks Connect… in the welcome, enters a token and closes Options ▸ Discogs…
- **THEN** the welcome opens again with the Browser step

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
Get the extension SHALL open the Chrome Web Store listing of Diggr Bridge (`https://chromewebstore.google.com/detail/diggr-bridge/cineoflmmlmgdkefaegeocjgelnijfkk`) in the default browser, and the Browser step SHALL then show the pairing code with how long it stays valid, and:
- **Open the extension's page**, which SHALL start the default browser on the extension's pairing page (`chrome-extension://cineoflmmlmgdkefaegeocjgelnijfkk/options.html`): on macOS the app that opens `https://` addresses, on Windows the program registered for them. When the default browser can't have the extension (Safari, Firefox and other non-Chromium browsers) or can't be found, the step SHALL say why;
- **Copy code**, which SHALL put the code on the clipboard;
- the page's address with **Copy**, for pasting by hand (another browser, a copy loaded unpacked).

The step SHALL turn Done as soon as a browser pairs, so the user installs and pairs without leaving the panel. The README SHALL link the same listing.

#### Scenario: Install and pair
- **WHEN** the user clicks Get the extension, installs it, clicks Open the extension's page, and pastes the copied code there
- **THEN** the step turns Done without restarting the app

#### Scenario: A browser that can't have it
- **WHEN** the default browser is Safari and the user clicks Open the extension's page
- **THEN** the step says Safari can't have the extension, and the address to paste stays shown

### Requirement: Empty crates show what to do
The hint an empty crate shows (how to paste a link, the stores, drop files, help) SHALL also link CONNECT YOUR DISCOGS while the Discogs step is Open and SET UP THE EXTENSION while the Browser step is Open, in the skin's font; each SHALL open the setup panel. Without the setup panel, SET UP THE EXTENSION SHALL open Options ▸ Browser… as before.

#### Scenario: Empty crate, nothing set up
- **WHEN** the user creates a new crate with no token and no paired browser
- **THEN** the crate's hint links CONNECT YOUR DISCOGS and SET UP THE EXTENSION

#### Scenario: Declined
- **WHEN** both steps are Declined
- **THEN** the hint shows neither link, and still shows the stores

### Requirement: Locked features say what unlocks them
Without a Discogs account, refreshing the collection or the wantlist SHALL open the setup panel instead of doing nothing; add to collection SHALL keep opening the Connect to Discogs dialog (see `discogs-write`). While the Discogs step is Declined, each of these SHALL instead say in one line that it needs a Discogs account.

#### Scenario: Refresh without an account
- **WHEN** the user without a token refreshes the collection crate
- **THEN** the setup panel opens

#### Scenario: Refresh with no account declared
- **WHEN** the same happens with the Discogs step Declined
- **THEN** the main window says it needs a Discogs account, and no panel opens

### Requirement: Nudges when they pay off
The app SHALL nudge at most once per session, each nudge at most once, never for a Declined step and never during a modal: pasting a page only the extension can send SHALL add to the existing message where to get the extension (OPT ▸ Set up Diggr…); the first send of more records than the anonymous rate allows in a minute SHALL say that connecting Discogs doubles the speed, and where.

#### Scenario: A Juno page pasted
- **WHEN** the user pastes a Juno address with the Browser step Open
- **THEN** the main window says Juno pages are sent from the browser extension, and to get it in OPT ▸ Set up Diggr…

#### Scenario: Declined
- **WHEN** the same happens with the Browser step Declined
- **THEN** only the existing message shows

