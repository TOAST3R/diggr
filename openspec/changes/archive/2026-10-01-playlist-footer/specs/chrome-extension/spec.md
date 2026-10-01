## MODIFIED Requirements

### Requirement: Feedback
After a send from a Discogs page, the page SHALL show, for 3 s, a confirmation naming the page and the crate. After a send from a link on another site, the extension's toolbar button SHALL show a mark for 3 s: success or failure. Clicking the toolbar button SHALL show whether the app is running and paired, what is playing, and sends in progress. When the app isn't running, the extension isn't paired, or the page isn't supported, the extension SHALL say so and how to fix it.

#### Scenario: Confirmation
- **WHEN** the user enqueues a label page
- **THEN** the page shows "Sent to ‹App›: Label: Lowtide Tapes → ‹crate›" for 3 s

#### Scenario: App not running
- **WHEN** the user chooses Enqueue in ‹App› while the app is closed
- **THEN** a message says that the app isn't running, and nothing else happens

#### Scenario: Not paired
- **WHEN** the user chooses an action before the extension is paired
- **THEN** the pairing screen opens and asks for the code shown in Options ▸ Browser…

### Requirement: Owned note
On release, master release and marketplace item pages, when the app says the page's record is owned, the extension SHALL show under its button "✓ In your collection", or "✓ Another pressing in your collection (‹catalog number›, ‹year›)". When the answer is "checking", it SHALL ask once more after 3 s. When the app has no Discogs token, it SHALL show, dimmed, that a token in the app (Options ▸ Discogs…) is needed to see owned records. When the record isn't owned, or the answer is unknown, it SHALL show nothing. It SHALL send the app only the page's address for this.

#### Scenario: Owned on a shop page
- **WHEN** the user opens a marketplace item of a record they own
- **THEN** within a few seconds the button shows "✓ In your collection"

#### Scenario: No token in the app
- **WHEN** the user opens a release page while the app has no Discogs token
- **THEN** a dimmed line under the button says to add a Discogs token in the app to see owned records

#### Scenario: Not owned
- **WHEN** the user opens a release they don't own
- **THEN** no note is shown
