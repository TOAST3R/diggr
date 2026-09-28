## MODIFIED Requirements

### Requirement: Track info display
The main section SHALL show a scrolling "N. Artist - Title (m:ss)" line, bitrate (kbps), sample rate (kHz), and mono/stereo indicators. For an entry from Discogs, the line SHALL continue with its side, catalog number, year and a for-sale summary: "K for sale from ‹lowest price›", "none for sale", or nothing when the numbers aren't known.

#### Scenario: Long title scrolls
- **WHEN** the title text exceeds the display width
- **THEN** it scrolls horizontally

#### Scenario: Discogs details
- **WHEN** entry 3, "Nightcraft - Glasshouse" (6:12), plays from side A1 of catalog number LT-012 (1994), with 6 copies for sale from €9.00
- **THEN** the line reads "3. Nightcraft - Glasshouse (6:12) · A1 · LT-012 · 1994 · 6 for sale from €9.00", in the skin's capitals

### Requirement: Keyboard shortcuts
The system SHALL support Z (previous), X (play), C (pause), V (stop), B (next), ←/→ (seek ∓5 s), ↑/↓ (volume), F (fullscreen visuals), H or F1 (shortcuts help), W (waveform section), S (spectrogram window), [ and ] (previous/next section), Shift+] (next energy rise), L (section loop), Shift+L (4/8/16-bar loop), Y (keep), N (pass), I (open the for-sale page), and Cmd+V, or Ctrl+V on Linux and Windows (paste a Discogs page).

#### Scenario: Classic keys
- **WHEN** the user presses B during playback
- **THEN** the next track starts

#### Scenario: Structure keys
- **WHEN** the user presses ] during playback of an analyzed track
- **THEN** playback moves to the next section on the next downbeat

#### Scenario: Spectrogram key
- **WHEN** the user presses S in the player window
- **THEN** the spectrogram window opens

#### Scenario: Dig keys
- **WHEN** the user presses N while a track from Discogs plays
- **THEN** the track is marked passed and the next track starts

#### Scenario: Paste a page
- **WHEN** the user presses Cmd+V with a Discogs label address on the clipboard
- **THEN** the label's tracks are added to the shown crate
