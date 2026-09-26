## ADDED Requirements

### Requirement: Gapless repeat modes
The engine SHALL support repeat modes Off, All and One. With All, playback SHALL continue from the last queue entry to the first; with One, the current track SHALL loop. Both SHALL be gapless and SHALL pre-warm the track that actually plays next.

#### Scenario: Repeat one
- **WHEN** repeat is One and the track ends
- **THEN** the same track starts again with no inserted silence

#### Scenario: Repeat all wraps
- **WHEN** repeat is All and the last queue entry ends
- **THEN** the first entry follows gaplessly, and `next` on the last entry also wraps to the first
