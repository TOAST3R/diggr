## MODIFIED Requirements

### Requirement: A found track says so
The tooltip of an entry from a searched track SHALL say "Preview: to search", "Preview: searching YouTube…" while its search runs, "Preview: found by search (‹video title›)" or "Preview: not found by search". While its search runs, its row SHALL end with "SEARCHING…" in place of its length, the row's text pulsing between its dimmed and full colour about once a second; the window SHALL repaint for the pulse only while a search runs and one of its rows is shown. The preview worker SHALL say when each search starts and ends. The found video's title SHALL be saved with the crate.

#### Scenario: Hover a found track
- **WHEN** the pointer rests on "The 89th Passenger - Paper Wings" after its search found a video
- **THEN** the tooltip says "Preview: found by search (The 89th Passenger – Paper Wings [AF069])"

#### Scenario: A track being searched
- **WHEN** the search for "The 89th Passenger - T6" starts
- **THEN** its row ends with "SEARCHING…", pulsing, and its tooltip says "Preview: searching YouTube…"; when the search ends, its row goes back to its waiting look, or to its found or not-found state

#### Scenario: Nothing searched, nothing repainted
- **WHEN** no search runs and nothing plays for 10 s
- **THEN** no row pulses and the UI performs no repaints
