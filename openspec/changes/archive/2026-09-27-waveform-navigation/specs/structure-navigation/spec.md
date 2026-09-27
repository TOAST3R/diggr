## ADDED Requirements

### Requirement: Section jumps
`]` SHALL jump to the start of the next section and `[` to the start of the current section, or the previous one when pressed within the first bar of a section. The keys SHALL work in the player window and in fullscreen.

#### Scenario: Next section
- **WHEN** the user presses ] during the second section of an analyzed track
- **THEN** playback continues from the start of the third section

### Requirement: Drop jump
`Shift+]` SHALL jump to the next section boundary whose energy rises by at least 4 dB, falling back to the next section when there is none.

#### Scenario: Skip the breakdown
- **WHEN** the next boundary is a quieter breakdown and the one after it rises by 8 dB
- **THEN** Shift+] jumps to the boundary after the breakdown

### Requirement: Quantized, seamless jumps
Structure jumps SHALL take effect exactly on a downbeat, namely the first upcoming downbeat that the engine has not yet sent to the device, sample-accurately and without audible gap or click. The pending jump SHALL be shown on the waveform. When the beat grid is unreliable, or no candidate downbeat can be reached, the jump SHALL happen immediately instead.

#### Scenario: On the beat
- **WHEN** the user presses ] halfway through a bar at 128 BPM
- **THEN** the jump happens exactly at the next downbeat, and the target's first downbeat follows with no gap

#### Scenario: Close to the bar line
- **WHEN** the user presses ] less than half a second before a downbeat
- **THEN** the jump happens exactly at the following downbeat

### Requirement: Bar loops
`L` SHALL toggle a loop over the current section (at most 32 bars), and `Shift+L` SHALL cycle a loop of 4, 8 or 16 bars starting at the current downbeat. The loop SHALL be gapless and shown on the waveform.

#### Scenario: Loop a 4-bar phrase
- **WHEN** the user presses Shift+L once
- **THEN** the next 4 bars repeat seamlessly until the user presses L, seeks outside them, or the track changes
