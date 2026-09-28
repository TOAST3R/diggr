## MODIFIED Requirements

### Requirement: Four bundled scenes
The system SHALL ship `julia_tunnel` (2D fragment), `liquid_feedback` (feedback-based fragment), `kifs_cathedral` (raymarched 3D fragment), `flame` (compute fractal flame), `polar_life` (polar cellular automaton drawn as a tunnel), and `coral_tunnel` (reaction-diffusion drawn as a tunnel), each with at least two variants and appropriate tags.

#### Scenario: Compute path
- **WHEN** the director selects `flame`
- **THEN** it renders via a compute pass on both native and Chrome WebGPU

#### Scenario: Calm available
- **WHEN** a breakdown rule requests `tag("calm")`
- **THEN** at least one bundled variant matches

#### Scenario: Automaton path
- **WHEN** the director selects `polar_life`
- **THEN** it steps its grid in compute passes and draws it on both native and Chrome WebGPU

#### Scenario: Automata hold the frame rate
- **WHEN** `polar_life` or `coral_tunnel` plays fullscreen on an M2 at native Retina resolution
- **THEN** it holds 60 fps at a render scale of 0.75 or more

## ADDED Requirements

### Requirement: Polar life scene
`polar_life` SHALL be an automaton scene that:
- starts from a random soup of live cells after every reset, so the tunnel is full from the first frame;
- gives birth on its inner ring where a band rises above its own recent level (an onset), and where kicks scatter seeds;
- evolves every step with a Life-like rule over the 8 neighbours (alive, dying through a number of decay states, dead), and cells older than `max_age` steps die;
- with `section_rules` on (the default), runs its own rule in builds (Brian's Brain), drops (Star Wars) and breakdowns (Day & Night), and the variant's rule elsewhere;
- pushes the whole grid one ring outward every `shift_every` steps (at the defaults of 8 steps per beat and a push every step: 8 rings per beat), and lunges forward on kicks;
- draws each cell as a bevelled tile of a log-polar tunnel with the vanishing point at the centre, lit from the vanishing point, coloured by cell age and birth band, with a halo from living cells into the gaps and tails behind cells as they fly outward;
- with `tube` at 1, bends the tunnel into a 3D pipe whose far end sways over 8 bars, lit by a headlight and fading with distance;
- reacts to every hit (a kick, and by default every beat too) with interference: for an instant rows of the picture jump sideways, bands of rings tear, colours split and static flickers, and a random band of rings in the grid gets cells flipped, which the rule then carries outward.

In drop sections it SHALL step faster than elsewhere, following the drop's energy: ×2 for an average drop and ×4 while a drop's energy is near the track's maximum, changing on the beat as the energy moves. The drawing SHALL glide continuously between pushes. The scene SHALL carry the `tunnel` and `automaton` tags.

#### Scenario: Full from the start
- **WHEN** `polar_life` is reset
- **THEN** its first step fills the grid with a soup of live cells at the `soup` density (35% by default)

#### Scenario: Music injects life
- **WHEN** a kick plays on every beat at 8 steps per beat
- **THEN** live cells appear on ring 0 on kicks and reach the outer rings within `rings` × `shift_every` + 32 steps

#### Scenario: Silence empties the tunnel
- **WHEN** after the kicks the input is silent for `rings` × `shift_every` + `max_age` steps, with the default rule
- **THEN** no live or dying cells remain in the grid

#### Scenario: Interference on hits
- **WHEN** a beat lands while the music has energy and `beat_hits` is on, even if the analysis found no kick there
- **THEN** the frames within about 0.2 beats show the interference, and a band of rings in the grid has flipped cells

#### Scenario: Drop speeds up the tunnel
- **WHEN** playback enters a drop section at the track's median energy
- **THEN** from the next beat on, `polar_life` runs 16 steps per beat instead of 8, and returns to 8 when the drop ends

#### Scenario: Powerful drop escalates
- **WHEN** a drop's energy climbs to the track's maximum and then eases back to the median
- **THEN** the speed goes ×2 → ×4 → ×2 on beat boundaries during the drop

#### Scenario: Smooth motion
- **WHEN** `polar_life` runs at 144 fps with `steps_per_beat: 8` at 120 BPM
- **THEN** the drawn rings move outward on every frame, not only on the frames where a push happens

#### Scenario: A breakdown keeps the tunnel alive
- **WHEN** playback enters a breakdown with `section_rules` on
- **THEN** the grid switches to the breakdown rule on the next step and still holds live cells

### Requirement: Coral tunnel scene
`coral_tunnel` SHALL be an automaton scene that:
- runs Gray-Scott reaction-diffusion (substrate A and growth B, `feed` and `kill` rates, the 3×3 Laplacian) several passes per step, with its values kept within 0..1;
- stores A with enough precision near rest that the known regimes (coral, dividing cells) sustain themselves;
- flows outward through the tunnel, taking in fresh substrate at the centre, with a closed outer edge;
- seeds growth where a spectrum band rises above its own recent level, in a random band of rings on each hit, and with a trickle of spores near the centre that grows with the music's energy;
- draws the growth as a glossy relief lit from the vanishing point, with the same tube option as `polar_life`;
- ships a calm coral variant and a dividing-cells variant.

#### Scenario: A kick splash grows
- **WHEN** a fresh grid with no seeds gets one kick, then 400 passes of silence
- **THEN** growth remains in the grid, and every value stays within 0..1

#### Scenario: Cells survive the flow
- **WHEN** the dividing-cells variant plays for 60 steps without music
- **THEN** growth covers part of every band of rings, not only the centre or the rim
