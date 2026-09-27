## ADDED Requirements

### Requirement: Two decks while mixing
From the moment a mix is planned until it ends, the waveform's zoomed row SHALL show two lanes, the outgoing track above the incoming track, each centered on its own audible position and drawn at the set tempo so that both beat grids line up; it SHALL mark the mix region and the handover, dim a lane's bass while its low band is cut, and follow each track's volume.

#### Scenario: Aligned grids
- **WHEN** a 126 BPM track is mixed into a 124 BPM set
- **THEN** both lanes show their downbeats at the same horizontal positions throughout the mix

#### Scenario: EQ visible
- **WHEN** the incoming track's low band is cut before the handover
- **THEN** its lane shows its bass (red) columns dimmed, and after the handover the outgoing lane's are dimmed instead
