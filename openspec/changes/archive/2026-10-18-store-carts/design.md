## Context

- **The Discogs cart (`discogs-cart`)** is the model: a pill on the row (+ CART, IN CART, REMOVE on hover), the change shown within a frame, outcomes (added, already in, sold, anything else opens the listing), a CART badge from a cached snapshot, and the rule that the app never empties a cart. It works because the app holds a Discogs token.
- **The stores give the app nothing like a token.** A Juno, Beatport, Traxsource or Bleep basket belongs to the user's browser session (cookies, and on Beatport a bearer token the site gets from that session). Only the browser can act on it.
- **The bridge is one-way today:** the extension calls the app (`/v1/send`, `/v1/crates`, `/v1/status`, `/v1/owned`); the app can't reach the extension. Chrome suspends an idle extension service worker after about 30 s.
- **Store entries already know their item:** `StoreTrack.track` and `page` (Beatport, Traxsource), the Juno product id (format included), and with `bleep` the Bleep release and its formats (their product ids are added by D1's results).

## Goals / Non-Goals

**Goals:**
- One click on a store entry's row puts it in that store's basket, the same way as Discogs.
- The click never does nothing: if the basket can't be reached, the item's page opens on the store.
- Badges show what is in the store baskets, as far as the extension could read them.

**Non-Goals:**
- Checking out, paying, coupons, or emptying a basket.
- Buying entries that didn't come from the store (a mix track, a Discogs record): finding them on a store first is a separate change.
- A store cart crate or subtotal view (row pills and badges are enough).
- Logging in for the user.

## Decisions

### D1. Look at each store's basket first
- **How:** task 1 records, per store, the basket's add, remove and read calls the site itself makes (method, address, body, the session it needs), and whether the extension's service worker can make them with the user's cookies (SameSite rules may block it) or must make them from a script in a store tab. The outcome per store goes in this design before code.
- **Why:** like the Discogs cart calls, these aren't published APIs; each store is its own small job, and one may turn out impossible. That store then only gets the fallback (D4), and the spec says which.

#### D1 results (recorded 2026-10-18, fixtures in `crates/dig/tests/fixtures/store_carts/`)

| Store | Add | Remove | Read | Session |
|---|---|---|---|---|
| **Bleep** | `POST https://bleep.com/rest/cart`, JSON `{"item":{"id":‹format id›,"type":"ReleaseFormat","qty":1,"bundled":[]}}`, header `X-Requested-With: XMLHttpRequest` | `GET https://bleep.com/cart/remove/‹line id›` (redirects to `/cart`) | `GET https://bleep.com/rest/cart` with the same header | the logged-in cookies |
| **Juno** | `GET https://www.juno.co.uk/cart/add/‹title›/‹n›/?json=1` for product `‹title›-0‹n›` | `GET https://www.juno.co.uk/cart/remove/?item[‹title›-‹n›]=1&json=1` | `GET https://www.juno.co.uk/cart/`, the checkout form's `item[‹title›-‹n›]` fields | cookies; **no account needed** (guest basket) |
| **Beatport** | — | — | — | **fallback-only**: every basket call needs an account; none was available to record them |
| **Traxsource** | — | — | — | **fallback-only**: Add to Cart asks to log in before any request; no account to record them |

- **Bleep answers.** An add returns 201 with the whole basket (`bleep_added.json`): `items[]` hold `cartableType`, `cartableId` (the format id) and `id` (the line id, needed to remove); prices are in `cartDetailPrices["com.bleep.shop.cart.CartDetail : ‹line id›"].priceWithDiscount`, currency in `currency`. Already in: 200 `{"error":true,"message":"cart.item.already.added"}`. An unknown id: 404, empty. **Without `X-Requested-With` the add answers 500 with an HTML error page yet the item is added**, so a non-JSON answer is "failed" and the extension reads the basket before reporting anything. A request without the browser's cookies gets an empty 202 (the site's bot check): "failed". Sold out wasn't seen (no sold-out format found): any `error` other than already-added is "failed", so the page opens and shows it.
- **Bleep's format ids aren't in today's intake.** Each format's buy button carries `data-itemid` (the `ReleaseFormat` id) and `data-value` (price); `bleep-intake`'s formats gain an `id` read from it. A record from a list tile has no formats, so its pill falls back to opening the release page.
- **Juno answers.** An add returns JSON (`juno_added.json`) whose `quickCart.items[]` is the basket after the add (`titleId`, `productId`, `unitPrice` as HTML text like `&#128;13,56`). Already in: the quantity stays 1 and `cart.warnings` reads "Already in cart" (`juno_already_in.json`); the extension checks the basket first and reports "already in" without adding. Anything in `cart.errors` is "failed"; `/cart/?json=1` also carries `oos_*` fields, so an out-of-stock add reports "sold out" when they are set. Juno never says "not logged in": the basket belongs to the browser.
- **From the service worker.** The calls are plain same-origin requests with the site's cookies, and the header is one a service worker can set. Chrome treats an extension's requests to sites it has host access to as same-site, so cookies are sent. If a store refuses them from the worker anyway (its bot check, an Origin test), the worker hands the call to the extension's content script in an open tab of that store (it already runs there; no new permission), and with no such tab the action falls back (D4). Task 4.1 checks this live.

