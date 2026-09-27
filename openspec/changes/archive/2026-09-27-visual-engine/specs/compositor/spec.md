## ADDED Requirements

### Requirement: Layer pipeline
Each frame SHALL render: active scene(s) → transition mix → feedback warp (previous frame, decay, warp function) → post effects (bloom, chromatic kick, vignette, grain) → upscale to surface → egui overlay/deck at full resolution.

#### Scenario: Crossfade
- **WHEN** a crossfade is active
- **THEN** both scenes render and are mixed by the transition progress

### Requirement: Frame rate governance
The compositor SHALL hold display refresh rate by adjusting internal render scale between 0.5 and 1.0, pre-lowering scale at the start of crossfades; text overlays SHALL remain at full resolution.

#### Scenario: Heavy scene
- **WHEN** the KIFS scene exceeds 90% of the frame budget for 30 frames
- **THEN** render scale drops until frame time is under budget, with no dropped audio

### Requirement: Never black, never blocking audio
Rendering SHALL only read the clock and snapshots, and SHALL NOT keep state on the GPU that cannot be rebuilt: when the host provides a new GPU device (`init` again), all resources SHALL be re-created and the show SHALL resume at the current musical time.

#### Scenario: New device
- **WHEN** the host re-initializes the visual engine with a new render state
- **THEN** the next frame renders the same scene at the current musical position
