## ADDED Requirements

### Requirement: Deck panel
In fullscreen only, `D` SHALL toggle a fader deck with six macro faders (intensity, chaos, stretch, speed, hue, feedback) and up to twelve faders for the current scene's mutable parameters, plus scene/variant name, rating, and mutate/keep/undo controls.

#### Scenario: Toggle
- **WHEN** the user presses D in fullscreen
- **THEN** the deck appears over the visuals; pressing D again hides it

### Requirement: Live value display
Each fader SHALL show its base value and its live post-modulation value, updating every frame.

#### Scenario: Watching modulation
- **WHEN** a kick route modulates zoom
- **THEN** the zoom fader's live indicator pumps with the kicks

### Requirement: Automation and return
Each fader SHALL be in AUTO or MANUAL mode; touching a fader makes it MANUAL; after release it SHALL glide back to AUTO after its RETURN time (1 beat, 1 bar, 4 bars, phrase, or ∞), landing on a bar line. A global RETURN setting SHALL apply unless a fader overrides it.

#### Scenario: Touch and return
- **WHEN** RETURN is 1 bar and the user pushes intensity up then releases
- **THEN** intensity holds, then glides back to automation, arriving on a bar line

#### Scenario: Latch
- **WHEN** a fader's RETURN is ∞
- **THEN** it stays at the user's value until re-armed to AUTO

### Requirement: Quantized speed
The speed macro SHALL snap to ¼, ½, 1, 2, and 4×.

#### Scenario: Speed snap
- **WHEN** the speed fader is released between 1× and 2×
- **THEN** it snaps to the nearer of the two
