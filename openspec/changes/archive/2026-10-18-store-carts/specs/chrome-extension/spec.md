## MODIFIED Requirements

### Requirement: Minimal access
The extension SHALL ask for access only to discogs.com, the subdomains of bandcamp.com, juno.co.uk, beatport.com, traxsource.com and bleep.com, and 127.0.0.1, and for its own storage, context menu and toolbar button. It SHALL send the app only the page's address, the chosen mode, the crate and the switches; for a Bandcamp page, its title; for a Juno page, the records the page shows as `juno-intake` describes them; for a Beatport or Traxsource page, the tracks the page shows as `beatport-intake` and `traxsource-intake` describe them; for a Bleep page, the records the page shows as `bleep-intake` describes them; and the results of store cart actions with the baskets read (see `browser-bridge`, Cart requests). It SHALL contact no other server. To juno.co.uk and bleep.com it SHALL make only the basket requests listed in `store-carts`' design (D1 results), and to beatport.com and traxsource.com none, and only to perform a cart action the app gave it or to read that store's basket; it SHALL make no request to any other site. It SHALL read nothing from pages beyond their address and title, except on Juno, Beatport, Traxsource and Bleep pages, where it SHALL read only the fields their specs list from the page already loaded (on Beatport, from the page's own data, `__NEXT_DATA__`), and only when the user chooses an action. It SHALL contain no remote code.

#### Scenario: Permissions at install
- **WHEN** the extension is loaded in Chrome
- **THEN** the only site access it lists is discogs.com, *.bandcamp.com, *.juno.co.uk, *.beatport.com, *.traxsource.com, *.bleep.com and 127.0.0.1

#### Scenario: Nothing read until asked
- **WHEN** the user opens a Juno product page and doesn't open the button's menu
- **THEN** the extension has read nothing from the page beyond its address, and has sent nothing to the app

#### Scenario: No fetching on Juno
- **WHEN** the user sends a Juno label page showing 20 of its 200 records
- **THEN** the extension sends those 20 records and makes no network request other than the one to the app

#### Scenario: Only basket calls
- **WHEN** the app gives the extension an add for a Bleep format
- **THEN** the extension makes only Bleep's basket requests for it, and reports the result to the app

## ADDED Requirements

### Requirement: Store cart actions
While the app runs, the extension SHALL wait for its pending store cart actions (see `browser-bridge`, Cart requests), perform each one in the user's browser session with that store's own basket requests, and report the outcome. After each action, and when the app starts, it SHALL read the store's basket when it can and report it. It SHALL report "not logged in" when the store says the session has no account, and "failed" for anything it doesn't recognise, never guessing an outcome.

#### Scenario: Unexpected answer
- **WHEN** the extension performs a Bleep add and Bleep answers with an HTML error page
- **THEN** it reads the basket, reports "added" if the format is in it and "failed" otherwise, never guessing
