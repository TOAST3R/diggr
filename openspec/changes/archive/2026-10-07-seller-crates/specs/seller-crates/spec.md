## ADDED Requirements

### Requirement: Top Sellers list
The app SHALL keep a list of sellers, each with its own crate named "Seller: ‹username›". The list and each seller's dig criteria and last dig time SHALL be saved in the config folder and survive restarts. A seller crate SHALL still be recognised as that seller's after it is renamed.

#### Scenario: Survives a restart
- **WHEN** the list holds decks.de and logon, and the app is restarted
- **THEN** TOP SELLERS lists decks.de and logon with their crates and entries

#### Scenario: Renamed
- **WHEN** the user renames "Seller: decks.de" to "decks"
- **THEN** it stays under TOP SELLERS, and double-clicking it digs decks.de

### Requirement: First list from purchases
The first time a Discogs token is saved, or at the first launch with a token after this change, the app SHALL fill the list once:
- it SHALL read the user's purchases and take the 100 most recent orders;
- it SHALL rank their sellers by number of orders, ties going to the most recent order;
- it SHALL keep the first 10 sellers that have at least one copy for sale, each with an empty crate that hasn't been dug.

It SHALL NOT fill the list again after that, even if the user removes every seller. It SHALL run behind other Discogs work, and SHALL NOT delay launch or playback. If it fails, it SHALL be tried again at the next launch. With no purchases, the list SHALL start empty.

#### Scenario: Ranking
- **WHEN** the last 100 purchases include 11 orders from www.hhv.de, 6 from Housevinyl.nl, 2 from musicman1 (nothing for sale) and 1 each from 30 other sellers
- **THEN** the list holds 10 sellers, www.hhv.de first and Housevinyl.nl second, and not musicman1

#### Scenario: Tie
- **WHEN** ADEPTA_STORE and logon both have 3 orders, the last from ADEPTA_STORE in April 2026 and the last from logon in March 2026
- **THEN** ADEPTA_STORE comes before logon

#### Scenario: Once only
- **WHEN** the list was filled, the user removed every seller, and the token is saved again
- **THEN** the list stays empty

#### Scenario: Offline at first
- **WHEN** Discogs is unreachable when the token is saved, and reachable at the next launch
- **THEN** the list is filled at the next launch

### Requirement: Add seller
Right-clicking the TOP SELLERS heading SHALL offer Add seller…, which opens a modal with one field. The field SHALL accept:
- a seller's Discogs page (`/seller/‹name›/profile`, `/seller/‹name›`, or `/user/‹name›`, with or without `www.`, a language prefix or a query);
- a bare username.

Once the user stops typing for 500 ms, the modal SHALL look the seller up and show their username and how many copies they have for sale, or say the user doesn't exist. Add SHALL be enabled only for a seller that exists. Adding SHALL put the seller at the top of the list, show its crate (empty), and open the dig flow ("Double-click to dig"). A seller already in the list SHALL instead be offered "Already in Top Sellers: Refresh" (see "Refreshing").

#### Scenario: Paste a seller's page
- **WHEN** the user pastes `https://www.discogs.com/seller/decks.de/profile` in Add seller…
- **THEN** the modal shows "decks.de · 40,728 for sale", and Add puts decks.de at the top of TOP SELLERS

#### Scenario: Unknown seller
- **WHEN** the user types "no-such-seller-xyz"
- **THEN** the modal says the seller doesn't exist, and Add is disabled

#### Scenario: Already there
- **WHEN** the user pastes the page of logon, which is in the list
- **THEN** the modal offers Refresh instead of Add

### Requirement: Remove seller
Right-clicking a seller crate SHALL offer Remove seller…. It SHALL ask for confirmation, then remove the seller from the list and delete its crate, as Delete crate does (playback from that crate stops).

#### Scenario: Remove
- **WHEN** the user chooses Remove seller… on "Seller: logon" and confirms
- **THEN** logon leaves TOP SELLERS and its crate is deleted

### Requirement: Click shows, double-click digs
A single click on a seller crate SHALL show it from what is saved, without any request to Discogs. A never-dug crate SHALL show "Double-click to dig ‹seller›". A double-click SHALL:
- for a crate never dug: count the seller's copies for sale with one request, then open the Dig modal (1,000 copies or fewer) or the Narrow down modal (more than 1,000);
- for a crate last dug more than 24 hours ago: refresh it with its saved criteria;
- for a crate dug within 24 hours: show it, with no request.

Without a token, a double-click SHALL open the Connect to Discogs dialog (see `discogs-write`) instead.

#### Scenario: Click
- **WHEN** the user clicks "Seller: decks.de", last dug yesterday
- **THEN** its saved copies and tracks are shown, and no request is sent to Discogs

#### Scenario: First dig of a small seller
- **WHEN** the user double-clicks "Seller: ADEPTA_STORE", never dug, which has 475 copies for sale
- **THEN** the Dig modal says "475 copies" and offers Dig; Dig fills the crate

#### Scenario: Stale
- **WHEN** the user double-clicks "Seller: logon", last dug 30 hours ago
- **THEN** the crate is refreshed with its saved criteria

#### Scenario: Fresh
- **WHEN** the user double-clicks "Seller: logon", last dug 2 hours ago
- **THEN** the crate is shown and nothing is sent to Discogs

### Requirement: Narrow down
When a seller has more than 1,000 copies for sale, the Narrow down modal SHALL offer these criteria:
- search text, which Discogs applies;
- "newest N" (N from 50 to 1,000);
- format group (Vinyl, File, CD, Cassette, Other);
- a price range;
- a minimum media condition;
- countries the copies ship from.

