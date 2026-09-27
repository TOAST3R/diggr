## MODIFIED Requirements

### Requirement: Monotonic and track-aware
The clock SHALL be monotonic within a track and SHALL switch track id at the exact audible frame of a gapless boundary. During a mix it SHALL report the outgoing track until the handover downbeat is audible, then the incoming track in that track's own time, advancing at the incoming track's playback speed.

#### Scenario: Gapless boundary
- **WHEN** playback crosses from track N into N+1 gaplessly
- **THEN** the clock reports N+1 with position 0 at the moment N+1's first frame becomes audible, not when it was decoded

#### Scenario: Seek discontinuity
- **WHEN** a seek occurs
- **THEN** the clock reports a discontinuity flag and the new position once post-seek audio is audible

#### Scenario: Handover during a mix
- **WHEN** a mix reaches its handover downbeat
- **THEN** the clock switches to the incoming track at the position of that downbeat in the incoming track

#### Scenario: Track time at a changed tempo
- **WHEN** the incoming track plays at ratio 0.95 after the handover
- **THEN** its reported position advances by 0.95 s per second heard, so its beats stay aligned with what is audible
