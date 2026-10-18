## MODIFIED Requirements

### Requirement: Track info display
The main section SHALL show a scrolling "N. (catno) Artist: Title (T BPM) (m:ss)" line, bitrate (kbps) and sample rate (kHz). It SHALL NOT show mono or stereo indicators. The name SHALL follow the playlist's display format. For an entry from Discogs, the line SHALL continue with its side, year and a for-sale summary: "K for sale from ‹lowest price›", "none for sale", or nothing when the numbers aren't known. The catalog number SHALL NOT be repeated in that continuation.

#### Scenario: Long title scrolls
- **WHEN** the title text exceeds the display width
- **THEN** it scrolls horizontally

#### Scenario: Discogs details
- **WHEN** entry 3, "Nightcraft" / "Glasshouse" (6:12) at 124 BPM, plays from side A1 of catalog number LT-012 (1994), with 6 copies for sale from €9.00
- **THEN** the line reads "3. (LT-012) Nightcraft: Glasshouse (124 BPM) (6:12) · A1 · 1994 · 6 for sale from €9.00", in the skin's capitals

#### Scenario: No channel indicators
- **WHEN** a mono file plays, and then a stereo file
- **THEN** the main section shows neither a MONO nor a STEREO indicator

### Requirement: Transport and sliders
The main section SHALL provide previous, play, pause, stop, next, shuffle, and repeat buttons, a seek bar, a volume slider, waveform, EQ and playlist toggles, and a SYNC button, all wired to the Engine or the view. It SHALL NOT provide an open (Eject) button. The waveform, EQ and playlist toggles SHALL be drawn as icons, not labels, and SHALL sit in one row with the SYNC button, which SHALL stay labelled SYNC. Shuffle and repeat SHALL be drawn as icons in the transport row; repeat one SHALL show the repeat icon with a small 1. Each icon SHALL be drawn lit while its setting is on, and each SHALL have a tooltip naming it. The waveform toggle SHALL sit where the balance slider was, SHALL show or hide the waveform section exactly as `W` does, and SHALL be drawn lit while the waveform section shows. The main section SHALL NOT offer a balance control, and playback SHALL always be centred.

#### Scenario: Drag seek bar
- **WHEN** the user drags the seek bar and releases at 75%
- **THEN** the Engine seeks to 75% of the track duration on release

#### Scenario: Volume change is immediate
- **WHEN** the volume slider is moved
- **THEN** the loudness change is audible within 20 ms

#### Scenario: Waveform button
- **WHEN** the waveform section is hidden and the user clicks the waveform button
- **THEN** the waveform section appears, the button is drawn lit, and the setting is remembered across launches

#### Scenario: Saved balance reset
- **WHEN** the app is launched with a settings file from an older version that holds a balance of -0.6
- **THEN** playback is centred

#### Scenario: Repeat icon cycles
- **WHEN** repeat is off and the user clicks the repeat icon twice
- **THEN** it is drawn lit after the first click (repeat all), and lit with a small 1 after the second (repeat one)

#### Scenario: No Eject
- **WHEN** the main section is drawn
- **THEN** it has no open (Eject) button, and ⌘O, the footer's ADD and drag-and-drop still add files to the shown crate

#### Scenario: SYNC beside the toggles
- **WHEN** the main section is drawn
- **THEN** the SYNC button, labelled SYNC, sits in the row of the waveform, EQ and playlist icons, and clicking it turns SYNC on or off as before
