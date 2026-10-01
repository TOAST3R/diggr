# equalizer Specification

## Purpose
TBD - created by archiving change audio-core. Update Purpose after archive.
## Requirements
### Requirement: 10-band graphic equalizer
The system SHALL provide a 10-band peaking equalizer at 60, 170, 310, 600, 1k, 3k, 6k, 12k, 14k, and 16k Hz with ±12 dB per band, plus a ±12 dB preamp, and an on/off switch.

#### Scenario: Band boost
- **WHEN** the 60 Hz band is set to +12 dB with EQ on
- **THEN** a 60 Hz test tone's output level increases by 12 dB (±0.5 dB)

#### Scenario: EQ off
- **WHEN** EQ is switched off
- **THEN** output is bit-identical to input (before volume/balance)

### Requirement: Immediate, click-free changes
EQ changes SHALL become audible within one device buffer and SHALL be applied with a short ramp that produces no audible clicks.

#### Scenario: Fast slider sweep
- **WHEN** a band slider is swept rapidly from −12 to +12 dB during playback
- **THEN** the change is heard without clicks or zipper noise

### Requirement: Real-time safe processing
EQ processing SHALL run inside the audio callback without allocation, and bands whose center exceeds the Nyquist frequency SHALL be bypassed.

#### Scenario: Low sample rate device
- **WHEN** the device runs at 22.05 kHz
- **THEN** the 12k, 14k, and 16k bands are bypassed and no instability occurs

### Requirement: Presets
The system SHALL store and load named EQ presets (all bands + preamp) and ship a small default set (Flat, Rock, Pop, Dance, Techno, Full Bass, Full Treble).

#### Scenario: Load preset
- **WHEN** the "Techno" preset is loaded
- **THEN** all band values and preamp are set to the preset's values

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

