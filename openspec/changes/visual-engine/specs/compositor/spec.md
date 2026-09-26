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
Rendering SHALL only read the clock and snapshots; GPU device loss SHALL be recovered by recreating resources and resuming at the current musical time.

#### Scenario: Sleep and wake
- **WHEN** the machine sleeps and wakes during fullscreen
- **THEN** visuals resume within 2 s at the correct musical position
