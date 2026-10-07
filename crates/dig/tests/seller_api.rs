//! The real Discogs API for sellers and the cart, read only: run by hand with a token,
//! `WINAMP_DISCOGS_TOKEN=… cargo test -p dig --test seller_api -- --ignored --nocapture`.
//! Nothing here adds to or removes from the cart.

use std::sync::Arc;

use dig::discogs::cache::DiskCache;
use dig::discogs::client::Client;
use dig::discogs::seller::{self, Criteria};
use dig::discogs::{cart, transport::UreqTransport};

fn client() -> Client {
    let token = std::env::var("WINAMP_DISCOGS_TOKEN").expect("WINAMP_DISCOGS_TOKEN set");
    Client::new(
        Arc::new(UreqTransport::default()),
        Arc::new(dig::clock::RealClock::default()),
        Some(token),
        DiskCache::default(),
    )
}

#[test]
#[ignore]
fn a_real_inventory_reads_as_copies() {
    let mut c = client();
    let total = seller::count(&mut c, "decks.de", "").unwrap();
    println!("decks.de: {total} for sale");
    assert!(total > 1000);
    let path = seller::inventory_path("decks.de", "", 1, seller::PER_PAGE);
    let v = c.get_json(&path).unwrap();
    let page = seller::inventory_page(&v, &Criteria::default());
    assert_eq!(page.copies.len(), 100);
    let first = &page.copies[0];
    println!(
        "{} {} {} {}",
        first.listing,
        first.price,
        first.grades(),
        first.ships_from
    );
    assert!(first.price > 0.0 && !first.currency.is_empty());
    assert_eq!(page.pages, 100, "Discogs serves at most 100 pages");
}

#[test]
#[ignore]
fn real_purchases_give_a_first_list() {
    let list = seller::first_list(&mut client()).unwrap();
    for (r, n) in &list {
        println!(
            "{:3} orders  {:>7} for sale  {}  last {}",
            r.orders,
            n,
            r.username,
            &r.last[..10]
        );
    }
    assert!(list.len() <= seller::FIRST_LIST);
    assert!(list.iter().all(|(_, n)| *n > 0));
}

#[test]
#[ignore]
fn the_real_cart_reads() {
    let snap = cart::read(&mut client(), 0).unwrap();
    for (s, c) in &snap.sellers {
        println!("{s}: {} · {}", c.count, c.subtotal);
    }
    assert_eq!(
        snap.items.len(),
        snap.sellers.values().map(|c| c.count).sum::<usize>()
    );
}
