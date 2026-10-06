## ADDED Requirements

### Requirement: Record artist on entries
Each entry from Discogs SHALL carry its record's credited artist, as Discogs shows it for the release (or the master, or the listing): "Various" for a compilation, joined names for a shared credit. Every entry of a record SHALL carry the same record artist, whatever its own track credit. The record artist SHALL be saved with the crate and SHALL survive reordering, sending to another crate and restarting. For a Discogs entry saved without a record artist, showing its crate SHALL fill it from the cached release or master data, and SHALL NOT make any request to do so. The entry's own artist (the track's credit) SHALL NOT change.

#### Scenario: Compilation
- **WHEN** a release credited to "Various", with tracks by Nightcraft and Lumen, is expanded
- **THEN** every entry carries the record artist "Various", and the entries' own artists stay Nightcraft and Lumen

#### Scenario: Older crate
- **WHEN** a crate saved before record artists existed is shown, and its releases' data is in the disk cache
- **THEN** its entries carry their record artists, and no request is sent to Discogs
