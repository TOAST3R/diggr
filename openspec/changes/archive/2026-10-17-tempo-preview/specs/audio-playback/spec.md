## ADDED Requirements

### Requirement: Varispeed
The engine SHALL play any track at a speed from 0.84 to 1.16 set for it, by resampling (pitch follows speed). A change to the playing track's speed SHALL be heard within 50 ms, without a click, a gap or an underrun: the old audio keeps playing until the new is queued, then a 2 ms crossfade joins them. Each queued track SHALL carry its own speed from its first frame, so gapless playback holds across tracks at different speeds. The audio callback SHALL stay real-time safe. At speed 1 the output SHALL match the previous resampler's within 90 dB SNR. Start (< 30 ms), seek (< 50 ms) and zero underruns SHALL hold at any speed in the range.

#### Scenario: Change heard fast
- **WHEN** the playing track's speed changes from 1.00 to 1.04
- **THEN** the new speed is heard within 50 ms, with no click and no underrun

#### Scenario: Gapless at two speeds
- **WHEN** a track at speed 1.03 ends and the next plays at 0.97
- **THEN** the next starts with no gap and no click

#### Scenario: Benchmark
- **WHEN** `diggr --bench --speed 1.04` runs on the reference Mac
- **THEN** worst start < 30 ms, worst seek < 50 ms, worst speed change < 50 ms, and 0 underruns
