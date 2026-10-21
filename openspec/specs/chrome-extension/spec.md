# chrome-extension Specification

## Purpose
A thin Chrome extension that sends the Discogs, Bandcamp, Juno, Beatport or Traxsource page you're on, or a Discogs or Bandcamp link, to the player in one click, and shows what the player is doing.
## Requirements
### Requirement: Page button
On Discogs release, master release, artist, label, wantlist, collection, list and marketplace listing pages, the extension SHALL show a button next to the page's title. When the page's title can't be found, the button SHALL float in a corner of the page. The button's menu SHALL offer:
- Play in ‹App›, which adds the page's tracks to the end of the crate on screen and plays the first track it adds;
- Send to crate, listing the app's crates and New crate…;
- the "skip passed" switch, which SHALL be remembered. There SHALL be no "vinyl only" switch: every send keeps records in every format.

On a seller's page (`/seller/‹name›/profile`, `/seller/‹name›`, or `/user/‹name›` with no further path), the button SHALL be shown the same way, and its menu SHALL offer only Add seller to ‹App›. Choosing it SHALL send the page to the app, which adds or refreshes that seller (see `seller-crates`).

On a label's page, the button's menu SHALL offer only ‹App›: Send label. Choosing it SHALL send the page to the app, which follows that label, or refreshes it when it is already followed (see `label-crates`).

New crate… SHALL open a field in the menu holding a suggested name, taken from the page's title, as editable text; Enter or Create SHALL send:
- for a release, master release or listing, "Artist - Title";
- for an artist, label or list, its name;
- for a wantlist, "Wantlist: ‹user›";
- for a collection, "Collection: ‹user›".

The suggestion SHALL leave out Discogs' name-variant asterisks and disambiguation numbers such as "(2)", SHALL use "-" between artist and title, and SHALL be at most 40 characters.

On other Discogs pages, no button SHALL be shown. The button SHALL follow Discogs' in-page navigation, appearing and disappearing as the address changes.

#### Scenario: Label page
- **WHEN** the user opens a Discogs label page while the app is running and paired
- **THEN** the button's menu offers only ‹App›: Send label, and choosing it follows the label in the app without changing its shown crate

#### Scenario: Followed label
- **WHEN** the user opens the page of a label the app already follows and chooses ‹App›: Send label
- **THEN** the label's crate is refreshed, and no second crate is made

#### Scenario: Send to a new crate
- **WHEN** the user chooses Send to crate, then New crate…, and enters "Friday"
- **THEN** a crate "Friday" is created in the app with the page's tracks

#### Scenario: Suggested name
- **WHEN** the user chooses New crate… on the release page titled "D'Arcangelo* – TimeLss | Releases | Discogs"
- **THEN** the field already holds "D'Arcangelo - TimeLss", and Enter sends it

#### Scenario: Listing page
- **WHEN** the user opens `https://www.discogs.com/shop/item/3923678974` and chooses Play in ‹App›
- **THEN** the button is there, the tracks of the release on sale are added to the shown crate, and the first of them plays

#### Scenario: Collection page
- **WHEN** the user opens `https://www.discogs.com/user/digger/collection` and chooses New crate…
- **THEN** the field holds "Collection: digger", and Enter creates that crate in the app with the collection's tracks, on screen

#### Scenario: Seller page
- **WHEN** the user opens `https://www.discogs.com/seller/decks.de/profile` while the app is running and paired
- **THEN** the button's menu offers only Add seller to ‹App›, and choosing it adds decks.de to the app's TOP SELLERS and shows its crate

#### Scenario: Not a supported page
- **WHEN** the user opens a Discogs forum thread
- **THEN** no button is shown

#### Scenario: No vinyl only switch
- **WHEN** the user opens the button's menu on an artist page
- **THEN** it offers the skip passed switch and no vinyl only switch

### Requirement: Links anywhere
Right-clicking a link to a supported Discogs page, on any website, SHALL offer Play in ‹App›, except a link to a label's page, which SHALL offer only ‹App›: Send label.

#### Scenario: Forum link
- **WHEN** the user right-clicks a Discogs release link in a forum post and chooses Play in ‹App›
- **THEN** the release's tracks are added to the app's shown crate, and the first of them plays

#### Scenario: Label link
- **WHEN** the user right-clicks a Discogs label link in a forum post
- **THEN** the menu offers only ‹App›: Send label, and choosing it follows the label in the app

