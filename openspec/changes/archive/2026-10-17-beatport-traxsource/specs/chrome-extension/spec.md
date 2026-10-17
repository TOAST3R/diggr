## MODIFIED Requirements

### Requirement: Minimal access
The extension SHALL ask for access only to discogs.com, the subdomains of bandcamp.com, juno.co.uk, beatport.com and traxsource.com, and 127.0.0.1, and for its own storage, context menu and toolbar button. It SHALL send the app only the page's address, the chosen mode, the crate and the switches; for a Bandcamp page, its title; for a Juno page, the records the page shows as `juno-intake` describes them; and for a Beatport or Traxsource page, the tracks the page shows as `beatport-intake` and `traxsource-intake` describe them. It SHALL contact no other server, and SHALL make no request to juno.co.uk, beatport.com, traxsource.com or any other site. It SHALL read nothing from pages beyond their address and title, except on Juno, Beatport and Traxsource pages, where it SHALL read only the fields their specs list from the page already loaded (on Beatport, from the page's own data, `__NEXT_DATA__`), and only when the user chooses an action. It SHALL contain no remote code.

#### Scenario: Permissions at install
- **WHEN** the extension is loaded in Chrome
- **THEN** the only site access it lists is discogs.com, *.bandcamp.com, *.juno.co.uk, *.beatport.com, *.traxsource.com and 127.0.0.1

#### Scenario: Nothing read until asked
- **WHEN** the user opens a Juno product page and doesn't open the button's menu
- **THEN** the extension has read nothing from the page beyond its address, and has sent nothing to the app

#### Scenario: No fetching on Juno
- **WHEN** the user sends a Juno label page showing 20 of its 200 records
- **THEN** the extension sends those 20 records and makes no network request other than the one to the app

## ADDED Requirements

### Requirement: Beatport and Traxsource page buttons
On track, release (Traxsource: title), chart, label, artist, genre and search pages of www.beatport.com and www.traxsource.com, the extension SHALL show its button next to the page's title (floating in a corner when it can't be found). Its menu SHALL offer Play, Enqueue, Send to crate and skip passed, as on a Discogs release, and SHALL say how many tracks the page shows. New crate… SHALL suggest "Artist - Title" on a track page and the page's heading on other pages, at most 40 characters. When the page can't be read, the menu SHALL say "Couldn't read this Beatport page" (or Traxsource), send nothing and offer no action. On other pages of either store (cart, account), no button SHALL be shown, and no menu items SHALL be shown on links to their pages.

#### Scenario: Beatport chart
- **WHEN** the user opens a Beatport chart showing 100 tracks and chooses Enqueue in ‹App›
- **THEN** the 100 tracks are sent, and the page shows "Sent to ‹App›: 100 tracks → ‹crate›" for 3 s

#### Scenario: Unreadable page
- **WHEN** Beatport changes its page data so no track can be read
- **THEN** the menu says "Couldn't read this Beatport page", and nothing is sent

#### Scenario: Cart
- **WHEN** the user opens the Traxsource cart
- **THEN** no button is shown

### Requirement: One Play action
Wherever the extension offered Play in ‹App› and Enqueue in ‹App› (the button's menu on Discogs, Bandcamp, Juno, Beatport and Traxsource pages, and the link menu), it SHALL offer only Play in ‹App›, which adds the page's tracks to the end of the crate on screen and plays the first track the send adds. Enqueue in ‹App› SHALL no longer be shown; Send to crate and the single-action menus (Send label, Add seller, Import collection & wishlist) are unchanged.

#### Scenario: Play from a release page
- **WHEN** the user chooses Play in ‹App› on a Discogs release page while the crate "Friday" is on screen
- **THEN** the release's tracks are added to the end of "Friday", no crate is created, and the first of them plays as soon as its preview is ready

#### Scenario: No Enqueue item
- **WHEN** the user opens the button's menu on any supported page, or right-clicks a supported link
- **THEN** no Enqueue in ‹App› item is shown

### Requirement: Play each record of a for-sale list
On a Discogs page listing marketplace items (a seller's shop, the marketplace), the extension SHALL show a small ▶ after each item's title link, one per item and none on its picture, which sends that item's address (`/sell/item/‹id›`) with mode Play. The page's own button is unchanged.

#### Scenario: A seller's shop
- **WHEN** the user clicks ▶ after a record's title on `https://www.discogs.com/seller/ADEPTA_STORE/profile`
- **THEN** that record is added to the crate on screen and its first track plays, and the page doesn't navigate

### Requirement: Send to crate lists crates that take sends
Send to crate SHALL list only the crates a send can fill: no Top Sellers crate, label crate (LABELS), Discogs collection or wantlist crate, or friend's crate.

#### Scenario: Special crates left out
- **WHEN** the user opens Send to crate while Diggr has a seller's crate and a followed label
- **THEN** neither is listed, and the Playlist and the user's own crates are
