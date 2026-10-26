//! Store baskets, headless: the cart pill on Juno, Bleep and Beatport entries, against a fake
//! extension that takes the queued actions and answers them as `/v1/cart/result` would
//! (the HTTP side is in `dig`'s bridge tests): added, already in, removed, sold out, not
//! logged in, no extension and timeout; Bleep's format menu; the basket at launch.

use super::*;
use ::dig::bridge::{BridgeCommand, Mode};
use ::dig::browser::FakeBrowser;
use ::dig::carts::{BasketItem, CartAction, CartKind, CartResult, Outcome, ResultBody};
use ::dig::clock::RealClock;
use ::dig::cover::{FakeImages, test_jpeg};
use ::dig::discogs::transport::FakeTransport;
use ::dig::jobs::Filters;
use ::dig::preview::fetcher::{FakeFetcher, Fetcher};
use ::dig::preview::scheduler::Finder;
use ::dig::stores::{Stock, Store, StoreRecord, StoreTrack};

const PRODUCT: &str = "1170462-01";
const JUNO_PAGE: &str = "https://www.juno.co.uk/products/acid-pauli-gwar-vinyl/1170462-01/";

#[derive(Clone)]
struct Fakes {
    browser: Arc<FakeBrowser>,
    fetcher: Arc<FakeFetcher>,
}

fn rig(name: &str) -> (Rig, Fakes) {
    rig_with(name, |_| {})
}

/// `cache` prepares the cache folder before the app starts.
fn rig_with(name: &str, cache: impl FnOnce(&Path) + 'static) -> (Rig, Fakes) {
    let fakes = Fakes {
        browser: Arc::new(FakeBrowser::default()),
        fetcher: Arc::new(FakeFetcher::new(fixture("tone.m4a"))),
    };
    let f = fakes.clone();
    let rig = Rig::with_dig(
        name,
        Vec::new(),
        |_| {},
        move |dir| {
            cache(&dir.join("cache"));
            let fetcher = f.fetcher.clone();
            let finder: Finder = Arc::new(move |_| {
                let v = fetcher.version()?;
                Some((fetcher.clone() as Arc<dyn Fetcher>, v))
            });
            Some(DigSetup {
                transport: Arc::new(FakeTransport::default()),
                images: FakeImages::new(Ok(test_jpeg(300, 300))),
                clock: Arc::new(RealClock::default()),
                finder,
                samples: f.fetcher.clone(),
                browser: f.browser.clone(),
                cache_root: Some(dir.join("cache")),
                bridge: BridgeSetup::Port(0),
                tools: None,
            })
        },
    );
    (rig, fakes)
}

fn record(store: Store, id: &str, page: &str, price: &str, buy: Vec<(&str, &str)>) -> StoreRecord {
    StoreRecord {
        store,
        id: id.into(),
        page: page.into(),
        slug: "acid-pauli-gwar-vinyl".into(),
        artist: "Acid Pauli".into(),
        title: "Gwar".into(),
        label: String::new(),
        catno: String::new(),
        format: String::new(),
        stock: Stock::Unknown,
        price: price.into(),
        cover: String::new(),
        tracks: vec![StoreTrack {
            position: "1".into(),
            artist: "Acid Pauli".into(),
            title: "Gwar".into(),
            track: "635821".into(),
            page: page.into(),
            ..StoreTrack::default()
        }],
        buy: buy
            .into_iter()
            .map(|(a, b)| (a.to_owned(), b.to_owned()))
            .collect(),
    }
}

fn juno() -> StoreRecord {
    record(Store::Juno, PRODUCT, JUNO_PAGE, "€13,56", Vec::new())
}

fn send(rig: &mut Rig, r: StoreRecord) {
    rig.app.dig_bridge_command(BridgeCommand::SendStore {
        records: vec![r],
        mode: Mode::Enqueue,
        filters: Filters { skip_passed: true },
    });
}

fn first(rig: &Rig) -> crate::playlist::Entry {
    rig.app.crates.get(PLAYLIST).unwrap().entries()[0].clone()
}

fn pill(rig: &Rig) -> StorePill {
    rig.app
        .store_pill_of(&first(rig))
        .expect("a store entry has a pill")
}

