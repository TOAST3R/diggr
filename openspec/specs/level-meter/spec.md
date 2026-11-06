# level-meter Specification

## Purpose

Shows how loud the music playing is, channel by channel, with the movement of a hardware LED
meter, so the player visibly moves with the music and a preview's level and dynamics read at a
glance.

## Requirements

### Requirement: Stereo level meter
The mini player SHALL show a level meter beside the time: two LED bars, left and right, spanning
−48 to 0 dBFS in 2 dB segments, lit green up to −12 dB, amber from −12 dB and red from −3 dB.
They SHALL show the audio as heard before the volume control.

#### Scenario: A full-scale hit
- **WHEN** a track plays a peak at −1 dBFS on the left channel
- **THEN** the left bar lights up into the red, and the right bar shows the right channel's own level

#### Scenario: Volume at zero
- **WHEN** the volume is at 0% and a track plays
- **THEN** the meter moves with the music as it would at full volume

### Requirement: Meter ballistics
Each bar SHALL rise to a new peak on the first frame drawn after it is heard, and SHALL fall at
20 dB per 1.5 s when the level drops. A peak dot SHALL mark each bar's highest level, hang for
1 s and then fall at the same rate. Every peak in the audio SHALL reach the meter, even one
shorter than a frame.

#### Scenario: Attack
- **WHEN** a click at −6 dBFS follows silence
- **THEN** the bar reaches −6 dB on the first frame drawn after the click is heard

#### Scenario: Release
- **WHEN** the music drops from −6 dBFS to silence
- **THEN** the bar falls to −26 dB 1.5 s later (±1 frame), and the peak dot stays at −6 dB for 1 s before falling

#### Scenario: A very short transient
- **WHEN** a single loud sample plays between two frames
- **THEN** the next frame shows its level

### Requirement: Meter at rest
When playback pauses or stops, the bars and peak dots SHALL fall to empty at their release rate,
and then the meter SHALL ask for no repaint. Reduce motion SHALL draw the bars at their level
without the fall and without peak dots.

#### Scenario: Paused
- **WHEN** a loud track is paused
- **THEN** the bars fall to empty within 3.6 s, the peak dots (after their second of hold) within 4.6 s, and the app then requests no repaint for the meter
