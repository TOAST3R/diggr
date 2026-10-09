## ADDED Requirements

### Requirement: Download paused by YouTube
While YouTube is limiting requests (see `preview-fetch`), Download all tracks' window SHALL show, instead of the tracks downloading, "Paused: YouTube is limiting requests. Trying again in ‹N› min.", with a Try now button that tries at once. If that try is limited too, the wait SHALL go on as if it had been the scheduled try. The window SHALL come back if it was hidden when the pause starts.

#### Scenario: Paused and shown
- **WHEN** Download all tracks runs with its window hidden and YouTube starts limiting
- **THEN** the window shows again, says it is paused for YouTube and in how many minutes it tries again, and offers Try now and Stop

#### Scenario: Try now
- **WHEN** the user clicks Try now and YouTube answers normally
- **THEN** the pause line goes, and downloading carries on

### Requirement: Retry failed tracks
A label crate's tracks marked "clip failed" or "not found" SHALL be retryable. Download all tracks' window SHALL offer Retry failed (N) while N of the crate's tracks are retryable, and the label crate's menu SHALL offer Retry failed tracks (N) when it isn't downloading and N is above 0. Retrying SHALL put those tracks back to waiting: "clip failed" ones to be downloaded again, "not found" ones to be searched again, with their remembered "not found" forgotten. It SHALL then run them as part of Download all tracks, starting it if it isn't running. "No clip" and "already in crate" tracks SHALL NOT be retried. When everything is done, the summary SHALL count the tracks still without a preview.

#### Scenario: Retry from the menu
- **WHEN** "Label: Analogical Force" has 30 tracks "not found" and 4 "clip failed", and the user chooses Retry failed tracks (34)
- **THEN** the 30 are searched again, the 4 are downloaded again, Download all tracks' window opens with them, and the tracks that now get a preview play like any other

#### Scenario: Not everything is retryable
- **WHEN** the same crate also has 5 tracks "already in crate" and 2 with "no clip"
- **THEN** those 7 stay as they are, and the count reads 34

#### Scenario: Retry from the window
- **WHEN** Download all tracks' window shows 39 without a preview, 34 of them retryable
- **THEN** it offers Retry failed (34), and choosing it puts them back into the running download
