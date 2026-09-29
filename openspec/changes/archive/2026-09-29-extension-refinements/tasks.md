## 1. App: listings

- [x] 1.1 `url.rs`: `PageKind::Listing(u64)` for `/shop/item/{id}` and `/sell/item/{id}` (language prefix, `www.`, query and fragment); unit tests, including a forum address still refused
- [x] 1.2 Resolve a listing to its release (`GET /marketplace/listings/{id}` → `release.id`), cached; a missing listing is "not found"; tests with the fake transport and a fixture
- [x] 1.3 A sent listing expands exactly like its release (crate name, entries, filters); intake test

## 2. Extension

- [x] 2.1 `pages.js`: listing pages in `pageKind` (kind "Listing") and in the link patterns
- [x] 2.2 `content.js`: a pure `suggestName(title, kind, address)` (asterisks, `(n)` suffixes, long dash, 40-character cut, wantlist user) and the prompt's default; check it in the browser console against real titles

## 3. Docs and checks

- [x] 3.1 README (listing pages, the suggested crate name) and the test count
- [x] 3.2 Load the unpacked extension and try a release, a label and a listing page; run fmt, clippy, the workspace tests and the wasm check
