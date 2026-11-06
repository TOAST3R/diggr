# Spec Delta

## MODIFIED Requirements

### Requirement: Beatport tracks into a crate
Each track SHALL become an entry titled "Artist - Title (Mix)", leaving out a mix named "Original Mix", with the artists joined by ", ". Each entry SHALL carry its length, the release's catalogue number, label and image, the track's BPM and key, and the Beatport track. Tracks SHALL be merged into the target crate as Juno tracks are (see `juno-intake`): matched by normalized artist and title, narrowed by catalogue number. A match that plays is skipped, a match that is unavailable gets the Beatport sample, and the rest are added in the page's order, honouring skip passed. Beatport sends SHALL use the modes of Discogs sends (Play, Enqueue, Crate). The main window SHALL say how many tracks were added, fixed and skipped. Adding SHALL take less than 200 ms for a 500-track send and SHALL cause zero underruns.

#### Scenario: Enqueue a chart
- **WHEN** a 100-track Beatport chart is enqueued while crate "Friday" is shown and empty
- **THEN** "Friday" holds 100 entries in chart order, each showing its BPM before any preview is downloaded

#### Scenario: Fixes a failed track
- **WHEN** a crate holds "Two Shell - Infinite Now" from Discogs marked "not found", with the same catalogue number as the Beatport release sent
- **THEN** that entry gets the Beatport sample and plays, and no second entry is added
