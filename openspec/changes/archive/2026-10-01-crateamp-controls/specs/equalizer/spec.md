## ADDED Requirements

### Requirement: Resonant low-pass filter
The equalizer SHALL have an LP knob, in the EQ section's top row, that sweeps a resonant low-pass filter (a slight peak at the cutoff) over everything that plays, from 20 kHz down to 60 Hz. With the knob fully right, the filter SHALL be off, leaving the audio unchanged. Double-clicking the knob SHALL turn it off. The knob's tooltip SHALL show the cutoff ("LP 2.4 kHz") or "LP off". Moving it, or switching it on or off, SHALL be click-free and audible within 20 ms. The filter SHALL apply before the visualizer tap, so the visuals follow the sweep. It SHALL be real-time safe like the equalizer: no allocation, lock or syscall in the audio callback. The knob SHALL start off at every launch.

#### Scenario: Sweep
- **WHEN** the user turns the knob from fully right to halfway during playback
- **THEN** the highs fade smoothly within 20 ms, with no click and zero underruns

#### Scenario: Off is transparent
- **WHEN** the knob is fully right
- **THEN** the output is identical, sample for sample, to the output without the filter

#### Scenario: Reset
- **WHEN** the user double-clicks the knob
- **THEN** the filter turns off and the tooltip reads "LP off"

#### Scenario: Not remembered
- **WHEN** the app is quit with the knob at halfway and started again
- **THEN** the knob is fully right and the filter is off