fn click_pill(rig: &mut Rig, format: Option<&str>) {
    let ctx = rig.ctx.clone();
    let a = DigAction::StoreCart(first(rig).id, format.map(str::to_owned));
    rig.app.apply(Action::Dig(a), &ctx);
}

fn message(rig: &Rig) -> String {
    rig.app
        .message
        .as_ref()
        .map(|(m, _)| m.clone())
        .unwrap_or_default()
}

/// The fake extension: the next add or remove queued (reads at launch are answered empty).
fn next_action(rig: &Rig) -> Option<CartAction> {
    let shared = rig.app.dig.as_ref()?.bridge_shared.clone()?;
    loop {
        let a = shared.cart().take(Instant::now())?;
        if a.kind != CartKind::Read {
            return Some(a);
        }
        let body = ResultBody {
            id: a.id,
            outcome: Outcome::Failed,
            basket: None,
        };
        shared.cart().answer(body).expect("a read's answer");
    }
}

/// The fake extension answers `a` as `/v1/cart/result` would.
fn answer(rig: &mut Rig, a: &CartAction, outcome: Outcome, basket: Option<Vec<BasketItem>>) {
    let shared = rig.app.dig.as_ref().unwrap().bridge_shared.clone().unwrap();
    let body = ResultBody {
        id: a.id,
        outcome,
        basket,
    };
    let r = shared.cart().answer(body).expect("a valid answer");
    rig.app.dig_bridge_command(BridgeCommand::CartResult(r));
}

/// What the bridge says when no one answered `a` in time.
fn expire(rig: &mut Rig, a: CartAction, outcome: Outcome) {
    rig.app
        .dig_bridge_command(BridgeCommand::CartResult(CartResult {
            action: a,
            outcome,
            basket: None,
        }));
}

#[test]
fn a_juno_record_goes_in_the_basket_and_out_again() {
    let (mut rig, fakes) = rig("cart-juno");
    send(&mut rig, juno());
    let out = rig.frame(Vec::new());
    assert!(shows(&out, "+ CART"), "{:?}", text_list(&out));
    assert_eq!(pill(&rig).tip, "Add to your Juno cart, €13,56");
    click_pill(&mut rig, None);
    // IN CART before the store answers.
    assert!(pill(&rig).in_cart);
    let out = rig.frame(Vec::new());
    assert!(shows(&out, "IN CART"), "{:?}", text_list(&out));
    let a = next_action(&rig).expect("queued");
    assert_eq!(
        (a.store, a.kind, a.item.as_str()),
        (Store::Juno, CartKind::Add, PRODUCT)
    );
    let basket = vec![BasketItem {
        item: PRODUCT.into(),
        price: "€13,56".into(),
    }];
    answer(&mut rig, &a, Outcome::Added, Some(basket));
    assert_eq!(message(&rig), "Added to your Juno cart");
    assert!(pill(&rig).in_cart);
    assert_eq!(pill(&rig).tip, "In your Juno cart, €13,56: click to remove");
    // Remove.
    click_pill(&mut rig, None);
    assert!(!pill(&rig).in_cart);
    let a = next_action(&rig).unwrap();
    assert_eq!((a.kind, a.item.as_str()), (CartKind::Remove, PRODUCT));
    answer(&mut rig, &a, Outcome::Removed, Some(Vec::new()));
    assert_eq!(message(&rig), "Removed from your Juno cart");
    assert!(!pill(&rig).in_cart);
    assert!(fakes.browser.opened.lock().unwrap().is_empty());
}

#[test]
fn already_in_counts_as_added_and_sold_out_says_so() {
    let (mut rig, _) = rig("cart-already");
    send(&mut rig, juno());
    click_pill(&mut rig, None);
    let a = next_action(&rig).unwrap();
    answer(&mut rig, &a, Outcome::AlreadyIn, None);
    assert_eq!(message(&rig), "Already in your Juno cart");
    assert!(pill(&rig).in_cart);
    // Out again, then sold out on the way back in.
    click_pill(&mut rig, None);
    let a = next_action(&rig).unwrap();
    answer(&mut rig, &a, Outcome::Removed, None);
    click_pill(&mut rig, None);
    let a = next_action(&rig).unwrap();
    answer(&mut rig, &a, Outcome::SoldOut, None);
    assert_eq!(message(&rig), "Sold out on Juno");
    let p = pill(&rig);
    assert!(!p.in_cart && p.sold);
    let out = rig.frame(Vec::new());
    assert!(shows(&out, "SOLD"), "{:?}", text_list(&out));
}

