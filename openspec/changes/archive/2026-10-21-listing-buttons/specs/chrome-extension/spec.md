## MODIFIED Requirements

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
