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
