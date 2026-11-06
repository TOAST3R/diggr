# Spec Delta

## ADDED Requirements

### Requirement: Beat lamp
The turntable SHALL show a lamp beside its state line that lights on each beat of the playing
track's analysed beat grid, on the first frame drawn at or after that beat is heard, brighter on
downbeats, and fades out over 150 ms. It SHALL follow the speed heard, and SHALL stay dark while
nothing plays, while the platter is held still, and when the track's beats aren't known.

#### Scenario: On the beat
- **WHEN** a 120 BPM track with a known beat grid plays
- **THEN** the lamp lights twice a second, each time on the first frame at or after a beat is heard, brighter every fourth beat on the downbeat

#### Scenario: Under SYNC
- **WHEN** that track plays fitted at 126 BPM
- **THEN** the lamp lights 2.1 times a second, with the beats as heard

#### Scenario: No beats yet
- **WHEN** a track plays whose analysis has no beats yet
- **THEN** the lamp stays dark
