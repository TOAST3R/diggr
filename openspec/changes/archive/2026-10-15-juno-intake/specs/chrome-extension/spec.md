## MODIFIED Requirements

### Requirement: Minimal access
The extension SHALL ask for access only to discogs.com, the subdomains of bandcamp.com and of juno.co.uk, and 127.0.0.1, and for its own storage, context menu and toolbar button. It SHALL send the app only the page's address, the chosen mode, the crate and the switches; for a Bandcamp page, its title; and for a Juno page, the records the page shows as `juno-intake` describes them. It SHALL contact no other server, and SHALL make no request to juno.co.uk or any other site. It SHALL read nothing from pages beyond their address and title, except on Juno pages, where it SHALL read only the records' fields, tracks, sample links and cover names from the page already loaded, and only when the user chooses an action. It SHALL contain no remote code.

#### Scenario: Permissions at install
- **WHEN** the extension is loaded in Chrome
- **THEN** the only site access it lists is discogs.com, *.bandcamp.com, *.juno.co.uk and 127.0.0.1

#### Scenario: Nothing read until asked
- **WHEN** the user opens a Juno product page and doesn't open the button's menu
- **THEN** the extension has read nothing from the page beyond its address, and has sent nothing to the app

#### Scenario: No fetching on Juno
- **WHEN** the user sends a Juno label page showing 20 of its 200 records
- **THEN** the extension sends those 20 records and makes no network request other than the one to the app

## ADDED Requirements

### Requirement: Juno page button
On product, label, artist, genre, chart, search and new release pages of www.juno.co.uk and its regional sites (`es.`, `de.`…, which share product ids), the extension SHALL show its button after the record's Play and Add to playlist buttons on a product page, else next to the page's title (floating in a corner when neither can be found). Its menu SHALL offer Play, Enqueue, Send to crate and skip passed, as on a Discogs release. New crate… SHALL suggest "Artist - Title" on a product page and the page's heading on other pages, at most 40 characters. The menu SHALL say how many records the page shows. When every record shows only one track, it SHALL add "1 track per record: set Show tracks to All for more". When the page can't be read, the menu SHALL say "Couldn't read this Juno page", send nothing and offer no action. On other Juno pages (cart, account, equipment), no button SHALL be shown. The extension SHALL show no menu items on links to Juno pages, since a link's page isn't loaded.

#### Scenario: Product page
- **WHEN** the user opens the page of product 1178909-01 and chooses Enqueue in ‹App›
- **THEN** the record with its two tracks is sent, and the page shows "Sent to ‹App›: 0 aka Mika Vainio - Rontgen → ‹crate›" for 3 s

#### Scenario: Listing with single samples
- **WHEN** the user opens a Juno label page showing 50 records, one sample each
- **THEN** the menu says "50 records" and "1 track per record: set Show tracks to All for more"

#### Scenario: Unreadable page
- **WHEN** Juno changes its markup so no record can be read
- **THEN** the menu says "Couldn't read this Juno page", and nothing is sent

#### Scenario: Equipment page
- **WHEN** the user opens a Juno DJ equipment page
- **THEN** no button is shown
