# Spec Delta

## MODIFIED Requirements

### Requirement: Open the spectrogram
Pressing `S` or clicking the mini player's spectrum button SHALL open a separate, resizable spectrogram window for the current track; pressing S or the spectrum button again, or closing the window, SHALL close it. Its size and mode SHALL persist, and the shortcuts help SHALL list `S`.

#### Scenario: Open and close
- **WHEN** the user presses S while a track plays
- **THEN** a spectrogram window for that track opens, and pressing S again closes it

#### Scenario: Spectrum button
- **WHEN** the user clicks the mini player's spectrum button, then clicks it again
- **THEN** the spectrogram window opens, then closes
