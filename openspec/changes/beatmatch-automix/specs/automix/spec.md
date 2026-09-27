## ADDED Requirements

### Requirement: Automix setting
The player SHALL offer an automix mode, off by default and persisted, with a mix length of 8, 16 or 32 bars (default 16). With automix on, consecutive tracks SHALL be mixed instead of joined by a gapless cut.

#### Scenario: Enable automix
- **WHEN** automix is turned on and the current track approaches its end
- **THEN** the next track is mixed in according to the mix plan instead of starting after the last frame

### Requirement: Beat-matched, phrase-aligned mix
A mix SHALL start on a phrase boundary of the outgoing track (preferring the start of its last section), bring in the incoming track at its first downbeat after leading silence, play the incoming track at the outgoing track's tempo when the tempos are within ±8% (including half or double time), and keep both tracks' downbeats aligned for the whole mix.

#### Scenario: 124 into 126 BPM
- **WHEN** a 124 BPM track mixes into a 126 BPM track with a 16-bar mix
- **THEN** the incoming track plays at 124 BPM with its downbeats within 5 ms of the outgoing track's downbeats throughout the 16 bars

#### Scenario: Glide back
- **WHEN** the outgoing track has faded out
- **THEN** the incoming track returns smoothly to 126 BPM over the next 16 bars

### Requirement: Clean crossfade with bass swap
The mix SHALL use an equal-power crossfade over its length and SHALL keep the incoming track's low band cut until the handover downbeat at the mix midpoint, where the low bands swap within one beat.

#### Scenario: No doubled kick
- **WHEN** both tracks have kicks on every beat during the first half of a mix
- **THEN** the mixed signal's low-band energy stays within 3 dB of the outgoing track's alone

### Requirement: Fallbacks
When tempos are incompatible or a beat grid is unreliable at the mix points, the system SHALL fall back to an equal-power crossfade on a phrase boundary without tempo change. When analysis is not ready in time, or the next track is too short, or repeat is One, it SHALL fall back to a gapless cut. Each fallback SHALL be reported to the UI.

#### Scenario: 128 into 174 BPM
- **WHEN** a 128 BPM track is followed by a 174 BPM track
- **THEN** they are joined by a phrase-aligned crossfade without tempo change, and the UI briefly shows why

### Requirement: Mix now
`Shift+B` SHALL start a mix into the next track at the next phrase boundary of the current track that is at least one bar ahead, regardless of the automix setting.

#### Scenario: Mix on demand
- **WHEN** the user presses Shift+B in the middle of a phrase
- **THEN** the mix begins at the next phrase boundary

### Requirement: Transport during a mix
During a mix, pause SHALL pause both tracks, seek SHALL cancel the mix and seek within the main track, next SHALL complete the handover at once, and stop SHALL stop both.

#### Scenario: Next during a mix
- **WHEN** the user presses B during a mix
- **THEN** the outgoing track stops and the incoming track continues from its current position without a gap
