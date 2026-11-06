# Spec Delta

## ADDED Requirements

### Requirement: Clock under a hold
While a track is held, braking or spinning up, the clock SHALL report the audible position in track time, advancing at the rate heard, and SHALL report that rate as its speed (0 while held). The position SHALL never go backwards, SHALL stay within 1 ms of the audible position, and SHALL continue from the held position on release.

#### Scenario: Held
- **WHEN** a track has been held to a stop
- **THEN** successive reads return the same position and a speed of 0

#### Scenario: Braking
- **WHEN** a track is braking
- **THEN** the position advances more slowly each read, never backwards

#### Scenario: Release
- **WHEN** a track held at 1:23.400 is released
- **THEN** the position continues from 1:23.400