It SHALL show the number of copies matching the current criteria, updated within one frame of each change once the listings it needs are read. While listings are read, it SHALL show progress and offer Cancel. Dig SHALL be enabled only when 1 to 1,000 copies match. When the seller has more than 10,000 copies for sale, the modal SHALL say that Discogs only shows the first 10,000 and suggest search text or newest N. The criteria SHALL be saved with the seller and reused by every refresh. Right-clicking a dug seller crate SHALL offer Narrow down… to change them.

#### Scenario: Search narrows
- **WHEN** decks.de has 40,728 copies and the user types "techno"
- **THEN** the count shows 8,389 and Dig stays disabled

#### Scenario: Newest
- **WHEN** the user picks newest 500 for decks.de
- **THEN** the count shows 500 and Dig is enabled

#### Scenario: Local criteria
- **WHEN** Wavelength_Records has 1,851 copies, the listings are read, and the user ticks Vinyl and sets a price of at most €15
- **THEN** the count shows the number of vinyl copies at €15 or less within one frame, and Dig is enabled once it is 1,000 or fewer

#### Scenario: Beyond 10,000
- **WHEN** the user double-clicks "Seller: www.hhv.de", which has 159,183 copies for sale
- **THEN** the modal says Discogs shows only the first 10,000 and suggests search text or newest N

### Requirement: Seller crate contents
Digging SHALL bring in every matching copy for sale. Each distinct release SHALL come in once, as a record expanded like any sent page: its clips, tracks to search and the vinyl-first rules (see `discogs-intake`, `preview-search`), fetched nearest the playhead first and cached. Each copy SHALL carry its listing id, price and currency, media and sleeve condition, the country it ships from, and the time it was listed. A seller crate SHALL be grouped by record until the user toggles it. The main window SHALL show progress in records ("decks.de: 120 of 450 records").

#### Scenario: Three copies of one record
- **WHEN** a seller has three copies of release 123456 at €9.00, €12.00 and €18.00
- **THEN** the crate holds release 123456's tracks once, and three copies of it

#### Scenario: Grouped by default
- **WHEN** a seller crate is shown for the first time
- **THEN** it is grouped by record

### Requirement: Refreshing
Right-clicking a dug seller crate, in the sidebar or the title-bar crate menu, SHALL offer Refresh seller. It SHALL refresh the crate whenever it was last dug. A refresh SHALL read the seller's listings again with the saved criteria, and then:
- bring in new copies, expanding their records if the crate doesn't hold them;
- mark copies no longer listed as SOLD, keeping them;
- update changed prices.

The main window SHALL then summarise ("decks.de: 14 new, 6 sold, 3 cheaper", or "decks.de: up to date"). If more than 1,000 copies now match, the refresh SHALL change nothing and open the Narrow down modal with the saved criteria. While it runs, the item SHALL read "Refreshing…" and be disabled. A failed refresh SHALL change nothing and say why.

#### Scenario: New and sold
- **WHEN** since the last dig, a seller listed 2 new copies and sold 1, and the user chooses Refresh seller
- **THEN** the 2 copies come in, the sold one is marked SOLD and stays, and the main window says "‹seller›: 2 new, 1 sold"

#### Scenario: Grew past the limit
- **WHEN** a seller dug with no criteria at 950 copies now has 1,120, and the crate is refreshed
- **THEN** the crate is unchanged and the Narrow down modal opens

#### Scenario: Offline
- **WHEN** Discogs is unreachable and the user chooses Refresh seller
- **THEN** the crate is unchanged, and the main window says the refresh failed because Discogs is offline

### Requirement: SOLD
A copy no longer for sale SHALL show a SOLD badge, drawn in the same style as OWNED, and its row SHALL be dimmed. A record whose copies are all sold SHALL be dimmed and marked SOLD on its record row, and SHALL stay in play order. Sold copies SHALL be listed after unsold ones.

#### Scenario: Sold copy
- **WHEN** a refresh finds the €12.00 copy of release 123456 gone
- **THEN** that copy row shows SOLD and is dimmed, and is listed after the other copies

#### Scenario: All sold
- **WHEN** every copy of a record is sold
- **THEN** its record row is dimmed and shows SOLD, and its tracks still play

### Requirement: Seller pages
A seller's page (`/seller/‹name›/profile`, `/seller/‹name›`, `/user/‹name›` with no further path) sent by pasting it in the player window or from the browser extension SHALL add that seller, as Add seller does, or refresh it if it is already in the list. Either way, its crate SHALL be shown. It SHALL NOT add tracks to any other crate.

#### Scenario: Paste a seller page
- **WHEN** the user presses Cmd+V with `https://www.discogs.com/seller/logon/profile` copied, and logon isn't in the list
- **THEN** logon is added at the top of TOP SELLERS, its crate is shown, and the dig flow opens

#### Scenario: Already there
- **WHEN** the same page is pasted and logon is in the list, last dug 3 days ago
- **THEN** "Seller: logon" is shown and refreshed

### Requirement: Sellers never touch playback
Reading the list, counting, digging, refreshing and the modals SHALL run off the UI thread and the audio path. They SHALL cause zero underruns, and SHALL NOT delay a start or a seek.

#### Scenario: Dig while playing
- **WHEN** a track plays and the user digs a seller of 1,000 copies
- **THEN** playback has zero underruns, and the UI keeps responding within one frame
