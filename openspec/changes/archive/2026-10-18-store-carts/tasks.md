## 1. Look at each store's basket (D1)

- [x] 1.1 For Juno, Beatport, Traxsource and Bleep: record in design.md the add, remove and read requests the site makes (method, address, body, session), and whether the extension's service worker can make them with the user's cookies or must use a store tab; save sample answers (added, already in, sold out, logged out) as fixtures. A store that can't be done is marked fallback-only in design and spec
- [ ] 1.2 Check that a held-open bridge request keeps the service worker awake for 10 minutes of idling and picks up an action within 100 ms; if not, record the polling that does and adjust the 3 s wait

## 2. The app side (`dig`, bridge)

- [x] 2.1 The store cart model (items per store with prices), `store-carts.ron` (fails soft), and the pending-action queue with the 3 s pick-up and 10 s result timeouts; unit tests
- [x] 2.2 `GET /v1/cart/next` (held up to 25 s) and `POST /v1/cart/result` with every check in `browser-bridge` Cart requests; verify with bridge tests (picked up at once, empty after 25 s, forged and unpaired refused, bad basket refused)

## 3. In the player

- [x] 3.1 The pill on store entries (look and precedence as Discogs'), Bleep's format menu, the change within one frame, outcomes and the fallback opening the store page, SOLD, and the CART badge with its tooltip from cached and read baskets; verify with headless tests against a fake extension (added, already in, sold, not logged in, no extension, timeout)

## 4. The extension

- [x] 4.1 The wait loop, the per-store basket calls from 1.1, reading baskets after actions and at the app's start, outcomes as specified; no other requests
- [x] 4.1b Bleep's intake reads each format's product id (`data-itemid`), checked as `bleep-intake` says; tests
- [x] 4.2 Manual checklist in the README: add and remove on Juno and Bleep (Bleep logged in and out), a sold-out item, extension stopped, and Beatport and Traxsource opening the page

## 5. Docs and checks

- [x] 5.1 README: "Buying from the stores", the extension's basket requests, `store-carts.ron` in the cache table, test areas and count
- [x] 5.2 Run `cargo test --workspace`, clippy with `-D warnings`, `cargo fmt --all --check` and the wasm check
- [x] 5.3 Live: add and remove a Juno product and a Bleep format and see each in the store's basket; + CART on Beatport and Traxsource opens the page