#[test]
fn anything_else_opens_the_page_and_puts_the_pill_back() {
    for (outcome, said) in [
        (
            Outcome::NotLoggedIn,
            "Log in to Juno to use its cart: opened the page instead",
        ),
        (
            Outcome::Failed,
            "Couldn't reach your Juno cart: opened the page instead",
        ),
        (
            Outcome::NoExtension,
            "Couldn't reach your Juno cart: opened the page instead",
        ),
        (
            Outcome::TimedOut,
            "Couldn't reach your Juno cart: opened the page instead",
        ),
    ] {
        let (mut rig, fakes) = rig(&format!("cart-fallback-{outcome:?}"));
        send(&mut rig, juno());
        click_pill(&mut rig, None);
        let a = next_action(&rig).unwrap();
        match outcome {
            Outcome::NoExtension | Outcome::TimedOut => expire(&mut rig, a, outcome),
            _ => answer(&mut rig, &a, outcome, None),
        }
        assert!(!pill(&rig).in_cart, "{outcome:?}");
        assert_eq!(message(&rig), said);
        assert_eq!(*fakes.browser.opened.lock().unwrap(), [JUNO_PAGE]);
    }
}

#[test]
fn a_bleep_record_offers_its_formats() {
    let (mut rig, _) = rig("cart-bleep");
    let page = "https://bleep.com/release/100168-bibio-zen-drumsdada-drums";
    let r = record(
        Store::Bleep,
        "100168",
        page,
        "Vinyl 22.99 €, Download 2.49 €",
        vec![("277380", "Vinyl 22.99 €"), ("277377", "Download 2.49 €")],
    );
    send(&mut rig, r);
    let p = pill(&rig);
    let labels: Vec<&str> = p.choose.iter().map(|(_, l)| l.as_str()).collect();
    assert_eq!(labels, ["Vinyl 22.99 €", "Download 2.49 €"]);
    // The pill opens the menu; a format chosen is added.
    let at = texts(&rig.frame(Vec::new()))
        .into_iter()
        .find(|t| t.text == "+ CART")
        .unwrap()
        .rect
        .center();
    let out = rig.click(at);
    let out = if shows(&out, "Download 2.49 €") {
        out
    } else {
        rig.frame(Vec::new())
    };
    assert!(shows(&out, "Download 2.49 €"), "{:?}", text_list(&out));
    assert!(
        next_action(&rig).is_none(),
        "nothing queued before a choice"
    );
    click_pill(&mut rig, Some("277377"));
    let a = next_action(&rig).unwrap();
    assert_eq!(
        (a.store, a.kind, a.item.as_str()),
        (Store::Bleep, CartKind::Add, "277377")
    );
    answer(&mut rig, &a, Outcome::Added, None);
    let p = pill(&rig);
    assert!(p.in_cart && p.choose.is_empty());
    assert_eq!(
        p.tip,
        "In your Bleep cart, Download 2.49 €: click to remove"
    );
}

#[test]
fn a_bleep_record_sent_from_a_list_opens_its_page_saying_why() {
    let (mut rig, fakes) = rig("cart-bleep-list");
    // The app rebuilds the page from the id and the name part (`record`'s slug).
    let page = "https://bleep.com/release/626858-acid-pauli-gwar-vinyl";
    send(
        &mut rig,
        record(Store::Bleep, "626858", page, "", Vec::new()),
    );
    assert_eq!(pill(&rig).tip, "Open on Bleep to buy it there");
    click_pill(&mut rig, None);
    assert!(next_action(&rig).is_none(), "nothing queued");
    assert_eq!(*fakes.browser.opened.lock().unwrap(), [page]);
    assert_eq!(
        message(&rig),
        "Send the record's own page from Bleep to choose its format here: opened the page instead"
    );
}

