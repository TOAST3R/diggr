# Spec Delta

## MODIFIED Requirements

### Requirement: Transport controls
The system SHALL support play, pause, resume, stop, seek (to an absolute time), next, previous, volume (0–100%), and balance (full left to full right). A seek, next or previous while audio plays SHALL fade the old audio out over 5 ms and the new audio in over 5 ms, so neither stops nor starts with a click; play from stopped and gapless playback SHALL be unchanged.

#### Scenario: Seek
- **WHEN** the user seeks to 2:00 in a 4:00 track
- **THEN** audio resumes from 2:00 within 50 ms; the pre-seek audio fades out over 5 ms and none of it is heard after that

#### Scenario: Pause and resume
- **WHEN** playback is paused and resumed
- **THEN** audio resumes from the exact frame where it paused

#### Scenario: No click at a seek
- **WHEN** a 1 kHz sine at half scale plays and the user seeks to 5 s
- **THEN** no two consecutive output samples differ by more than the sine itself does (0.08), and there is no underrun
