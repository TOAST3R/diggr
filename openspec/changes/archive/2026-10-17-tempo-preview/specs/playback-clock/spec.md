## ADDED Requirements

### Requirement: Clock under varispeed
Under any speed, the clock SHALL report the audible source position (track seconds), derived from the source frame stamped on each output block rather than accumulated, and the playing speed. It SHALL stay within 1 ms of the true source position over 10 minutes at speed 1.06, and SHALL stay monotonic across a speed change.

#### Scenario: No drift
- **WHEN** a track plays 10 minutes at speed 1.06
- **THEN** the clock's position is within 1 ms of the source position being heard

#### Scenario: Speed change
- **WHEN** the speed changes from 1.00 to 0.96 mid-track
- **THEN** the reported position never goes backwards, and advances at 0.96 s per second afterwards
