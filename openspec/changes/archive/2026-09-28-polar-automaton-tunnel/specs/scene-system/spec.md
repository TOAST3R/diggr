## MODIFIED Requirements

### Requirement: Scene package
A scene SHALL consist of `scene.wgsl` and `scene.ron` in `visuals/scenes/<id>/`; the manifest SHALL declare name, tags, kind (fragment, compute, or automaton), typed parameters with default/range/mutable, macro mappings, routes, and optional feedback settings. An automaton kind SHALL declare its grid size (`theta` and `rings`, each 8..=1024) and `steps_per_beat` (greater than 0, at most 16), and MAY declare `preroll` (steps run after a reset, 16 by default, at most 1024) and `substeps` (`fn rule` passes per step, 1 by default, 1..=32). A manifest outside these ranges SHALL be rejected with an error naming the field.

#### Scenario: Add a scene
- **WHEN** a new valid scene folder is added
- **THEN** it becomes available to the director and deck without recompiling the app

#### Scenario: Invalid automaton grid
- **WHEN** a manifest declares `kind: Automaton(theta: 4, rings: 64, steps_per_beat: 4.0)`
- **THEN** the manifest is rejected with an error naming `theta`, and on hot reload the previous version keeps running

## ADDED Requirements

### Requirement: Persistent automaton state
An automaton scene SHALL have a state grid of `theta` × `rings` cells (four floats each) that persists from frame to frame, private to each drawn instance of the scene. Crossfades, the other layer, render-scale changes and post effects SHALL NOT alter it. The author's `fn rule` SHALL compute a cell's next state from the previous state only, with θ wrapping around and rings outside the grid reading as empty.

#### Scenario: Same scene on both layers
- **WHEN** the director crossfades from one variant of an automaton scene to another variant of the same scene
- **THEN** each layer steps its own grid, and neither grid contains the other's cells

#### Scenario: Render scale changes
- **WHEN** the render scale drops from 1.0 to 0.6 while an automaton scene plays
- **THEN** the grid keeps its declared size and its contents are unchanged by the resize

#### Scenario: Angular wrap
- **WHEN** a single live cell at θ = 0 is stepped with a rule that copies the neighbour at θ + 1
- **THEN** the cell appears at θ = `theta` − 1

### Requirement: Steps on musical time
An automaton SHALL advance `steps_per_beat × rate` steps per beat, evenly spaced within the beat, independent of frame rate.
- **Rate:** 1 unless the scene declares a `step_rate` param. If it does, the rate SHALL be the param's modulated value, sampled once when each beat starts, snapped to one of 0.5, 1, 2 and 4, and held for that beat. The snap SHALL use hysteresis in octaves: the rate moves to the level nearest `log2(step_rate)` only when that is more than 0.6 from the current level, and it MAY jump several levels at once. The rate MAY change on any beat, any number of times.
- **Catch-up:** when several steps are due in one frame, it SHALL run them in that frame, up to 8 per frame, and carry the rest over.
- **Musical time moving back or standing still:** it SHALL NOT step while musical time stands still, and SHALL NOT step backwards when musical time moves back by less than one step.

#### Scenario: Frame-rate independence
- **WHEN** the same 8-beat stretch at 120 BPM with `steps_per_beat: 4` is rendered at 30 fps and at 144 fps
- **THEN** both runs execute 32 steps, and each step happens in the first frame whose musical time reaches it

#### Scenario: Faster in a drop
- **WHEN** a scene routes `step_rate` to 2 in drop sections, with `steps_per_beat: 4`, and plays 4 beats of groove followed by 4 beats of drop
- **THEN** 16 steps run in the groove and 32 in the drop, at both 30 and 144 fps

#### Scenario: Rate held within a beat
- **WHEN** `step_rate` moves from 1 to 2 halfway through a beat
- **THEN** the rest of that beat keeps rate 1, and rate 2 applies from the next beat

