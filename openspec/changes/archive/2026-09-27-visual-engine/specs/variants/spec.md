## ADDED Requirements

### Requirement: Variant files
A variant SHALL be stored as `visuals/variants/<scene>/<name>.ron` with scene id, parent, seed, parameter values, route overrides, tags, and rating.

#### Scenario: Load variant
- **WHEN** the director picks variant "ice" of `julia_tunnel`
- **THEN** the scene renders with the variant's parameter values

### Requirement: Mutation
`M` SHALL apply a small seeded Gaussian mutation to mutable parameters in normalized space, and `Shift+M` SHALL apply a large mutation that may also swap one optional route.

#### Scenario: Small mutate
- **WHEN** the user presses M
- **THEN** the look changes subtly and non-mutable params are unchanged

### Requirement: Keep, undo, rate
`K` SHALL save the current state (including fader positions) as a new auto-named variant with its parent recorded; `Backspace` SHALL undo to the previous state in the session; keys `1`–`5` SHALL set the current variant's rating.

#### Scenario: Keep after tweaking
- **WHEN** the user adjusts faders and presses K
- **THEN** a new variant file exists with those values and parent set to the previous variant

#### Scenario: Undo
- **WHEN** the user presses M three times then Backspace twice
- **THEN** the look equals the state after the first mutation