### Requirement: Feedback
After a send from a Discogs page, the page SHALL show, for 3 s, a confirmation naming the page and the crate. After a send from a link on another site, the extension's toolbar button SHALL show a mark for 3 s: success or failure. Clicking the toolbar button SHALL show whether the app is running and paired, what is playing, and sends in progress. When the app isn't running, the extension isn't paired, or the page isn't supported, the extension SHALL say so and how to fix it.

#### Scenario: Confirmation
- **WHEN** the user sends a label page (‹App›: Send label)
- **THEN** the page shows "Sent to ‹App›: Label: Lowtide Tapes → ‹crate›" for 3 s

#### Scenario: App not running
- **WHEN** the user chooses Play in ‹App› while the app is closed
- **THEN** a message says that the app isn't running, and nothing else happens

#### Scenario: Not paired
- **WHEN** the user chooses an action before the extension is paired
- **THEN** the pairing screen opens and asks for the code shown in Options ▸ Browser…

### Requirement: Minimal access
The extension SHALL ask for access only to discogs.com, the subdomains of bandcamp.com, juno.co.uk, beatport.com, traxsource.com and bleep.com, www.youtube.com, soundcloud.com, and 127.0.0.1, and for its own storage, context menu and toolbar button. It SHALL send the app only the page's address, the chosen mode, the crate and the switches; for a Bandcamp page, its title; for a Juno page, the records the page shows as `juno-intake` describes them; for a Beatport or Traxsource page, the tracks the page shows as `beatport-intake` and `traxsource-intake` describe them; for a Bleep page, the records the page shows as `bleep-intake` describes them; and the results of store cart actions with the baskets read (see `browser-bridge`, Cart requests). It SHALL contact no other server. To juno.co.uk and bleep.com it SHALL make only the basket requests listed in `store-carts`' design (D1 results), and to beatport.com and traxsource.com none, and only to perform a cart action the app gave it or to read that store's basket; it SHALL make no request to any other site. On YouTube and SoundCloud it SHALL read and send only the page's address (the app reads the mix itself). It SHALL read nothing from pages beyond their address and title, except on Juno, Beatport, Traxsource and Bleep pages, where it SHALL read only the fields their specs list from the page already loaded (on Beatport, from the page's own data, `__NEXT_DATA__`), and only when the user chooses an action. It SHALL contain no remote code.

#### Scenario: Permissions at install
- **WHEN** the extension is loaded in Chrome
- **THEN** the only site access it lists is discogs.com, *.bandcamp.com, *.juno.co.uk, *.beatport.com, *.traxsource.com, *.bleep.com, www.youtube.com, soundcloud.com and 127.0.0.1

#### Scenario: Nothing read until asked
- **WHEN** the user opens a Juno product page and doesn't open the button's menu
- **THEN** the extension has read nothing from the page beyond its address, and has sent nothing to the app

#### Scenario: No fetching on Juno
- **WHEN** the user sends a Juno label page showing 20 of its 200 records
- **THEN** the extension sends those 20 records and makes no network request other than the one to the app

#### Scenario: Only basket calls
- **WHEN** the app gives the extension an add for a Bleep format
- **THEN** the extension makes only Bleep's basket requests for it, and reports the result to the app

### Requirement: The app's name
Every label that names the player (for example "Play in ‹App›") SHALL use the name the app reports, so renaming the app renames them.

#### Scenario: Renamed app
- **WHEN** the app reports its name as "My Player"
- **THEN** the menus read "Play in My Player"

### Requirement: Owned note
On release, master release and marketplace item pages, when the app says the page's record is owned, the extension SHALL show under its button "✓ In your collection", or "✓ Another pressing in your collection (‹catalog number›, ‹year›)". When the answer is "checking", it SHALL ask once more after 3 s. When the app has no Discogs token, it SHALL show, dimmed, that a token in the app (Options ▸ Discogs…) is needed to see owned records. When the record isn't owned, or the answer is unknown, it SHALL show nothing. It SHALL send the app only the page's address for this.

#### Scenario: Owned on a shop page
- **WHEN** the user opens a marketplace item of a record they own
- **THEN** within a few seconds the button shows "✓ In your collection"

#### Scenario: No token in the app
- **WHEN** the user opens a release page while the app has no Discogs token
- **THEN** a dimmed line under the button says to add a Discogs token in the app to see owned records

#### Scenario: Not owned
- **WHEN** the user opens a release they don't own
- **THEN** no note is shown

### Requirement: Bandcamp page button
On a Bandcamp album or track page, the extension SHALL show its button next to the page's title (floating in a corner when the title can't be found). Its menu SHALL be the same as on a Discogs release: Play, Send to crate and skip passed. New crate… SHALL suggest "Artist - Title" from the page's title. On a Bandcamp label or artist page (`/` or `/music`), its menu SHALL offer only ‹App›: Send label. On a Bandcamp fan's collection or wishlist page (`bandcamp.com/‹user›` or `bandcamp.com/‹user›/wishlist`), its menu SHALL offer only ‹App›: Import collection & wishlist, which sends only the page's address. On other Bandcamp pages, no button SHALL be shown.

