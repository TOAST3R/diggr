## MODIFIED Requirements

### Requirement: Lock-free consumption
Consumers SHALL read the score through immutable snapshots that are swapped atomically, with no locks on the render path.

#### Scenario: Render-thread read
- **WHEN** the player reads the score every frame (waveform, structure keys) while the analyzer appends
- **THEN** reads never block
