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

