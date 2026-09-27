## ADDED Requirements

### Requirement: Scene package
A scene SHALL consist of `scene.wgsl` and `scene.ron` in `visuals/scenes/<id>/`; the manifest SHALL declare name, tags, kind (fragment or compute), typed parameters with default/range/mutable, macro mappings, routes, and optional feedback settings.

#### Scenario: Add a scene
- **WHEN** a new valid scene folder is added
- **THEN** it becomes available to the director and deck without recompiling the app

### Requirement: Prelude and generated bindings
The engine SHALL prepend a prelude providing `struct Music`, the previous-frame texture, and helper functions (complex math, noise, palettes, rotations, SDF and raymarch utilities), and SHALL generate `struct Params` and its uniform layout from the manifest.

#### Scenario: Author writes only the look
- **WHEN** an author writes `fn scene(uv, m, p) -> vec4f` using `p.zoom` and `m.beat`
- **THEN** the scene compiles without any binding boilerplate

### Requirement: Hot reload with safe fallback
On native, edits to scene files SHALL be applied while playing; invalid WGSL or RON SHALL NOT interrupt the show — the last valid version keeps running and an error toast shows file, line, and message.

#### Scenario: Save a valid edit
- **WHEN** the author changes a color in `scene.wgsl` and saves
- **THEN** the change is visible within 500 ms and music is unaffected

#### Scenario: Save a broken shader
- **WHEN** the author saves WGSL with a syntax error
- **THEN** the previous shader keeps rendering and an error toast appears

### Requirement: Tolerant variants
Variants SHALL keep loading when their scene's parameters change: unknown keys are ignored and missing keys take defaults.

#### Scenario: Param removed
- **WHEN** a parameter is removed from a scene manifest
- **THEN** existing variants of that scene still load
