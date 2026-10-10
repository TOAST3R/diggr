## MODIFIED Requirements

### Requirement: LCD colour from the skin
Every LCD text the app draws (the title line, kbps and kHz, the filter bar's search text, BPM range, filters and cart switch, and the playlist footer's info) SHALL use the skin's LCD colour. A skin file that names no LCD colour SHALL get the earlier green (0, 236, 0).

#### Scenario: Amber default
- **WHEN** a track plays with the default skin
- **THEN** its title line, kbps and kHz are drawn in the default skin's amber

#### Scenario: Older skin file
- **WHEN** a skin file without an LCD colour is loaded
- **THEN** its LCD text is drawn in green (0, 236, 0)

#### Scenario: Filter bar text
- **WHEN** the user types "parrish" in the search field with the default skin
- **THEN** "PARRISH" is drawn in the default skin's amber
