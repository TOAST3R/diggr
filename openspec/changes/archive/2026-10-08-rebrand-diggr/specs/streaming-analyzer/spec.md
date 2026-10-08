## MODIFIED Requirements

### Requirement: Cache
Scores (including partial coverage) SHALL be cached on disk by content hash and algorithm version in the platform cache directory (`DIGGR_CACHE_DIR` overrides), and a cached score SHALL be available immediately on play.

#### Scenario: Second play
- **WHEN** a previously fully analyzed track is played again
- **THEN** its full score is available at play start without re-analysis

#### Scenario: Algorithm upgrade
- **WHEN** the analysis algorithm version changes
- **THEN** old cache entries are ignored and regenerated
