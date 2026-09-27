## ADDED Requirements

### Requirement: Spectral overview
The track overview pass SHALL also produce a fixed-size spectral overview of the whole track (4096 time columns × 256 log-spaced frequency rows from 20 Hz to Nyquist, in dB), cached with the overview.

#### Scenario: Tone lands on the right row
- **WHEN** a track contains a steady 1 kHz sine
- **THEN** the spectral overview's strongest row in every column is the row whose band contains 1 kHz

#### Scenario: Size is independent of length
- **WHEN** overviews are computed for a 3-minute track and a 2-hour mix
- **THEN** both spectral overviews have the same size

### Requirement: On-demand detail
The system SHALL compute the spectrogram of any time range at a requested column and row resolution in the background, from the track's native-rate audio, delivering it progressively.

#### Scenario: Zoomed detail
- **WHEN** a 2-second range is requested at 1000 columns
- **THEN** the result resolves events 10 ms apart and arrives without blocking playback or the UI

### Requirement: Lossy cutoff detection
The system SHALL estimate the frequency where a track's content ends and SHALL flag a lossless-container file as "likely from a lossy source" only when that cutoff is below 19.5 kHz and has a steep edge.

#### Scenario: Transcoded file
- **WHEN** a FLAC file contains music brick-wall filtered at 16 kHz
- **THEN** the cutoff is reported as about 16 kHz and the file is flagged as likely lossy

#### Scenario: Dark but genuine
- **WHEN** a FLAC file contains music whose highs roll off gradually below 12 kHz
- **THEN** the file is not flagged
