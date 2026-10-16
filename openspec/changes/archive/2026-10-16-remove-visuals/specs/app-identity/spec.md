## ADDED Requirements

### Requirement: No leftover visuals folder
The app SHALL delete the folder `visuals` inside its config folder (`<config>/Diggr/visuals/`, or inside `DIGGR_CONFIG_DIR` when set), which earlier versions used for the user's scenes and variants, when it exists at launch. It SHALL do so after the first frame, off the UI thread, and SHALL touch no other path. A failure to delete it SHALL be ignored.

#### Scenario: Upgrade from a version with visuals
- **WHEN** the app starts with a config folder holding `visuals/scenes/` and `settings.ron`
- **THEN** `visuals/` is gone after launch, and `settings.ron` and every other file are untouched

#### Scenario: Nothing to delete
- **WHEN** the config folder has no `visuals` folder
- **THEN** launch is unchanged, and nothing is created or deleted
