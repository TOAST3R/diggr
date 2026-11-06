# Spec Delta

## MODIFIED Requirements

### Requirement: SYNC control
The turntable SHALL always be shown in the band, between the mini player and the waveform (see `player-window` "Full-screen window"), with no × and no title bar, and SHALL have a SYNC button, off by default and drawn lit while on. While SYNC is off the turntable SHALL be drawn dimmed: its platter turns with the speed heard, its digits show the playing record's own BPM, and its pitch fader, 8%/16% button and RESET do nothing. Turning SYNC on or off SHALL say what it does in the main window: "SYNC on: every track in the list plays at ‹target› BPM (within ±‹range›%)" (the target with up to two decimals, trailing zeros dropped), "SYNC on: every track will play at this one's BPM once it's analyzed" while it waits for one, or "SYNC off: every track plays at its own tempo". Turning it on SHALL set the target as follows:
- to the playing track's BPM when that is known;
- to the last target when nothing plays;
- when a track plays whose BPM isn't known yet, SYNC SHALL wait for it. The playing track SHALL go to the front of the analysis queue, the target and record BPM SHALL read "?", and the target SHALL become the track's BPM as soon as it is known. Waiting SHALL end, keeping the last target, when the analysis finds no tempo or another track starts. A track started while SYNC is on and whose BPM is being analyzed or queued SHALL show its record BPM as "?" until known, and then be fitted to the target.

The turntable SHALL have a pitch fader whose centre is the playing record's own tempo, up faster and down slower across the range, setting the target to the nearest 0.01 BPM kept within the range. Scrolling over it, and Shift+↑ / Shift+↓ with the player focused (with the playlist focused they extend the selection), SHALL step the target by 1 BPM. While SYNC waits for a BPM, the fader, scroll, Shift+↑ / Shift+↓ and RESET SHALL leave the target unchanged and show "Analyzing the BPM, wait a moment". The turntable SHALL show the target, its whole BPM in big digits and its hundredths in small figures beside them, the playing record's push ("+2.4%") and own BPM, a RESET button taking the record's own tempo, an 8%/16% range button, and a line saying what SYNC is doing (see "SYNC state on the turntable"). Its platter SHALL turn with the speed heard, and its strobe dots SHALL stand still while the record is fitted to the target and drift otherwise. SYNC's state, target (to 0.01 BPM) and range SHALL be remembered across launches; a target saved as a whole BPM by an older version SHALL load as that BPM. The help SHALL list its keys.

#### Scenario: Says what it does
- **WHEN** a 124 BPM track plays at ±8% and the user turns SYNC on
- **THEN** the main window says "SYNC on: every track in the list plays at 124 BPM (within ±8%)"

#### Scenario: Turn on
- **WHEN** a 124 BPM track plays and the user turns SYNC on
- **THEN** the turntable lights up, the target reads 124 with the fader at its centre, and the track keeps playing at its own speed

#### Scenario: Always there
- **WHEN** the app starts with SYNC off, and again after SYNC is turned on and off
- **THEN** the turntable is shown in the band at the same place and size each time, dimmed while SYNC is off, with the record's own BPM in its digits

#### Scenario: Off does nothing
- **WHEN** SYNC is off and the user drags the pitch fader or clicks RESET
- **THEN** the target and the speed heard are unchanged

#### Scenario: Turn on before the BPM is known
- **WHEN** an unanalyzed 170 BPM track plays, the last target was 112, and the user turns SYNC on
- **THEN** the target and record BPM read "?", and once the track's analysis finishes the target reads 170 with the fader at its centre

#### Scenario: Fader while analyzing
- **WHEN** SYNC waits for the playing track's BPM and the user drags the pitch fader
- **THEN** the target still reads "?", the speed heard doesn't change, and the message "Analyzing the BPM, wait a moment" shows

#### Scenario: Fader to the top
- **WHEN** a 123 BPM track plays with SYNC on at ±8% and the user pulls the fader to the top
- **THEN** the target reads 132.84 (123 + 8%, kept within the range) and the track is heard at 132.84 BPM

#### Scenario: Raise the target
- **WHEN** SYNC is on at 124 and the user presses Shift+↑ twice
- **THEN** the target reads 126, and the track is heard at 126 BPM within 50 ms

## ADDED Requirements

### Requirement: Fine tempo steps
With SYNC on and the record's BPM known, Shift+drag on the pitch fader SHALL move the target by 0.01 BPM per skin pixel from its value when the drag (or Shift) began, up faster; Shift+scroll over the fader SHALL step it by 0.01 BPM per 4 skin pixels of scroll; Alt+↑ / Alt+↓ (Option on macOS), with the player focused, SHALL step it by 0.01 BPM, and plain ↑ / ↓ by 0.1 BPM. Every step SHALL keep the target within the range and be heard within 50 ms.

#### Scenario: Shift+drag
- **WHEN** SYNC is on at 124.00 over a 124 BPM record and the user Shift+drags the fader up by 35 skin pixels
- **THEN** the target reads 124.35 and the track is heard at 124.35 BPM

#### Scenario: Alt+arrow
- **WHEN** SYNC is on at 126.00 with the player focused and the user presses Alt+↑ three times
- **THEN** the target reads 126.03

#### Scenario: Plain arrows
- **WHEN** SYNC is on at 126.00 with the player focused and the user presses ↑
- **THEN** the target reads 126.10

#### Scenario: Shift+scroll
- **WHEN** SYNC is on at 126.00 and the user Shift+scrolls up 8 skin pixels over the fader
- **THEN** the target reads 126.02

#### Scenario: Kept within the range
- **WHEN** SYNC is on at 133.92 over a 124 BPM record at ±8% and the user presses Alt+↑
- **THEN** the target stays 133.92

#### Scenario: Coarse steps unchanged
- **WHEN** SYNC is on at 126.35 and the user presses Shift+↑
- **THEN** the target reads 127.35

### Requirement: Typing the tempo
With SYNC on and the record's BPM known, clicking the turntable's big digits SHALL open them for typing a tempo of up to three digits, a point and two decimals. Enter SHALL set the target to the typed value kept within the record's range, saying so when it had to be moved; Esc, a click elsewhere, or SYNC turning off SHALL cancel. While typing, no other shortcut SHALL act.

#### Scenario: Type a tempo
- **WHEN** SYNC is on over a 124 BPM record at ±8%, the user clicks the digits, types 126.35 and presses Enter
- **THEN** the target reads 126.35 and the track is heard at 126.35 BPM

#### Scenario: Out of range
- **WHEN** SYNC is on over a 124 BPM record at ±8%, and the user types 150 and presses Enter
- **THEN** the target reads 133.92, and a message says it was kept within ±8% of the record

#### Scenario: A comma and Backspace
- **WHEN** the user clicks the digits and types 1, 2, 6, a comma, 3, 5 and 9, then presses Backspace
- **THEN** after the 9 the digits show 126.35 (a third decimal is refused), and after Backspace 126.3

#### Scenario: Cancel
- **WHEN** the user clicks the digits, types 99 and presses Esc
- **THEN** the target is unchanged

#### Scenario: Shortcuts wait
- **WHEN** the digits are open for typing and the user presses Space
- **THEN** playback doesn't pause, and the typed text is unchanged

#### Scenario: Not while off
- **WHEN** SYNC is off and the user clicks the digits
- **THEN** nothing opens for typing
