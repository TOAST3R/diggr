## MODIFIED Requirements

### Requirement: Friends list from Discogs
With a Discogs token set, the app SHALL read the user's friends from Discogs (`/users/‹username›/friends`, every page) and keep a crate per friend named "Friend: ‹username›". It SHALL read the list the first time a token is set (once the window is interactive), and then at most once every 7 days, at a moment with no send waiting. The list, when it was last read, and which friends' collections are private SHALL be saved in the config folder and survive restarts; whether FRIENDS is open SHALL be saved with the sidebar's settings (see `crates` "Crate sidebar"), not with the list. There SHALL be no way to add or remove a friend in the app. A friend's crate SHALL still be recognised as that friend's after it is renamed. Without a token nothing SHALL be read and FRIENDS SHALL NOT be shown.

#### Scenario: First read
- **WHEN** a token for a user with 12 friends is saved
- **THEN** FRIENDS lists 12 crates named "Friend: ‹username›", none of them dug, after one request

#### Scenario: Within a week
- **WHEN** the app is launched and the list was read 3 days ago
- **THEN** no friends request is sent, and FRIENDS shows the saved list

#### Scenario: A week later
- **WHEN** the app is launched and the list was read 8 days ago, and the user has made a new friend since
- **THEN** the list is read again and the new friend's crate appears

#### Scenario: Renamed
- **WHEN** the user renames "Friend: javier.camunas" to "Javier"
- **THEN** it stays under FRIENDS, and double-clicking it digs javier.camunas's collection

#### Scenario: An old friends file
- **WHEN** the app starts with a friends file saved by an earlier version, holding `folded: true` and 12 friends
- **THEN** the file loads with its 12 friends, and FRIENDS starts closed
