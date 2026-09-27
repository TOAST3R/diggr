## ADDED Requirements

### Requirement: SYNC mode
The player SHALL offer a SYNC mode, off by default and remembered across launches. In SYNC mode, consecutive tracks SHALL be mixed into one another instead of joined by gapless cuts.

#### Scenario: A set instead of a playlist
- **WHEN** SYNC is on and the current track reaches its planned mix point
- **THEN** the next track is mixed in according to the mix plan instead of starting after the last frame of the current one

### Requirement: Held set tempo
The incoming track SHALL be played at the set tempo (the tempo currently heard), with pitch following tempo (vinyl-style), and SHALL keep that tempo for its whole length. Tempos within ±8% of the set tempo, including half or double time, SHALL be matched. In SYNC mode, a next track whose tempo cannot be matched SHALL be skipped in favour of the next track in play order that can be matched, and each skip SHALL be shown briefly; when no remaining track can be matched, the next track SHALL be crossfaded at its own tempo and the set tempo SHALL become its tempo. A track the user starts SHALL never be skipped.

#### Scenario: 124 into 126 BPM
- **WHEN** a set at 124 BPM mixes into a 126 BPM track
- **THEN** the new track plays at 124 BPM from its entry to its end

#### Scenario: Half time
- **WHEN** a set at 172 BPM mixes into an 87 BPM track
- **THEN** the new track plays at 86 BPM (half of the set tempo) and its downbeats align with the set's

#### Scenario: Out of range is skipped
- **WHEN** a set at 128 BPM is followed in the queue by a 150 BPM track and then a 126 BPM track
- **THEN** the 150 BPM track is skipped, the 126 BPM track is mixed in at 128 BPM, and the main window briefly says the 150 BPM track was skipped

#### Scenario: Nothing left to match
- **WHEN** every remaining track in the queue is out of range of a 128 BPM set
- **THEN** the next track is crossfaded without tempo change and the set continues at its tempo

### Requirement: Beat-matched, phrase-aligned mix
A matched mix SHALL start on a phrase boundary of the outgoing track and SHALL keep both tracks' downbeats within 5 ms of each other for its whole length.

#### Scenario: Kicks together
- **WHEN** two tracks are mixed at a matched tempo
- **THEN** every downbeat of the incoming track is audible within 5 ms of a downbeat of the outgoing track until the outgoing track is silent

### Requirement: Mix length
The mix length SHALL be the whole number of 8-bar phrases at the set tempo closest to the mix-length setting (30 s by default), between 8 and 32 bars.

#### Scenario: About 30 seconds
- **WHEN** a set at 128 BPM mixes with the default setting
- **THEN** the mix lasts 16 bars (30 s)

#### Scenario: Fast tempo
- **WHEN** a set at 174 BPM mixes with the default setting
- **THEN** the mix lasts 16 bars (about 22 s)

### Requirement: Mix at the best moment
In SYNC mode, the mix SHALL start at the outgoing track's outro (the first boundary after its last energy rise where the energy falls by at least 4 dB), no earlier than 16 bars after that rise, and not wait for the track's end. The incoming track SHALL enter so that the mix ends where its first energy rise (its drop) begins, skipping the part of its intro before that. Without a clear outro, the mix SHALL start at the latest phrase boundary that leaves room for it. A track started by the user SHALL play from its beginning.

#### Scenario: Outro skipped
- **WHEN** the outgoing track has a 64-bar outro after its last drop and the mix length is 16 bars
- **THEN** the mix starts at the outro's first bar and the last 48 bars of the outro are never played

#### Scenario: Intro skipped
- **WHEN** the incoming track's first drop is at bar 64 and the mix length is 16 bars
- **THEN** the incoming track enters at bar 48 and its drop begins exactly as the mix ends

#### Scenario: User starts a track
- **WHEN** the user double-clicks a track in SYNC mode
- **THEN** it plays from its beginning

### Requirement: EQ mix
The incoming track's low band (below 200 Hz: kick and bass) SHALL be cut by at least 24 dB from its entry until the handover downbeat halfway through the mix; at the handover the low bands SHALL swap within one beat, so the incoming track's kick and bass play and the outgoing track's are muted.

#### Scenario: No doubled kick
- **WHEN** both tracks have kicks on every beat during the first half of a mix
- **THEN** the mixed signal's low-band energy stays within 3 dB of the outgoing track's alone

#### Scenario: Kick swap
- **WHEN** the mix reaches its handover downbeat
- **THEN** within one beat the incoming track's kick is at full level and the outgoing track's kick is cut

### Requirement: Manual kick swap
During a mix, `E` SHALL swap the two tracks' low bands (kick and bass) on the next beat, within one beat; pressing it again SHALL swap them back. Once the user has swapped, the automatic swap at the handover SHALL NOT happen.

#### Scenario: Early swap
- **WHEN** the user presses E four bars into a 16-bar mix
- **THEN** on the next beat the incoming track's kick and bass come in and the outgoing track's are muted, and nothing changes in the low bands at the handover

#### Scenario: Swap back
- **WHEN** the user presses E again one bar later
- **THEN** on the next beat the outgoing track's kick and bass return and the incoming track's are cut

### Requirement: Volume crossfade
Over the mix, the outgoing track SHALL start louder and the balance SHALL change gradually (equal power) until the incoming track is at full volume and the outgoing track is silent. The incoming track's level SHALL be matched to the outgoing track's analyzed loudness during the mix (at most ±6 dB), then return to its own level over 4 bars.

#### Scenario: Gradual balance
- **WHEN** a mix is a quarter of the way through
- **THEN** the outgoing track is at least 6 dB louder than the incoming one, the two are equal at the halfway point, and at the end the outgoing track is silent and the incoming one at full volume

#### Scenario: Quieter master
- **WHEN** the incoming track is mastered 4 dB quieter than the outgoing one
- **THEN** during the mix it is raised by 4 dB so the transition has no drop in loudness

### Requirement: Fallbacks
When a beat grid is unreliable at the mix points, or no remaining track's tempo can be matched, the tracks SHALL be crossfaded at their own tempos (with the same EQ swap). When the analysis is not ready in time, a track is shorter than the mix, repeat is One, or it is the last track with repeat Off, the transition SHALL be a gapless cut. Each fallback SHALL be shown briefly with its reason.

#### Scenario: Not analyzed in time
- **WHEN** the next track's analysis is not ready 2 s before the planned mix and no later phrase fits
- **THEN** the tracks are joined by a gapless cut at the end and the main window briefly says why

### Requirement: Mix now
`Shift+B` SHALL start a mix into the next track at the next phrase boundary of the current track that is at least one bar ahead, whether or not SYNC is on.

#### Scenario: Mix on demand
- **WHEN** the user presses Shift+B in the middle of a phrase
- **THEN** the mix begins at the next phrase boundary

### Requirement: Transport during a mix
During a mix, `B` (next) SHALL cut straight to the incoming track at its current position, seek SHALL cancel the mix and seek within the main track, pause and stop SHALL apply to both tracks, and turning SYNC off SHALL cancel a mix that has not started.

#### Scenario: Next during a mix
- **WHEN** the user presses B during a mix
- **THEN** the outgoing track stops and the incoming track continues from its current position without a gap
