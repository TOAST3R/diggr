## 1. Parsing (`dig::tracklist`)

- [x] 1.1 The tracklist model and the line parser (D2), with a table test of ~30 real line shapes (times, numbers, dashes, labels, versions, IDs, non-track lines) and the fewer-than-3 rule
- [x] 1.2 Chapters first, description otherwise (D1); verify with saved yt-dlp JSON fixtures of a chaptered YouTube mix, a description-only one and a SoundCloud mix

## 2. Reading mixes

- [x] 2.1 The yt-dlp metadata read for a YouTube id or a rebuilt SoundCloud address (D5), `--dump-single-json --skip-download --no-playlist`, off the UI thread; verify the arguments with the fake fetcher and that nothing is written
- [x] 2.2 The tracklist send: bridge checks (256 KB, 300 tracks, fields, refused alone), answered within 100 ms; verify with bridge tests
- [x] 2.3 Pasting a YouTube or SoundCloud address reads it as a mix; pasting 1001tracklists or Mixcloud says to use the extension; verify with headless tests

## 3. The Mix crate

- [x] 3.1 "Mix: ‹title›" in mix order, entries waiting to be searched, IDs unavailable "unidentified in the mix", sending again adds only new tracks, a Play send starts the first track; Discogs linking on demand (Find on Discogs: one fitting record links it, several or none open Discogs' search); verify with headless tests
- [x] 3.2 Open in mix at m:ss / Open mix with the rebuilt addresses (D4); verify with a headless test against the fake browser

## 4. The extension

- [x] 4.1 Moved to a later change: 1001tracklists refuses automated browsers and the Mixcloud show checked had no tracklist, so the readers need pages saved from a real browser. The app's side (the tracklist send) is in this change

## 5. Docs and checks

- [x] 5.1 README: mixes as a source, the four sites, Open in mix, test areas and count
- [x] 5.2 Run `cargo test --workspace`, clippy with `-D warnings`, `cargo fmt --all --check` and the wasm check; dig one real mix from YouTube and from SoundCloud

## 6. The extension on YouTube and SoundCloud

- [x] 6.1 Mix pages recognised (YouTube `/watch?v=`, SoundCloud ‹user›/‹name›, not sets or SoundCloud's own pages), access to both sites, the button sending the address, "Mix: ‹title›" suggested; checked on the real pages with the scripts injected (button by the title, Play sends the address)
- [x] 6.2 The bridge takes a mix's address (`SendMix`) and the player reads it with the send's mode; bridge and headless tests

