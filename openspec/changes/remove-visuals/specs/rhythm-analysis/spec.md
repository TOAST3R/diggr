## MODIFIED Requirements

### Requirement: Band onsets
The analyzer SHALL detect kick, snare, and hi-hat onsets as timestamped events, and SHALL use them to estimate tempo. They SHALL NOT be kept in the score, which no reader needs them from.

#### Scenario: Four-on-the-floor
- **WHEN** a house track with a kick on every beat is analyzed
- **THEN** the analyzer's onset detection reports a kick within ±20 ms of each audible kick, and the score holds no drum events
