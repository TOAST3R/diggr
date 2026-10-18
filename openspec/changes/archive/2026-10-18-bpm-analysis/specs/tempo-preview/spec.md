## MODIFIED Requirements

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