#### Scenario: Album page
- **WHEN** the user opens a Bandcamp album page while the app is running and paired
- **THEN** the button offers Play in ‹App› and Send to crate

#### Scenario: Label page
- **WHEN** the user opens `https://analogicalforce.bandcamp.com/music`
- **THEN** the button's menu offers only ‹App›: Send label, and choosing it follows the label in the app without changing its shown crate

#### Scenario: Fan page
- **WHEN** the user opens `https://bandcamp.com/toast3r` and chooses ‹App›: Import collection & wishlist
- **THEN** the app starts importing fan "toast3r", and the extension sent nothing but the address

#### Scenario: Merch page
- **WHEN** the user opens a Bandcamp merch page
- **THEN** no button is shown

### Requirement: Bandcamp links
Right-clicking a link to a Bandcamp album or track page, on any website, SHALL offer Play in ‹App›. A link to a Bandcamp label page (`/` or `/music`) SHALL offer only ‹App›: Send label.

#### Scenario: Album link in a forum
- **WHEN** the user right-clicks a Bandcamp album link and chooses Play in ‹App›
- **THEN** its tracks are merged into the app's shown crate, the first new one plays, and the toolbar button shows success for 3 s

### Requirement: Juno page button
On product, label, artist, genre, chart, search and new release pages of www.juno.co.uk and its regional sites (`es.`, `de.`…, which share product ids), the extension SHALL show its button after the record's Play and Add to playlist buttons on a product page, else next to the page's title (floating in a corner when neither can be found). Its menu SHALL offer Play, Send to crate and skip passed, as on a Discogs release. New crate… SHALL suggest "Artist - Title" on a product page and the page's heading on other pages, at most 40 characters. The menu SHALL say how many records the page shows. When every record shows only one track, it SHALL add "1 track per record: set Show tracks to All for more". When the page can't be read, the menu SHALL say "Couldn't read this Juno page", send nothing and offer no action. On other Juno pages (cart, account, equipment), no button SHALL be shown. The extension SHALL show no menu items on links to Juno pages, since a link's page isn't loaded.

#### Scenario: Product page
- **WHEN** the user opens the page of product 1178909-01 and chooses Play in ‹App›
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

### Requirement: Beatport and Traxsource page buttons
On track, release (Traxsource: title), chart, label, artist, genre and search pages of www.beatport.com and www.traxsource.com, the extension SHALL show its button next to the page's title (floating in a corner when it can't be found). Its menu SHALL offer Play, Send to crate and skip passed, as on a Discogs release, and SHALL say how many tracks the page shows. New crate… SHALL suggest "Artist - Title" on a track page and the page's heading on other pages, at most 40 characters. When the page can't be read, the menu SHALL say "Couldn't read this Beatport page" (or Traxsource), send nothing and offer no action. On other pages of either store (cart, account), no button SHALL be shown, and no menu items SHALL be shown on links to their pages.

#### Scenario: Beatport chart
- **WHEN** the user opens a Beatport chart showing 100 tracks and chooses Play in ‹App›
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
On every page of a supported store that lists records or tracks, the extension SHALL show a small ▶ after each record's (or, on Beatport and Traxsource, each track's) title link, one per record or track and none on its picture, which sends only that one with mode Play (added to the end of the crate on screen and played) without navigating:
- on Discogs: after each marketplace item title link (sending `/sell/item/‹id›`), and after each release or master title link on any page that isn't that release's or master's own page, including search results, labels, artists, lists, wantlists and collections (sending the link's address);
- on Bandcamp: after each album or track title link on `bandcamp.com/search` and on an account's music page (sending the link's address without its query);
- on Juno: after each record's title on listings, including search results (sending that record as the page shows it);
- on Traxsource: after each track's title on listings, including search results (sending that track); on Beatport: after each track's title on listings whose page data holds the track's sample (charts, releases, labels, artists, genres), and not on search results, whose data has none;
- on Bleep: after each record's title on lists, including search results.

