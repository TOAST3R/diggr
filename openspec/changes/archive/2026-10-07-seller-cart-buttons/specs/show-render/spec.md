## REMOVED Requirements

### Requirement: Render from the player
**Reason**: The player is for listening and digging. Rendering a show to video is rarely done from the playlist, and the command line already offers every option the dialog had.
**Migration**: Run `diggr --render-show TRACK -o OUT.mp4` with `--size`, `--fps`, `--from`, `--to`, `--overlay` and `--look` as needed (see README, "Render a show to video").
