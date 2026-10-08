## Why

Everything in this product — the classic UI, the music analysis, and the fractal visualizer — depends on an audio engine that starts instantly, never glitches, and publishes an exact "what is at the speaker right now" clock. Classic 2000s desktop players earned their reputation on streaming, chunked playback; we need the same foundation in Rust before anything else can be built or synced.

## What Changes

- New cargo workspace with `crates/platform`, `crates/audio`, and `apps/native`.
- Three platform seams (`AudioSink`, `Spawner`, `FileSource`) with native implementations, so a web target can be added later without touching the core.
- Streaming decode of MP3, FLAC, WAV, OGG Vorbis and AAC/M4A via symphonia, resampled to the device rate, fed through a lock-free SPSC ring buffer.
- Audio output device opened once at startup and kept running (emitting silence when idle) so pressing play never pays device-open cost.
- Transport: play, pause, stop, seek, next/previous, volume, balance.
- Gapless playback: the next track is opened and pre-decoded ~30 s before the current track ends.
- Playback clock published lock-free from the audio callback: track id, frame position, host timestamp, and output latency.
- 10-band graphic equalizer + preamp (cascaded biquad peaking filters) applied in the audio callback so EQ changes are audible immediately.
- Audio tap: post-EQ samples, stamped with frame position, delivered to non-real-time consumers (visualizer, spectrum) without ever blocking the callback.
- Resilience: corrupt frames skipped, output device loss/switch recovered, no allocation or locks in the callback.
- Track metadata (artist, title, album, duration) exposed for the UI and overlay.

## Capabilities

### New Capabilities
- `platform-seams`: AudioSink / Spawner / FileSource abstractions and their native implementations.
- `audio-playback`: decoding, transport, gapless/pre-warm, always-open device, resilience, metadata.
- `playback-clock`: lock-free publication of the audible playback position for A/V sync.
- `equalizer`: 10-band EQ + preamp, real-time safe, instantly applied.
- `audio-tap`: post-EQ, position-stamped sample stream for visual and analysis consumers.

### Modified Capabilities
<!-- none: greenfield project -->

## Impact

- New crates: `platform`, `audio`; new binary `apps/native` (headless test harness until `classic-ui` lands).
- Dependencies: symphonia, rubato, cpal, rtrb, audio_thread_priority (or equivalent).
- Every later change (`classic-ui`, `music-analysis`, `visual-engine`, `web-target`) consumes the public API defined here: `Engine` commands, `ClockReader`, `TapReader`, `TrackInfo`.
