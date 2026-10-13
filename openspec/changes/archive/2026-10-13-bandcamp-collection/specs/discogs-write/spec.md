## MODIFIED Requirements

### Requirement: Refresh from Discogs
With a Discogs token, or once a Bandcamp fan has been imported, right-clicking the wantlist crate or the collection crate in the crate sidebar, and the title-bar crate menu, SHALL offer Refresh wantlist and Refresh collection. Each SHALL make the crate match its sources, in one direction only (neither Discogs nor Bandcamp is changed by it, except for the additions of `bandcamp-collection`):
- Refresh collection SHALL sync the collection (incrementally, as a weekly sync does). Records in the Discogs collection that the crate doesn't hold SHALL then be fetched into it. Entries of records no longer in the collection SHALL leave it, unless the Bandcamp collection holds them;
- Refresh wantlist SHALL read the Discogs wantlist again, even if it was read this session. Records newly wanted SHALL be fetched into the wantlist crate. Records no longer on it, with no change waiting, SHALL leave it, unless the Bandcamp wishlist holds them;
- once a fan has been imported, Refresh collection SHALL also refresh the fan's Bandcamp collection, and Refresh wantlist the fan's Bandcamp wishlist, each only its own (see `bandcamp-collection`). Without a token, they SHALL refresh only those.

Records in both SHALL be left as they are. While Refresh wantlist runs, a "Refresh wantlist" window SHALL show its progress with a progress bar ("Reading your wantlist…", then "Adding records… ‹k› of ‹n›") and a Stop button. Stop while reading SHALL drop the refresh's summary; Stop while adding SHALL work as Refresh collection's (see `discogs-collection`). When the refresh is done, the main window SHALL say what changed ("Collection: 3 new, 1 gone", "Wantlist up to date"). While a refresh runs, its item SHALL read "Refreshing…" and be disabled. A refresh that fails SHALL change nothing and say why. Without a token and no imported fan, the items SHALL NOT be offered.

#### Scenario: Bought and sold on discogs.com
- **WHEN** the collection crate holds release 1001, the Discogs collection now holds 1003 instead, and the user chooses Refresh collection
- **THEN** release 1003's entries come into the crate, release 1001's leave it, and the main window says "Collection: 1 new, 1 gone"

#### Scenario: Removed from the wantlist elsewhere
- **WHEN** releases 1002 and 1006 were removed from the wantlist on discogs.com during the session, and the user chooses Refresh wantlist
- **THEN** their entries leave the wantlist crate, they are no longer wanted, and the main window says "Wantlist: 2 gone"

#### Scenario: Still on the Bandcamp wishlist
- **WHEN** release 1002 was removed from the Discogs wantlist but its album is on the imported Bandcamp wishlist, and the user chooses Refresh wantlist
- **THEN** its entries stay in the wantlist crate

#### Scenario: Wantlist progress
- **WHEN** the user chooses Refresh wantlist and 12 records newly wanted are fetched into the crate
- **THEN** a "Refresh wantlist" window shows "Adding records… 6 of 12" with the bar at 50 % halfway, and Stop ends it, keeping the records that arrived

#### Scenario: Narrow playlist
- **WHEN** the playlist is 400 points wide and the user clicks its title bar
- **THEN** the crate menu offers Refresh wantlist and Refresh collection under its Discogs crates

#### Scenario: Offline
- **WHEN** Discogs is unreachable and the user chooses Refresh collection
- **THEN** the crate is unchanged, and the main window says the refresh failed because Discogs is offline
