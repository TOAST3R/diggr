## ADDED Requirements

### Requirement: Unreachable source
When yt-dlp's error for a download, a search or a Bandcamp read, or a Juno sample request, says that the source can't be reached (no connection, connection refused or reset, a name that doesn't resolve, or a certificate that fails to verify), the app SHALL treat it as unreachable, not as a failure of that track. The track SHALL NOT be marked "clip failed" or "not found", its tries SHALL NOT be counted, and a search SHALL NOT be remembered as failed. While a source is unreachable, no download, search or read from it SHALL start, and the other sources SHALL go on. The app SHALL try again after 1 minute, then after 2, 4, 8 and at most 10 minutes while the source stays unreachable, and once a request to it goes through, the wait SHALL start again at 1 minute. The main window SHALL say once per unreachable period "Can't reach ‹source›: trying again in ‹N› min", and when the cause is a certificate that fails to verify, it SHALL add that a proxy, VPN or security tool may be intercepting secure connections. Entries waiting meanwhile SHALL keep waiting, with the tooltip "waiting for ‹source›". The pause SHALL stay visible after that message: an armed entry waiting for the source SHALL say in the status line why it waits and when the next try is ("Waiting for ‹entry›. Can't reach YouTube: trying again in 1 min…"), and with nothing else to show, the status line SHALL keep saying it. An HTTP error status, "Video unavailable" and a download stopped after 120 s SHALL still count as failed tries. Being unreachable SHALL never touch playback of previews already downloaded.

#### Scenario: Intercepted connection
- **WHEN** every yt-dlp run fails with "[SSL: CERTIFICATE_VERIFY_FAILED] certificate verify failed"
- **THEN** no entry becomes "clip failed" or "not found", the entries keep waiting with "waiting for YouTube", and the main window says once that YouTube can't be reached, when it tries again, and that a proxy, VPN or security tool may be intercepting secure connections

#### Scenario: A double-clicked entry says why it waits
- **WHEN** the user double-clicks a track waiting to be searched while YouTube can't be reached
- **THEN** the status line says "Waiting for ‹track›. Can't reach YouTube: trying again in ‹N› min", with the interception hint after a certificate error, until it plays

#### Scenario: Shorter waits than limiting
- **WHEN** YouTube is still unreachable at each try
- **THEN** the tries come after 1, 2, 4 and 8 minutes, and then every 10 minutes

#### Scenario: Network comes back
- **WHEN** a try after the wait succeeds
- **THEN** downloads and searches carry on, the waiting entries download, and the next unreachable period waits 1 minute again

#### Scenario: Only one source unreachable
- **WHEN** YouTube can't be reached while Juno samples download normally
- **THEN** JN entries keep downloading and YT entries keep waiting

#### Scenario: A missing video is still a failure
- **WHEN** yt-dlp fails a download with "HTTP Error 404" or "Video unavailable"
- **THEN** it counts as a failed try, and after two the track is "clip failed"