#[test]
fn beatport_opens_the_page_at_once() {
    let (mut rig, fakes) = rig("cart-beatport");
    let page = "https://www.beatport.com/track/gwar/635821";
    send(
        &mut rig,
        record(Store::Beatport, "t635821", page, "$1.69", Vec::new()),
    );
    click_pill(&mut rig, None);
    assert!(next_action(&rig).is_none(), "nothing queued");
    assert_eq!(*fakes.browser.opened.lock().unwrap(), [page]);
    assert_eq!(
        message(&rig),
        "Beatport's cart can't be reached from Diggr: opened the page instead"
    );
    assert!(
        !shows(&rig.frame(Vec::new()), "PAY »"),
        "never in a cart: no PAY »"
    );
}

#[test]
fn the_checkout_pill_opens_the_juno_basket() {
    let (mut rig, fakes) = rig("cart-juno-checkout");
    send(&mut rig, juno());
    assert!(
        !shows(&rig.frame(Vec::new()), "PAY »"),
        "not in the basket yet"
    );
    click_pill(&mut rig, None);
    let a = next_action(&rig).unwrap();
    answer(&mut rig, &a, Outcome::Added, None);
    let out = rig.frame(Vec::new());
    let checkout = texts(&out)
        .into_iter()
        .find(|t| t.text == "PAY »")
        .unwrap_or_else(|| panic!("IN CART then »: {:?}", text_list(&out)))
        .rect
        .center();
    let selected = rig.app.crates.shown().selected_ids();
    rig.click(checkout);
    assert_eq!(
        *fakes.browser.opened.lock().unwrap(),
        ["https://www.juno.co.uk/cart/"]
    );
    assert_eq!(
        rig.app.crates.shown().selected_ids(),
        selected,
        "no selection"
    );
    assert!(pill(&rig).in_cart, "still in the basket");
}

#[test]
fn an_owned_juno_record_offers_nothing_to_buy() {
    let (mut rig, _) = rig("cart-owned-juno");
    send(&mut rig, juno());
    assert!(shows(&rig.frame(Vec::new()), "+ CART"));
    // Seen from the collection crate's point of view, it is owned.
    let c = rig.app.crates.create("Collection: digger").unwrap();
    let e = first(&rig);
    let p = rig.app.crates.get_mut(c).unwrap();
    p.add_waiting(&e.artist, &e.title, None, e.origin.clone(), "listed");
    rig.app.crates.set_collection(c);
    rig.app.crates.show(c);
    let out = rig.frame(Vec::new());
    assert!(!shows(&out, "+ CART"), "{:?}", text_list(&out));
}

#[test]
fn the_basket_shows_at_launch() {
    let (mut rig, _) = rig_with("cart-launch", |cache| {
        let carts = ::dig::carts::StoreCarts {
            juno: vec![BasketItem {
                item: PRODUCT.into(),
                price: "€13,56".into(),
            }],
            ..Default::default()
        };
        carts.save(cache).unwrap();
    });
    send(&mut rig, juno());
    // IN CART in the first frame, before any read.
    let out = rig.frame(Vec::new());
    assert!(pill(&rig).in_cart);
    assert!(shows(&out, "IN CART"), "{:?}", text_list(&out));
}

#[test]
fn an_added_item_is_kept_for_the_next_launch() {
    let (mut rig, _) = rig("cart-kept");
    send(&mut rig, juno());
    click_pill(&mut rig, None);
    let a = next_action(&rig).unwrap();
    answer(&mut rig, &a, Outcome::Added, None);
    let saved = ::dig::carts::StoreCarts::load(&rig.dir.join("cache"));
    assert_eq!(saved.get(Store::Juno, PRODUCT).unwrap().price, "€13,56");
}

// ---- the buy pill on every dig entry ------------------------------------------------------

/// Adds a Discogs entry of release `release` (none: a record known nowhere) to the Playlist.
fn discogs_entry(rig: &mut Rig, release: Option<u64>, bandcamp: &str) -> EntryId {
    let origin = crate::playlist::Origin {
        release,
        artist: "Kerri Chandler".into(),
        album: "Rain".into(),
        bandcamp: bandcamp.into(),
        for_sale: release.map(|_| crate::playlist::ForSale {
            count: 12,
            lowest_cents: Some(900),
            currency: "EUR".into(),
            fetched_at: 0,
        }),
        ..Default::default()
    };
    let p = rig.app.crates.get_mut(PLAYLIST).unwrap();
    p.add_waiting("Kerri Chandler", "Rain", None, Some(origin), "listed")
}

