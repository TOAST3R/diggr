## MODIFIED Requirements

### Requirement: Minimal access
The extension SHALL ask for access only to discogs.com, the subdomains of bandcamp.com and 127.0.0.1, and for its own storage, context menu and toolbar button. It SHALL send the app only the page's address, the chosen mode, the crate and the switches, and, for a Bandcamp page, its title. It SHALL contact no other server, SHALL read nothing from pages beyond their address and title, and SHALL contain no remote code.

#### Scenario: Permissions at install
- **WHEN** the extension is loaded in Chrome
- **THEN** the only site access it lists is discogs.com, *.bandcamp.com and 127.0.0.1

## ADDED Requirements

### Requirement: Bandcamp page button
On a Bandcamp album or track page, the extension SHALL show its button next to the page's title (floating in a corner when the title can't be found). Its menu SHALL be the same as on a Discogs release: Play, Enqueue, Send to crate and skip passed. New crate… SHALL suggest "Artist - Title" from the page's title. On a Bandcamp label or artist page (`/` or `/music`), its menu SHALL offer only ‹App›: Send label. On other Bandcamp pages, no button SHALL be shown.

#### Scenario: Album page
- **WHEN** the user opens a Bandcamp album page while the app is running and paired
- **THEN** the button offers Play in ‹App›, Enqueue in ‹App› and Send to crate

#### Scenario: Label page
- **WHEN** the user opens `https://analogicalforce.bandcamp.com/music`
- **THEN** the button's menu offers only ‹App›: Send label, and choosing it follows the label in the app without changing its shown crate

#### Scenario: Merch page
- **WHEN** the user opens a Bandcamp merch page
- **THEN** no button is shown

### Requirement: Bandcamp links
Right-clicking a link to a Bandcamp album or track page, on any website, SHALL offer Play in ‹App› and Enqueue in ‹App›. A link to a Bandcamp label page (`/` or `/music`) SHALL offer only ‹App›: Send label.

#### Scenario: Album link in a forum
- **WHEN** the user right-clicks a Bandcamp album link and chooses Enqueue in ‹App›
- **THEN** its tracks are merged into the app's shown crate, and the toolbar button shows success for 3 s