#### Scenario: Several changes in one drop
- **WHEN** during a drop `step_rate` is 2 for 4 beats, 4 for 4 beats, then 2 again for 4 beats, with `steps_per_beat: 4`
- **THEN** 8, 16 and 8 steps per beat run in those stretches

#### Scenario: No flicker at a boundary
- **WHEN** `step_rate` alternates between 1.35 and 1.5 on successive beats at rate 1
- **THEN** the rate stays at 1, because neither value is more than 0.6 octaves from level 0

#### Scenario: Paused
- **WHEN** playback is paused for 5 s
- **THEN** no steps run during the pause

#### Scenario: Phase-lock correction
- **WHEN** musical time moves back by 0.1 beat because of a phase-lock correction at `steps_per_beat: 4`
- **THEN** no step runs and the grid is unchanged until musical time passes the last step again

### Requirement: Automaton reset and pre-roll
An automaton's grid SHALL be cleared and pre-rolled for its `preroll` steps (16 by default) when its scene instance is created or its program is rebuilt, when the track changes, and when musical time jumps by more than 2 beats in either direction. The pre-roll SHALL respect the per-frame step limit, and the playback path SHALL never wait for it.

#### Scenario: Seek
- **WHEN** the user seeks from 0:30 to 2:10 while an automaton scene plays
- **THEN** the grid is cleared, the 16 pre-roll steps (the default) run over the next two frames, and stepping then continues from 2:10's musical time

#### Scenario: Long pre-roll
- **WHEN** a scene declares `preroll: 256` and is reset
- **THEN** the pre-roll runs 8 steps per frame over the next 32 frames

#### Scenario: Hot reload
- **WHEN** the author saves a valid change to the scene's `fn rule`
- **THEN** the grid restarts from empty with a pre-roll, and the new rule is visible within 500 ms

### Requirement: Automaton helpers
For automaton scenes, the engine SHALL generate:
- the step entry point and the ping-pong bindings;
- `cell` (a cell of the previous state: θ wraps, rings outside the grid are empty);
- `grid`, `tick`, `substeps` and `since_reset` (steps since the last reset: 0 on the first step, which starts from an empty grid, so a rule can seed its initial state there);
- `tick_phase` (the fraction of the current step elapsed, for gliding between steps);
- `inject_level` (the 19 spectrum bars mapped onto θ in 0..1 with mirror symmetry and linear interpolation);
- `state_at` (the grid sampled from `fn scene` at a fractional ring depth and θ).

The author SHALL only write `fn rule` and `fn scene`.

With `substeps` N, every step SHALL run `fn rule` N times, each pass reading the previous pass's state, all with the same `tick`. Step counting, catch-up and reset SHALL be unchanged.

#### Scenario: Substeps
- **WHEN** a scene declares `substeps: 12` and one step is due
- **THEN** its rule runs 12 passes in that frame, and `substeps()` returns 12

### Requirement: Tunnel helpers in the prelude
The prelude SHALL provide `tube_hit`, which maps a screen point to the depth, angle and distance from the axis of a tube whose far end sits at a given offset, and `tube_facing`. With a zero offset, `tube_hit` SHALL match the flat tunnel mapping (depth ∝ 1 / screen radius, angle = screen angle).

#### Scenario: Flat when straight
- **WHEN** `tube_hit` is called with `bend` = 0
- **THEN** the angle equals the screen angle and the distance from the axis equals the screen radius

#### Scenario: Author writes only the rule and the look
- **WHEN** an author writes `fn rule` using `cell`, `inject_level` and `tick`, and `fn scene` using `state_at` and `tick_phase`
- **THEN** the scene compiles and validates without any binding boilerplate

#### Scenario: Mirror injection
- **WHEN** only the lowest spectrum bar is at full level
- **THEN** `inject_level` is highest at θ = 0 and θ = 1 and falls to the level of the neighbouring bars toward θ = 0.5
