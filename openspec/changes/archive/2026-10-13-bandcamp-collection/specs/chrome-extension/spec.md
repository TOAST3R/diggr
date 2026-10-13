## MODIFIED Requirements

### Requirement: Bandcamp page button
On a Bandcamp album or track page, the extension SHALL show its button next to the page's title (floating in a corner when the title can't be found). Its menu SHALL be the same as on a Discogs release: Play, Enqueue, Send to crate and skip passed. New crate… SHALL suggest "Artist - Title" from the page's title. On a Bandcamp label or artist page (`/` or `/music`), its menu SHALL offer only ‹App›: Send label. On a Bandcamp fan's collection or wishlist page (`bandcamp.com/‹user›` or `bandcamp.com/‹user›/wishlist`), its menu SHALL offer only ‹App›: Import collection & wishlist, which sends only the page's address. On other Bandcamp pages, no button SHALL be shown.

#### Scenario: Album page
- **WHEN** the user opens a Bandcamp album page while the app is running and paired
- **THEN** the button offers Play in ‹App›, Enqueue in ‹App› and Send to crate

#### Scenario: Label page
- **WHEN** the user opens `https://analogicalforce.bandcamp.com/music`
- **THEN** the button's menu offers only ‹App›: Send label, and choosing it follows the label in the app without changing its shown crate

#### Scenario: Fan page
- **WHEN** the user opens `https://bandcamp.com/toast3r` and chooses ‹App›: Import collection & wishlist
- **THEN** the app starts importing fan "toast3r", and the extension sent nothing but the address

#### Scenario: Merch page
- **WHEN** the user opens a Bandcamp merch page
- **THEN** no button is shown
