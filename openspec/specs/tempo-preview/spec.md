# tempo-preview Specification

## Purpose
SYNC: hear every track at one target tempo, the way it will sit in a set. A turntable section with a pitch fader sets the tempo; each track plays fitted to it, vinyl-style (pitch follows speed), within a range, while everything else keeps working in the track's own time.
## Requirements
### Requirement: SYNC control
The main window SHALL have a SYNC button, off by default. Turning it on SHALL open a turntable section under the main window; turning it off, by SYNC or the turntable's ×, SHALL close it. Turning it on SHALL set the target as follows:
- to the playing track's BPM when that is known;
- to the last target when nothing plays;
- when a track plays whose BPM isn't known yet, SYNC SHALL wait for it. The playing track SHALL go to the front of the analysis queue, the target and record BPM SHALL read "?", and the target SHALL become the track's BPM as soon as it is known. Waiting SHALL end, keeping the last target, when the analysis finds no tempo or another track starts. A track started while SYNC is on and whose BPM is being analyzed or queued SHALL show its record BPM as "?" until known, and then be fitted to the target.

The turntable SHALL have a pitch fader whose centre is the playing record's own tempo, up faster and down slower across the range, setting the target in whole BPM kept within the range. Scrolling over it, and Shift+↑ / Shift+↓ with the player focused (with the playlist focused they extend the selection), SHALL step the target by 1 BPM. While SYNC waits for a BPM, the fader, scroll, Shift+↑ / Shift+↓ and RESET SHALL leave the target unchanged and show "Analyzing the BPM, wait a moment". The turntable SHALL show the target, the playing record's push ("+2.4%") and own BPM, a RESET button taking the record's own tempo, an 8%/16% range button, and an OWN TEMPO lamp. Its platter SHALL turn with the speed heard, and its strobe dots SHALL stand still while the record is fitted to the target and drift otherwise. SYNC's state, target and range SHALL be remembered across launches. The help SHALL list its keys.

#### Scenario: Turn on
- **WHEN** a 124 BPM track plays and the user turns SYNC on
- **THEN** the turntable opens, the target reads 124 with the fader at its centre, and the track keeps playing at its own speed

#### Scenario: Turn on before the BPM is known
- **WHEN** an unanalyzed 170 BPM track plays, the last target was 112, and the user turns SYNC on
- **THEN** the target and record BPM read "?", and once the track's analysis finishes the target reads 170 with the fader at its centre

#### Scenario: Fader while analyzing
- **WHEN** SYNC waits for the playing track's BPM and the user drags the pitch fader
- **THEN** the target still reads "?", the speed heard doesn't change, and the message "Analyzing the BPM, wait a moment" shows

#### Scenario: Fader to the top
- **WHEN** a 123 BPM track plays with SYNC on at ±8% and the user pulls the fader to the top
- **THEN** the target reads 132 (not 133, which would leave the range) and the track is heard at 132 BPM

#### Scenario: Raise the target
- **WHEN** SYNC is on at 124 and the user presses Shift+↑ twice
- **THEN** the target reads 126, and the track is heard at 126 BPM within 50 ms

### Requirement: Each track fitted to the target
With SYNC on, a track SHALL play at the speed `target / BPM`, choosing among its BPM, double and half the one giving the speed closest to 1, vinyl-style (pitch follows speed). When that speed differs from 1 by more than the range (±8%, or ±16% when chosen), or the track has no BPM, the track SHALL play at its own speed and its row SHALL show "(own tempo)". The next track's speed SHALL be set before it starts, so it starts gaplessly at its fitted speed.

#### Scenario: Fitted
- **WHEN** SYNC is on at 126 and the next track is 122 BPM
- **THEN** it starts gaplessly at speed 1.033, heard at 126 BPM

#### Scenario: Half time
- **WHEN** SYNC is on at 126 and a track is analysed at 64 BPM
- **THEN** it plays at speed 0.984 (fitted to 128), not 1.97

#### Scenario: Out of range
- **WHEN** SYNC is on at 126 with range ±8%, and a track is 140 BPM
- **THEN** it plays at its own speed, and its row shows "(own tempo)"

### Requirement: Speed is only heard
Under any speed, the shown BPM, the waveform, section jumps and loops, analysis, previews' preparation and the spectrogram SHALL work in the track's own time, as at speed 1. While a track plays at a speed other than 1, the main window SHALL show the difference as a percentage ("+2.4%"), and nothing when it plays at its own speed.

#### Scenario: Jumps on the beat
- **WHEN** a track plays at speed 1.04 and the user presses ]
- **THEN** playback jumps on the track's next downbeat, exactly as at speed 1

#### Scenario: Shown speed
- **WHEN** a 123 BPM track plays with SYNC at 126
- **THEN** the main window shows "+2.4%", and its row still shows "(123 BPM)"
