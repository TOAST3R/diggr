//! The user's Discogs cart: read it, add copies, take one out.
//!
//! None of this is in Discogs' published API; it is what discogs.com itself calls, checked
//! against a real account on 2026-10-07. `GET /cart` answers one cart per seller;
//! `POST /cart/items {"item_ids": […]}` answers 201 with an error per item it refused (422
//! already there, 404 not for sale); `DELETE /cart/item/{id}` answers 204. Answers are read
//! leniently, and anything unexpected is an error the caller turns into "open the listing on
//! discogs.com". The cart is never emptied from here: `DELETE /cart` and
//! `DELETE /cart/seller/{id}` exist and are never called.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::client::{ApiError, Client};
use super::transport::Method;

/// A seller's part of the cart.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SellerCart {
    pub count: usize,
    /// As Discogs writes it ("€88.74"); empty when it doesn't say.
    pub subtotal: String,
}

/// A copy in the cart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CartItem {
    pub listing: u64,
    pub release: u64,
    pub seller: String,
    /// As Discogs writes it ("€9.00").
    pub price: String,
}

/// The whole cart, as of `read_at`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CartSnapshot {
    pub items: Vec<CartItem>,
    /// By seller username.
    pub sellers: BTreeMap<String, SellerCart>,
    /// Seconds since the Unix epoch; 0 when never read.
    pub read_at: u64,
}

/// The last cart read, in the cache folder, so badges show at launch.
pub const FILE: &str = "cart.ron";

impl CartSnapshot {
    pub fn path(cache: &Path) -> PathBuf {
        cache.join(FILE)
    }

    /// The cached cart, or an empty one never read.
    pub fn load(cache: &Path) -> Self {
        std::fs::read_to_string(Self::path(cache))
            .ok()
            .and_then(|t| ron::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, cache: &Path) -> std::io::Result<()> {
        let text = ron::ser::to_string(self).map_err(std::io::Error::other)?;
        crate::config::write_atomic(&Self::path(cache), text.as_bytes(), false)
    }

    pub fn has_listing(&self, listing: u64) -> bool {
        self.items.iter().any(|i| i.listing == listing)
    }

    /// The first copy of `release` in the cart, from any seller.
    pub fn for_release(&self, release: u64) -> Option<&CartItem> {
        self.items.iter().find(|i| i.release == release)
    }

    pub fn releases(&self) -> BTreeSet<u64> {
        self.items.iter().map(|i| i.release).collect()
    }

    /// Reflects an add or removal at once, before the next read confirms it.
    pub fn apply(&mut self, item: CartItem, added: bool) {
        self.items.retain(|i| i.listing != item.listing);
        let seller = self.sellers.entry(item.seller.clone()).or_default();
        if added {
            seller.count += 1;
            self.items.push(item);
        } else {
            seller.count = seller.count.saturating_sub(1);
        }
    }
}

/// What became of one copy in an add.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddResult {
    Added,
    AlreadyThere,
    /// No longer for sale (Discogs answered 404 for it).
    Sold,
    /// Refused for another reason, or the answer couldn't be read.
    Failed,
}

/// Reads the cart: one request.
pub fn read(client: &mut Client, now: u64) -> Result<CartSnapshot, ApiError> {
    let v = client.get_json("/cart")?;
    parse(&v, now).ok_or_else(|| ApiError::Other("the cart's answer wasn't understood".into()))
}

/// `None` when the answer isn't a list of carts.
pub fn parse(v: &Value, now: u64) -> Option<CartSnapshot> {
    let mut snap = CartSnapshot {
        read_at: now,
        ..CartSnapshot::default()
    };
    for cart in v.as_array()? {
        let seller = cart["seller"]["username"].as_str().unwrap_or("").to_owned();
        let mut count = 0;
        for item in cart["cart_items"].as_array().into_iter().flatten() {
            let Some(listing) = item["item_id"].as_u64() else {
                continue;
            };
            count += 1;
            snap.items.push(CartItem {
                listing,
                release: item["release"]["id"].as_u64().unwrap_or(0),
                seller: seller.clone(),
                price: item["price"]["formatted"].as_str().unwrap_or("").to_owned(),
            });
        }
        snap.sellers.insert(
            seller,
            SellerCart {
                count,
                subtotal: cart["subtotal"]["formatted"]
                    .as_str()
                    .unwrap_or("")
                    .to_owned(),
            },
        );
    }
    Some(snap)
}

