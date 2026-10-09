## ADDED Requirements

### Requirement: YouTube limiting
When yt-dlp's error for a download or a search says that YouTube is limiting requests (HTTP 429, "Too Many Requests", or a request to sign in to confirm the user isn't a bot), the app SHALL treat it as limited, not as a failure of that track: the track SHALL NOT be marked "clip failed" or "not found", and its tries SHALL NOT be counted. While limited, no preview download or search SHALL start. The app SHALL try again after 10 minutes, then after 20, 40 and at most 60 minutes while YouTube keeps limiting, and the wait SHALL start again at 10 minutes once a download or search succeeds. The main window SHALL say once per limited period "YouTube is limiting requests: trying again in ‹N› min". Entries waiting for their preview meanwhile SHALL keep waiting, and their tooltip SHALL say "waiting for YouTube". Being limited SHALL never touch playback of previews already downloaded.

#### Scenario: Too many requests
- **WHEN** yt-dlp fails a download with "HTTP Error 429: Too Many Requests"
- **THEN** the track stays waiting (not "clip failed"), no other download or search starts, and the main window says YouTube is limiting requests and in how many minutes it tries again

#### Scenario: Bot check on a search
- **WHEN** a search fails with "Sign in to confirm you're not a bot"
- **THEN** the track stays waiting to be searched, it isn't remembered as not found, and searching pauses like downloading

#### Scenario: Longer waits
- **WHEN** YouTube is still limiting when the app tries again after 10 minutes
- **THEN** it waits 20 minutes, then 40, then 60 minutes between tries

#### Scenario: Back to normal
- **WHEN** a try after the wait succeeds
- **THEN** downloads and searches carry on, and the next limited period waits 10 minutes again

#### Scenario: A broken video is still a failure
- **WHEN** yt-dlp fails a download with "Video unavailable"
- **THEN** it counts as a failed try as before, and after two the track is "clip failed"
