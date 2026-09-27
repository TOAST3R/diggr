# spectral-analysis Specification

## Purpose
TBD - created by archiving change spectrogram-window. Update Purpose after archive.
## Requirements
### Requirement: Spectral overview
The track overview pass SHALL also produce a bounded spectral overview of the whole track, for the mid and side channels: at most 4096 time columns × 256 log-spaced frequency rows from 20 Hz to Nyquist, in dB, cached with the overview. It SHALL work when the track's length is not known in advance, and the complete overview (waveform and spectral) of a 2-hour mix SHALL stay within 16 MB.

#### Scenario: Tone lands on the right row
- **WHEN** a track contains a steady 1 kHz sine
- **THEN** the spectral overview's strongest row in every column is the row whose band contains 1 kHz, to within one FFT bin

#### Scenario: Size is bounded regardless of length
- **WHEN** overviews are computed for a 3-minute track, a 2-hour mix, and a track whose length is not known in advance
- **THEN** each spectral overview has between 2048 and 4096 columns, and the 2-hour mix's complete overview uses no more than 16 MB

### Requirement: On-demand detail
The system SHALL compute the spectrogram of any time range at a requested column and row resolution in the background, from the track's native-rate audio, delivering it progressively. The FFT size SHALL follow the zoom (the smallest power of two at least 4× the column hop, from 512 to 8192 points), so short ranges resolve time and long ranges resolve frequency.

#### Scenario: Zoomed detail
- **WHEN** a 2-second range is requested at 1000 columns
- **THEN** the result resolves events 10 ms apart and arrives without blocking playback or the UI

### Requirement: Lossy cutoff detection
The system SHALL estimate the frequency where a track's content ends and SHALL flag a file decoded with a lossless codec (FLAC, PCM, ALAC) as "likely from a lossy source" only when that cutoff is below 19.5 kHz and has a steep edge (more than 40 dB drop within 500 Hz).

#### Scenario: Transcoded file
- **WHEN** a FLAC file contains music brick-wall filtered at 16 kHz
- **THEN** the cutoff is reported as about 16 kHz and the file is flagged as likely lossy

#### Scenario: Dark but genuine
- **WHEN** a FLAC file contains music whose highs roll off gradually below 12 kHz
- **THEN** the file is not flagged

#### Scenario: Lossy codec is never flagged
- **WHEN** an AAC `.m4a` file has a 16 kHz cutoff
- **THEN** the cutoff is reported but the file is not flagged

