# Spec Delta

## ADDED Requirements

### Requirement: Holding the platter
Pressing and holding the primary button on the turntable's platter while a track plays SHALL hold the record (see `audio-playback` "Platter hold"), and releasing the button, or the pointer leaving the window, SHALL release it. While the button is held the app SHALL confirm the hold at least every 0.2 s. It SHALL work with SYNC on or off; while SYNC is off the platter SHALL not be dimmed. Pressing it while nothing plays SHALL do nothing.

#### Scenario: Hand on the record
- **WHEN** a track plays and the user presses and holds the platter
- **THEN** the record slows to a stop, the platter's mark and strobe dots slow and stop with it, and the time stops

#### Scenario: Let go
- **WHEN** the user releases the platter
- **THEN** the record spins back up and plays on from where it stopped

#### Scenario: With SYNC off
- **WHEN** SYNC is off and the user holds the platter
- **THEN** the record stops just the same, and the platter is drawn undimmed

#### Scenario: Nothing playing
- **WHEN** nothing plays and the user presses the platter
- **THEN** nothing happens