### D2. The app asks, the extension acts
```
 click + CART          GET /v1/cart/next (held open)      store basket call
 ┌─────────┐ queue ──▶ ┌──────────────┐ ───────────────▶ ┌─────────────┐
 │  Diggr  │           │  extension   │                  │ store (user │
 │ PENDING │ ◀──────── │              │ ◀─────────────── │  session)   │
 │ IN CART │ POST /v1/cart/result      └──────────────┘                  └─────────────┘
 └─────────┘   (+ the basket it read)
```
- **Pending actions** are queued in the app, each with an id, store, kind (add, remove) and item ids rebuilt from the entry, never free addresses.
- **The extension waits** on `GET /v1/cart/next`, held open up to 25 s while the app has nothing, so the action is picked up at once and the service worker stays awake while the app runs. It is keyed like every bridge request (pairing).
- **The extension reports** each result (added, already in, removed, sold out, not logged in, failed) and, when it read it, the store's basket.
- **Why not click the store's buttons in a page:** the store's own calls are steadier than its markup, and need no tab in front.

### D3. The pill and what it adds
- **Pill:** the Discogs pill's look, on store entry rows. It changes to IN CART within the frame of the click (pending), as Discogs' does; a failed outcome puts it back.
- **Juno:** the product (a product is one format). **Beatport, Traxsource:** the track. **Bleep:** a record sold in several formats opens a small menu of its formats with prices; one format adds directly.
- **Which entries:** only entries with a store item; others show no pill. An entry with both a Discogs copy pill (seller crates) and a store item shows the Discogs pill there, the store pill elsewhere.

### D4. Outcomes and the fallback
- **Added / already in:** IN CART and the badge. **Sold out:** SOLD for that item, and the main window says so.
- **Anything else** (no extension waiting within 3 s, not logged in, refused, unexpected answer, no result within 10 s): the item's page opens on the store in the default browser, and the main window says the basket couldn't be reached and the page was opened instead. A store D1 found impossible (Beatport, Traxsource) always takes this path at once, with nothing queued.

### D5. Badges from the baskets read
- The extension reads a store's basket after each action, and whenever it can (at the app's launch, on request), best effort. The app keeps the last read per store in `store-carts.ron` and shows CART on every entry of an item in it, with "In your Beatport cart, $1.69". A basket never read shows nothing; items added from the app show CART until a read says otherwise.

### D6. The extension's access widens, narrowly
- Today the extension makes no request but to the app. Now it may call each store's basket endpoints found in D1, and only those: to perform an action the app queued, or to read the basket. It still reads nothing from other pages and contacts no other server. Host access stays what `bleep` gives (juno.co.uk, beatport.com, traxsource.com, bleep.com).

## Risks / Trade-offs

- **[A store's basket calls change or are blocked]** → That store falls back to opening the page (D4); nothing else breaks. Fixtures of the calls make the break obvious.
- **[The app could put things in the user's baskets]** → Only from a click on a pill, only items rebuilt from entries the store sent, only through the paired extension; never checkout, never emptying. A basket costs nothing until paid.
- **[Service worker sleeping]** → The held-open request keeps it awake while the app runs; if it sleeps anyway, the 3 s wait fails over to opening the page.
- **[Session-bound calls need a store tab]** → If D1 finds a store's calls work only from a page, the extension uses an open tab of that store, or the action falls back.
