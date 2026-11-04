## MODIFIED Requirements

### Requirement: Asked at first launch
On Windows x86_64, while yt-dlp can't be found, the welcome at launch (see `welcome-setup`) SHALL include a Previews step, listed first. It SHALL say that previews from Discogs, Bandcamp and the other shops need yt-dlp, the download size, where it comes from, that the user's own files play without it, and that it also installs a Python and a JavaScript runtime used only by Diggr. It SHALL offer Install, which installs and shows the progress in the step as the guided question does, and Later. It SHALL follow the welcome's rules for when it opens, so no separate modal SHALL open at launch for yt-dlp. Showing it SHALL NOT delay launch: the lookup SHALL run off the UI thread, and the step SHALL appear once it has found no yt-dlp.

#### Scenario: Fresh Windows PC
- **WHEN** Diggr is opened for the first time on a Windows PC without yt-dlp
- **THEN** the player opens, and within 2 s the welcome lists the Previews step first, with Install and Later

#### Scenario: yt-dlp already there
- **WHEN** Diggr is opened on a Windows PC with `yt-dlp.exe` on the PATH
- **THEN** the welcome has no Previews step

#### Scenario: Turned off
- **WHEN** the user ticks "Don't show at startup" and reopens the app without yt-dlp
- **THEN** no modal opens at launch; the question still opens when the user plays a preview
