# Spec Delta

## ADDED Requirements

### Requirement: Output levels
The audio callback SHALL measure the peak of each channel of the audio it plays, post-EQ and
pre-volume as the tap is, and SHALL publish the loudest peak since the app last read it, without
locks, allocation or waiting. Reading the levels SHALL clear them.

#### Scenario: Loudest peak kept
- **WHEN** three buffers play with peaks of 0.2, 0.9 and 0.4 before the app reads the levels
- **THEN** the app reads 0.9, and reading again before more audio plays gives 0

#### Scenario: Real-time safe
- **WHEN** levels are measured and read in the allocation-counting test
- **THEN** the audio callback allocates nothing, and the start, seek and underrun targets still hold
