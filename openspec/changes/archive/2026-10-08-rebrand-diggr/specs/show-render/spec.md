## MODIFIED Requirements

### Requirement: Render a track's show to video
`diggr --render-show <track> -o <file.mp4>` SHALL render the track's visual show and the track's audio into an H.264/AAC MP4, with options for size (default 1920×1080; the width divisible by 8 and the height even), frame rate (default 60), a time range, the overlay card, and a pinned look.

#### Scenario: Default render
- **WHEN** the user runs `diggr --render-show track.flac -o show.mp4`
- **THEN** show.mp4 is 1920×1080 at 60 fps, as long as the track, with the track's audio

#### Scenario: Range and size
- **WHEN** the user adds `--from 1:00 --to 1:30 --size 1280x720 --fps 30`
- **THEN** the file is a 30-second 1280×720 30 fps video of that part of the show and audio
