# Spec Delta

## ADDED Requirements

### Requirement: Platter hold
The engine SHALL let the playing track be held and released. A hold SHALL slow it to a stop in 0.4 s, its pitch falling with its speed, after which the output SHALL be silent and no more of the queue SHALL be consumed. A release SHALL bring it back to its speed in 0.3 s from wherever it got to. A hold or a release SHALL start to be heard within 20 ms, with no click, and SHALL cause no underrun.

#### Scenario: Brake
- **WHEN** a track plays at speed 1 and the app holds it
- **THEN** the rate heard falls from 1 to 0 over 0.4 s (±10 ms), the output is silent afterwards, and the queue's position stops advancing

#### Scenario: Spin up
- **WHEN** a held track is released
- **THEN** it plays again from where it stopped, the rate rising from 0 to its speed over 0.3 s, with no gap and no underrun

#### Scenario: Release before it stops
- **WHEN** a track is held and released 0.2 s later
- **THEN** the rate rises again from about 0.5, without having reached 0

#### Scenario: Under SYNC
- **WHEN** a track plays fitted at speed 1.03 and is held, then released
- **THEN** it slows from 1.03 to 0 and comes back to 1.03

#### Scenario: Reaction
- **WHEN** the app holds a playing track
- **THEN** the first slowed audio reaches the device within 20 ms

### Requirement: A hold ends by itself
A hold SHALL end, as a release would, when the app hasn't confirmed it for 0.5 s. A seek, stop, pause, play, next, previous or speed change SHALL end it at once, and the audio that follows SHALL play at its own speed from its first frame.

#### Scenario: Watchdog
- **WHEN** the app holds a track and then stops confirming the hold
- **THEN** the track spins back up 0.5 s after the last confirmation

#### Scenario: Seek ends it
- **WHEN** a held track is sought to 1:00
- **THEN** playback continues from 1:00 at its speed, not held

### Requirement: The stage at rest
While no hold is active or ending, the platter stage SHALL pass the audio through unchanged: the output SHALL be identical, sample for sample, to playback without the stage. The stage SHALL allocate nothing, take no lock and make no system call in the audio callback, and the start, seek and underrun targets SHALL still hold.

#### Scenario: Bit-exact at rest
- **WHEN** a track plays through without a hold
- **THEN** its output equals the output without the platter stage, sample for sample

#### Scenario: Real-time safe
- **WHEN** a hold and a release run in the allocation-counting test
- **THEN** the audio callback allocates nothing