Nothing SHALL be read from a row before its ▶ is clicked, apart from finding its title link (and, on Beatport, which tracks the page's data holds, read once per page load). Of several links to one record, the ▶ SHALL follow the one shown (not a hidden copy kept for another layout). The page's own button SHALL be unchanged.

#### Scenario: A seller's shop
- **WHEN** the user clicks ▶ after a record's title on `https://www.discogs.com/seller/ADEPTA_STORE/profile`
- **THEN** that record is added to the crate on screen and its first track plays, and the page doesn't navigate

#### Scenario: Discogs search results
- **WHEN** the user clicks ▶ after "Kerri Chandler – Rain" on `https://www.discogs.com/search/?q=Kerri%20Chandler%20Rain&type=release`
- **THEN** that release's tracks are added to the crate on screen and the first plays, and no other result is sent

#### Scenario: Bandcamp search results
- **WHEN** the user clicks ▶ after an album on `https://bandcamp.com/search?q=Kerri%20Chandler`
- **THEN** that album's tracks are added to the crate on screen and the first plays

#### Scenario: One Juno record
- **WHEN** the user clicks ▶ after a record on a Juno search page listing 50 records
- **THEN** only that record is sent, and it plays

#### Scenario: One Beatport track
- **WHEN** the user clicks ▶ after a track on the Beatport Top 100
- **THEN** only that track is sent, and it plays

#### Scenario: Beatport search
- **WHEN** the user opens a Beatport search page
- **THEN** no ▶ is shown, since its results carry no sample

#### Scenario: A Discogs label page
- **WHEN** the user opens a Discogs label page that also holds hidden copies of its release links
- **THEN** each release shown gets one ▶, after its visible title

#### Scenario: A release's own page
- **WHEN** the user opens a Discogs release page
- **THEN** no ▶ is added after the links to its other versions, and the page's button still plays it

### Requirement: Send to crate lists crates that take sends
Send to crate SHALL list only the crates a send can fill: no Top Sellers crate, label crate (LABELS), Discogs collection or wantlist crate, or friend's crate.

#### Scenario: Special crates left out
- **WHEN** the user opens Send to crate while Diggr has a seller's crate and a followed label
- **THEN** neither is listed, and the Playlist and the user's own crates are

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

### Requirement: Store cart actions
While the app runs, the extension SHALL wait for its pending store cart actions (see `browser-bridge`, Cart requests), perform each one in the user's browser session with that store's own basket requests, and report the outcome. After each action, and when the app starts, it SHALL read the store's basket when it can and report it. It SHALL report "not logged in" when the store says the session has no account, and "failed" for anything it doesn't recognise, never guessing an outcome.

#### Scenario: Unexpected answer
- **WHEN** the extension performs a Bleep add and Bleep answers with an HTML error page
- **THEN** it reads the basket, reports "added" if the format is in it and "failed" otherwise, never guessing

### Requirement: Mix page button
On a YouTube video page (`/watch?v=‹id›`) and a SoundCloud track page (`soundcloud.com/‹user›/‹name›`, not SoundCloud's own pages nor a user's sets, likes or other lists), the extension SHALL show its button next to the page's title. Its menu SHALL offer Play in ‹App› and Send to crate, and SHALL send only the page's address (see `mix-tracklists`). New crate… SHALL suggest "Mix: ‹title›", without the site's own words ("- YouTube", "Stream …", "by ‹uploader›", "| Listen online…").

#### Scenario: A SoundCloud mix
- **WHEN** the user opens `https://soundcloud.com/tracklistings3-0/tracklistings-mixtape-781?in=toast3r/sets/x` and chooses Play in ‹App›
- **THEN** the page's address is sent with mode Play, and the player reads the mix and plays its first track

#### Scenario: A SoundCloud playlist
- **WHEN** the user opens `https://soundcloud.com/toast3r/sets/2023-2025-vinyl-mixes`
- **THEN** no button is shown

