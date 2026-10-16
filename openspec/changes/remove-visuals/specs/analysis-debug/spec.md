## REMOVED Requirements

### Requirement: Debug timeline strip
**Reason**: The strip was drawn only in fullscreen, which is removed (`remove-visuals`).
**Migration**: Analysis quality is measured with `analysis-eval`; beats and sections remain visible in the waveform.

### Requirement: Annotation capture
**Reason**: Annotation mode lived in fullscreen, which is removed (`remove-visuals`).
**Migration**: `analysis-eval` still reads annotation files already captured, or written by hand in the same JSON format.
