## MODIFIED Requirements

### Requirement: Minimal access
The extension SHALL ask for access only to discogs.com, the subdomains of bandcamp.com, juno.co.uk, beatport.com, traxsource.com and bleep.com, and 127.0.0.1, and for its own storage, context menu and toolbar button. It SHALL send the app only the page's address, the chosen mode, the crate and the switches; for a Bandcamp page, its title; for a Juno page, the records the page shows as `juno-intake` describes them; for a Beatport or Traxsource page, the tracks the page shows as `beatport-intake` and `traxsource-intake` describe them; and for a Bleep page, the records the page shows as `bleep-intake` describes them. It SHALL contact no other server, and SHALL make no request to juno.co.uk, beatport.com, traxsource.com, bleep.com or any other site. It SHALL read nothing from pages beyond their address and title, except on Juno, Beatport, Traxsource and Bleep pages, where it SHALL read only the fields their specs list from the page already loaded (on Beatport, from the page's own data, `__NEXT_DATA__`), and only when the user chooses an action. It SHALL contain no remote code.

#### Scenario: Permissions at install
- **WHEN** the extension is loaded in Chrome
- **THEN** the only site access it lists is discogs.com, *.bandcamp.com, *.juno.co.uk, *.beatport.com, *.traxsource.com, *.bleep.com and 127.0.0.1

#### Scenario: Nothing read until asked
- **WHEN** the user opens a Juno product page and doesn't open the button's menu
- **THEN** the extension has read nothing from the page beyond its address, and has sent nothing to the app

#### Scenario: No fetching on Juno
- **WHEN** the user sends a Juno label page showing 20 of its 200 records
- **THEN** the extension sends those 20 records and makes no network request other than the one to the app

## ADDED Requirements

### Requirement: Bleep page button
On release, label, artist, search and other list pages of bleep.com, the extension SHALL show its button next to the page's title (floating in a corner when it can't be found). Its menu SHALL offer the actions it offers on a Juno page, and SHALL say how many records the page shows. On a page listing records, each record SHALL also get a small ▶ after its title that plays that record alone in ‹App› (Play, as on a Discogs for-sale list). New crate… SHALL suggest "Artist - Title" on a release page and the page's heading on other pages, at most 40 characters. When the page can't be read, the menu SHALL say "Couldn't read this Bleep page", send nothing and offer no action. On other Bleep pages (basket, account, checkout), no button SHALL be shown, and no menu items SHALL be shown on links to them.

#### Scenario: A label page
- **WHEN** the user opens a Bleep label page showing 24 releases and chooses Play in ‹App›
- **THEN** the 24 records are sent, the first track sent plays, and the page shows "Sent to ‹App›: 24 records → ‹crate›" for 3 s

#### Scenario: One record from a list
- **WHEN** the user clicks the ▶ after "Deep Time" on a Bleep label page
- **THEN** only that record is sent with mode Play, and the page shows "Playing in ‹App›: Deep Time"

#### Scenario: Unreadable page
- **WHEN** Bleep changes its pages so no record can be read
- **THEN** the menu says "Couldn't read this Bleep page", and nothing is sent

#### Scenario: Basket
- **WHEN** the user opens their Bleep basket
- **THEN** no button is shown