fn buy(rig: &mut Rig, id: EntryId) {
    let ctx = rig.ctx.clone();
    rig.app.apply(Action::Dig(DigAction::Buy(id)), &ctx);
}

/// The chooser on screen (a new window is measured in its first frame, drawn in the next).
fn chooser(rig: &mut Rig) -> egui::FullOutput {
    rig.frame(Vec::new());
    rig.frame(Vec::new())
}

fn choosing(rig: &Rig) -> bool {
    rig.app.dig.as_ref().unwrap().stores.buy.is_some()
}

#[test]
fn a_record_known_only_on_discogs_opens_its_marketplace() {
    let (mut rig, fakes) = rig("buy-discogs");
    let id = discogs_entry(&mut rig, Some(555), "");
    let out = rig.frame(Vec::new());
    assert!(
        shows(&out, "+ CART"),
        "every dig entry: {:?}",
        text_list(&out)
    );
    assert_eq!(pill(&rig).tip, "Open it on the Discogs marketplace");
    buy(&mut rig, id);
    assert!(!choosing(&rig), "one store: no chooser");
    assert_eq!(
        *fakes.browser.opened.lock().unwrap(),
        ["https://www.discogs.com/sell/release/555"]
    );
}

#[test]
fn a_bandcamp_entry_opens_its_page() {
    let (mut rig, fakes) = rig("buy-bandcamp");
    let page = "https://kerrichandler.bandcamp.com/track/rain";
    let id = discogs_entry(&mut rig, None, page);
    buy(&mut rig, id);
    assert!(!choosing(&rig));
    assert_eq!(*fakes.browser.opened.lock().unwrap(), [page]);
}

#[test]
fn a_record_known_nowhere_offers_six_searches_and_holds_the_keys() {
    let (mut rig, fakes) = rig("buy-nowhere");
    let id = discogs_entry(&mut rig, None, "");
    assert_eq!(pill(&rig).tip, "Search the stores for it");
    buy(&mut rig, id);
    assert!(choosing(&rig));
    let out = chooser(&mut rig);
    assert!(
        shows(&out, "Buy Kerri Chandler – Rain"),
        "{:?}",
        text_list(&out)
    );
    assert!(!shows(&out, "KNOWN"));
    for shop in [
        "Discogs",
        "Bandcamp",
        "Juno",
        "Bleep",
        "Beatport",
        "Traxsource",
    ] {
        assert!(shows(&out, shop), "{shop}: {:?}", text_list(&out));
    }
    // It holds the keyboard: H doesn't open the help.
    rig.key(Key::H, Modifiers::NONE);
    assert!(!rig.app.help);
    // A search link opens the store's search, and the chooser stays.
    rig.click_text("Bandcamp");
    assert_eq!(
        *fakes.browser.opened.lock().unwrap(),
        ["https://bandcamp.com/search?q=Kerri%20Chandler%20Rain&item_type=a"]
    );
    assert!(choosing(&rig));
    rig.key(Key::Escape, Modifiers::NONE);
    assert!(!choosing(&rig), "Esc closes it");
}

#[test]
fn a_juno_record_linked_to_discogs_chooses_between_them() {
    let (mut rig, _) = rig("buy-juno-discogs");
    send(&mut rig, juno());
    let id = first(&rig).id;
    let p = rig.app.crates.get_mut(PLAYLIST).unwrap();
    p.entries_mut()
        .next()
        .unwrap()
        .origin
        .as_mut()
        .unwrap()
        .release = Some(555);
    assert!(pill(&rig).buy, "known on two stores");
    buy(&mut rig, id);
    let out = chooser(&mut rig);
    assert!(shows(&out, "KNOWN"), "{:?}", text_list(&out));
    assert!(shows(&out, "Discogs"), "{:?}", text_list(&out));
    assert!(shows(&out, "Juno · €13,56"), "{:?}", text_list(&out));
    // Juno's button adds to the Juno basket, as its own pill does, and closes the chooser.
    rig.click_text("Juno · €13,56");
    assert!(!choosing(&rig));
    let a = next_action(&rig).expect("queued");
    assert_eq!((a.store, a.kind), (Store::Juno, CartKind::Add));
}

