## MODIFIED Requirements

### Requirement: Low cost
The spectrogram window SHALL repaint only while playing (at the display rate in live mode, at most 10 times a second in track mode for the playhead), while detail is arriving, or on input.

#### Scenario: Track mode, paused
- **WHEN** the window is in track mode and playback is paused with no input
- **THEN** the window does not repaint
