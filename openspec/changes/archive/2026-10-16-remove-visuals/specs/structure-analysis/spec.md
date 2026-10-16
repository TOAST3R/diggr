## REMOVED Requirements

### Requirement: Tension and drop countdown
**Reason**: Only the visuals and the fullscreen debug strip read `tension` and `drop_in`; both are removed (`remove-visuals`).
**Migration**: None. Drop jumps (`structure-navigation`) use section energy, not tension.