#[test]
fn a_discogs_record_a_juno_send_linked_gets_the_juno_basket() {
    let (mut rig, _) = rig("buy-linked");
    send(&mut rig, juno());
    // The send's record is linked to release 777 as a Discogs lookup would.
    let d = rig.app.dig.as_mut().unwrap();
    let seen = d.stores.memory_mut(Store::Juno).products.get_mut(PRODUCT);
    seen.expect("sent").release = Some(777);
    let id = discogs_entry(&mut rig, Some(777), "");
    let e = rig
        .app
        .crates
        .get(PLAYLIST)
        .unwrap()
        .get(id)
        .unwrap()
        .clone();
    let known: Vec<_> = rig.app.known_stores(&e).iter().map(|k| k.shop).collect();
    assert_eq!(known, [::dig::buy::Shop::Discogs, ::dig::buy::Shop::Juno]);
}

#[test]
fn a_local_file_has_no_pill() {
    let (mut rig, _) = rig("buy-local");
    rig.fill_playlist(1);
    assert!(rig.app.store_pill_of(&first(&rig)).is_none());
}

#[test]
fn a_bleep_record_known_on_discogs_lists_each_format() {
    let (mut rig, _) = rig("buy-bleep");
    let page = "https://bleep.com/release/100168-acid-pauli-gwar";
    let formats = vec![("501", "Vinyl £22.99"), ("502", "Digital £7.99")];
    send(&mut rig, record(Store::Bleep, "100168", page, "", formats));
    let p = rig.app.crates.get_mut(PLAYLIST).unwrap();
    p.entries_mut()
        .next()
        .unwrap()
        .origin
        .as_mut()
        .unwrap()
        .release = Some(555);
    let known = rig.app.known_stores(&first(&rig));
    let notes: Vec<(&str, &str)> = known
        .iter()
        .map(|k| (k.shop.name(), k.note.as_str()))
        .collect();
    // Its Discogs release (nothing known for sale), then each format apart.
    assert_eq!(
        notes,
        [
            ("Discogs", ""),
            ("Bleep", "Vinyl £22.99"),
            ("Bleep", "Digital £7.99"),
        ]
    );
}

// ---- the empty crate's hint ----------------------------------------------------------------

/// Where the hint's link `label` was drawn this frame, if it was.
fn hint_link(rig: &mut Rig, label: &'static str) -> Option<egui::Rect> {
    rig.frame(Vec::new());
    rig.app
        .pl_hint_links
        .iter()
        .find(|(l, _)| *l == label)
        .map(|(_, r)| *r)
}

#[test]
fn an_empty_crate_links_the_stores_and_the_extension() {
    let (mut rig, fakes) = rig("hint-stores");
    let c = rig.app.crates.create("asdf").unwrap();
    rig.app.show_crate(c);
    // Tall enough for the stores.
    rig.app.settings.playlist_rows = 20;
    let at = hint_link(&mut rig, "BANDCAMP").expect("the Bandcamp link");
    rig.click(at.center());
    assert_eq!(
        *fakes.browser.opened.lock().unwrap(),
        ["https://bandcamp.com"]
    );
    assert_eq!(rig.app.crates.shown_id(), c, "the crate stays shown");
    let at = hint_link(&mut rig, "SET UP THE EXTENSION").expect("the extension link");
    rig.click(at.center());
    assert!(rig.app.dig.as_ref().unwrap().bridge_dialog.is_some());
}

#[test]
fn a_short_empty_crate_keeps_the_four_lines() {
    let (mut rig, _) = rig("hint-short");
    let c = rig.app.crates.create("asdf").unwrap();
    rig.app.show_crate(c);
    rig.app.settings.playlist_rows = 4;
    assert!(hint_link(&mut rig, "BANDCAMP").is_none(), "no store links");
}
