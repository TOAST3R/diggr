## MODIFIED Requirements

### Requirement: Track info display
The main section SHALL show a scrolling "N. (catno) Artist: Title (T BPM) (m:ss)" line, bitrate (kbps), sample rate (kHz), and mono/stereo indicators. The name SHALL follow the playlist's display format. For an entry from Discogs, the line SHALL continue with its side, year and a for-sale summary: "K for sale from ‹lowest price›", "none for sale", or nothing when the numbers aren't known. The catalog number SHALL NOT be repeated in that continuation.

#### Scenario: Long title scrolls
- **WHEN** the title text exceeds the display width
- **THEN** it scrolls horizontally

#### Scenario: Discogs details
- **WHEN** entry 3, "Nightcraft" / "Glasshouse" (6:12) at 124 BPM, plays from side A1 of catalog number LT-012 (1994), with 6 copies for sale from €9.00
- **THEN** the line reads "3. (LT-012) Nightcraft: Glasshouse (124 BPM) (6:12) · A1 · 1994 · 6 for sale from €9.00", in the skin's capitals