/// Adds copies in one request; the result of each, in the order given.
pub fn add(client: &mut Client, listings: &[u64]) -> Result<Vec<(u64, AddResult)>, ApiError> {
    let r = client.call_json(
        Method::Post,
        "/cart/items",
        &serde_json::json!({ "item_ids": listings }),
    )?;
    let v: Value = serde_json::from_str(&r.body)
        .map_err(|_| ApiError::Other("the cart's answer wasn't understood".into()))?;
    let errors = v["errors"]
        .as_array()
        .ok_or_else(|| ApiError::Other("the cart's answer wasn't understood".into()))?;
    Ok(listings
        .iter()
        .map(|&id| {
            let refused = errors.iter().find(|e| e["item_id"].as_u64() == Some(id));
            let result = match refused.and_then(|e| e["status"].as_u64()) {
                None if refused.is_none() => AddResult::Added,
                Some(422) => AddResult::AlreadyThere,
                Some(404) => AddResult::Sold,
                _ => AddResult::Failed,
            };
            (id, result)
        })
        .collect())
}

/// Takes one copy out of the cart. A copy that isn't there counts as taken out.
pub fn remove(client: &mut Client, listing: u64) -> Result<(), ApiError> {
    match client.call(Method::Delete, &format!("/cart/item/{listing}")) {
        Ok(_) | Err(ApiError::NotFound) => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::FakeClock;
    use crate::discogs::cache::DiskCache;
    use crate::discogs::transport::FakeTransport;
    use std::sync::Arc;

    /// The shape `GET /cart` answered on 2026-10-07 (trimmed).
    const CART: &str = r#"[
      {"seller": {"id": 4447743, "username": "decks.de"},
       "subtotal": {"curr_abbr": "EUR", "formatted": "€32.21", "value": 32.21},
       "shipping_methods": [], "shipping_method_fees": [], "payment_methods": [], "tax": {},
       "cart_items": [
         {"item_id": 3006657095, "price": {"curr_abbr": "EUR", "formatted": "€23.21", "value": 23.21},
          "media_condition": "Mint (M)", "sleeve_condition": "Mint (M)",
          "item_url": "https://www.discogs.com/sell/item/3006657095",
          "release": {"id": 29742223, "title": "HEADSET005"}},
         {"item_id": 11, "price": {"formatted": "€9.00"}, "release": {"id": 1001}}]},
      {"seller": {"id": 2, "username": "Housevinyl.nl"},
       "subtotal": {"formatted": "€14.00"},
       "cart_items": [{"item_id": 21, "price": {"formatted": "€14.00"}, "release": {"id": 555}}]}
    ]"#;

    fn client(t: &Arc<FakeTransport>) -> Client {
        Client::new(
            t.clone(),
            Arc::new(FakeClock::default()),
            Some("tok".into()),
            DiskCache::default(),
        )
    }

    #[test]
    fn the_cart_reads_per_seller() {
        let t = Arc::new(FakeTransport::new());
        t.route("/cart", 200, CART);
        let snap = read(&mut client(&t), 77).unwrap();
        assert_eq!(snap.read_at, 77);
        assert_eq!(snap.items.len(), 3);
        assert!(snap.has_listing(3006657095) && !snap.has_listing(12));
        assert_eq!(snap.for_release(555).unwrap().seller, "Housevinyl.nl");
        assert_eq!(snap.for_release(555).unwrap().price, "€14.00");
        assert_eq!(
            snap.sellers["decks.de"],
            SellerCart {
                count: 2,
                subtotal: "€32.21".into()
            }
        );
        assert_eq!(snap.releases().len(), 3);
    }

    #[test]
    fn an_answer_not_understood_is_an_error() {
        let t = Arc::new(FakeTransport::new());
        t.route("/cart", 200, r#"{"message": "something else"}"#);
        assert!(matches!(read(&mut client(&t), 0), Err(ApiError::Other(_))));
        assert_eq!(
            parse(&serde_json::json!([]), 5).unwrap().items.len(),
            0,
            "empty cart"
        );
    }

    #[test]
    fn adds_answer_per_item() {
        let t = Arc::new(FakeTransport::new());
        t.route_for(
            Method::Post,
            "/cart/items",
            201,
            r#"{"message": "Cart merge complete", "errors": [
                {"item_id": 2, "message": "This item is already in your cart.", "status": 422},
                {"item_id": 3, "message": "This item is not for sale.", "status": 404},
                {"item_id": 4, "message": "Seller does not ship to you.", "status": 400}]}"#,
        );
        let mut c = client(&t);
        let r = add(&mut c, &[1, 2, 3, 4]).unwrap();
        assert_eq!(
            r,
            [
                (1, AddResult::Added),
                (2, AddResult::AlreadyThere),
                (3, AddResult::Sold),
                (4, AddResult::Failed)
            ]
        );
        assert_eq!(
            t.log()[0].0.body.as_deref(),
            Some(r#"{"item_ids":[1,2,3,4]}"#)
        );
        assert_eq!(t.paths(), ["/cart/items"], "one request for all of them");
    }

    #[test]
    fn an_unexpected_add_answer_is_an_error() {
        let t = Arc::new(FakeTransport::new());
        t.route_for(Method::Post, "/cart/items", 201, r#"{"ok": true}"#);
        assert!(matches!(
            add(&mut client(&t), &[1]),
            Err(ApiError::Other(_))
        ));
        t.route_for(Method::Post, "/cart/items", 500, "");
        assert!(matches!(
            add(&mut client(&t), &[1]),
            Err(ApiError::Other(_))
        ));
    }

    #[test]
    fn remove_takes_one_copy_out_and_never_empties_the_cart() {
        let t = Arc::new(FakeTransport::new());
        t.route_for(Method::Delete, "/cart/item/12", 404, "{}");
        let mut c = client(&t);
        remove(&mut c, 11).unwrap();
        remove(&mut c, 12).unwrap();
        let deletes: Vec<String> = t
            .log()
            .into_iter()
            .filter(|(r, _)| r.method == Method::Delete)
            .map(|(r, _)| r.path)
            .collect();
        assert_eq!(deletes, ["/cart/item/11", "/cart/item/12"]);
        assert!(
            !deletes
                .iter()
                .any(|p| p == "/cart" || p.starts_with("/cart/seller"))
        );
    }

    #[test]
    fn the_cart_is_cached_for_the_next_launch() {
        let d = crate::test_dir("cart-cache");
        assert_eq!(CartSnapshot::load(d.path()), CartSnapshot::default());
        let snap = parse(&serde_json::from_str(CART).unwrap(), 9).unwrap();
        snap.save(d.path()).unwrap();
        assert_eq!(CartSnapshot::load(d.path()), snap);
    }

    #[test]
    fn a_snapshot_follows_the_apps_own_changes() {
        let mut snap = parse(&serde_json::from_str(CART).unwrap(), 0).unwrap();
        let item = CartItem {
            listing: 99,
            release: 1002,
            seller: "decks.de".into(),
            price: "€7.50".into(),
        };
        snap.apply(item.clone(), true);
        assert!(snap.has_listing(99));
        assert_eq!(snap.sellers["decks.de"].count, 3);
        snap.apply(item, false);
        assert!(!snap.has_listing(99));
        assert_eq!(snap.sellers["decks.de"].count, 2);
    }
}
