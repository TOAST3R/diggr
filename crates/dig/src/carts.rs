//! Store baskets: the app has no login to Juno or Bleep, so every add, remove or read of a
//! basket is queued here, picked up by the paired browser extension over the bridge
//! (`/v1/cart/next`), done in the user's browser session with the store's own requests, and
//! answered (`/v1/cart/result`). An action carries the store and the item's id, never an
//! address. Beatport and Traxsource baskets can't be reached at all (the store-carts design,
//! D1): their entries open the store's page instead, and nothing is queued for them.
//!
//! The baskets last read (and what the app added since) are kept in `store-carts.ron` in the
//! cache folder, so CART shows at launch before any read.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::juno;
use crate::stores::{Store, text, valid_id};

/// An action no extension picked up within this is answered "no extension".
pub const PICK_UP: Duration = Duration::from_secs(3);
/// An action picked up and not answered within this is answered "timed out".
pub const RESULT: Duration = Duration::from_secs(10);
/// How long `/v1/cart/next` is held open while nothing is pending.
pub const HOLD: Duration = Duration::from_secs(25);
/// The most items a basket read may hold.
pub const MAX_BASKET: usize = 500;

/// Whether the app can reach the store's basket through the extension.
pub fn reachable(store: Store) -> bool {
    matches!(store, Store::Juno | Store::Bleep)
}

/// A basket item's id: a Juno product (`1170462-01`), a Bleep format's product id (`277377`).
pub fn valid_item(store: Store, item: &str) -> bool {
    match store {
        Store::Juno => juno::valid_product(item),
        Store::Bleep => valid_id(item),
        Store::Beatport | Store::Traxsource => false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CartKind {
    Add,
    Remove,
    /// Read the basket (at launch): no item.
    Read,
}

/// One action for the extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartAction {
    pub id: u64,
    pub store: Store,
    pub kind: CartKind,
    /// Empty for a read.
    pub item: String,
}

impl CartAction {
    /// As `/v1/cart/next` gives it: `{"id":1,"store":"juno","kind":"add","item":"1170462-01"}`.
    pub fn wire(&self) -> Value {
        json!({
            "id": self.id,
            "store": self.store.name().to_ascii_lowercase(),
            "kind": self.kind,
            "item": self.item,
        })
    }
}

/// How an action ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Added,
    AlreadyIn,
    Removed,
    SoldOut,
    NotLoggedIn,
    Failed,
    /// A read's answer.
    Read,
    /// No extension picked it up in time (never sent by one).
    #[serde(skip)]
    NoExtension,
    /// Picked up, not answered in time (never sent by one).
    #[serde(skip)]
    TimedOut,
}

impl Outcome {
    /// Whether an extension may answer an action of `kind` with it.
    fn fits(self, kind: CartKind) -> bool {
        use Outcome::*;
        match kind {
            CartKind::Add => matches!(self, Added | AlreadyIn | SoldOut | NotLoggedIn | Failed),
            CartKind::Remove => matches!(self, Removed | NotLoggedIn | Failed),
            CartKind::Read => matches!(self, Read | NotLoggedIn | Failed),
        }
    }
}

/// One item of a basket as read, its price as the store shows it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct BasketItem {
    pub item: String,
    pub price: String,
}

/// An action's end, for the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartResult {
    pub action: CartAction,
    pub outcome: Outcome,
    /// The store's basket, when the extension read it.
    pub basket: Option<Vec<BasketItem>>,
}

/// The body of `/v1/cart/result`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultBody {
    pub id: u64,
    pub outcome: Outcome,
    #[serde(default)]
    pub basket: Option<Vec<BasketItem>>,
}

/// The actions waiting for an extension, and those given out and not answered yet.
#[derive(Debug, Default)]
pub struct CartQueue {
    last: u64,
    waiting: VecDeque<(CartAction, Instant)>,
    given: Vec<(CartAction, Instant)>,
}

impl CartQueue {
    /// Queues an action; `None` (nothing queued) for a store that can't be reached or an item
    /// that isn't one of its ids.
    pub fn push(&mut self, store: Store, kind: CartKind, item: &str, now: Instant) -> Option<u64> {
        let ok = match kind {
            CartKind::Read => item.is_empty(),
            _ => valid_item(store, item),
        };
        if !reachable(store) || !ok {
            return None;
        }
        self.last += 1;
        let action = CartAction {
            id: self.last,
            store,
            kind,
            item: item.to_owned(),
        };
        self.waiting.push_back((action, now));
        Some(self.last)
    }

    /// The oldest waiting action, now given out.
    pub fn take(&mut self, now: Instant) -> Option<CartAction> {
        let (a, _) = self.waiting.pop_front()?;
        self.given.push((a.clone(), now));
        Some(a)
    }

    /// The answer to an action given out, checked: `None` (nothing changes) when no such
    /// action is pending or the answer breaks a rule.
    pub fn answer(&mut self, body: ResultBody) -> Option<CartResult> {
        let i = self.given.iter().position(|(a, _)| a.id == body.id)?;
        let action = &self.given[i].0;
        let basket_ok = body.basket.as_ref().is_none_or(|b| {
            b.len() <= MAX_BASKET
                && b.iter()
                    .all(|it| valid_item(action.store, &it.item) && text(&it.price, 0, 20))
        });
        if !body.outcome.fits(action.kind) || !basket_ok {
            return None;
        }
        let (action, _) = self.given.remove(i);
        Some(CartResult {
            action,
            outcome: body.outcome,
            basket: body.basket,
        })
    }

    /// Actions past their time: waiting over [`PICK_UP`], or given out over [`RESULT`].
    pub fn expire(&mut self, now: Instant) -> Vec<CartResult> {
        let mut out = Vec::new();
        let late = |since: Instant, limit| now.saturating_duration_since(since) >= limit;
        while self.waiting.front().is_some_and(|(_, t)| late(*t, PICK_UP)) {
            let (action, _) = self.waiting.pop_front().expect("checked");
            out.push((action, Outcome::NoExtension));
        }
        let (old, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut self.given)
            .into_iter()
            .partition(|(_, t)| late(*t, RESULT));
        self.given = kept;
        out.extend(old.into_iter().map(|(a, _)| (a, Outcome::TimedOut)));
        out.into_iter()
            .map(|(action, outcome)| CartResult {
                action,
                outcome,
                basket: None,
            })
            .collect()
    }

    pub fn has_waiting(&self) -> bool {
        !self.waiting.is_empty()
    }
}

/// The store baskets as last known (`store-carts.ron` in the cache folder).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct StoreCarts {
    pub juno: Vec<BasketItem>,
    pub bleep: Vec<BasketItem>,
}

impl StoreCarts {
    pub fn path(cache: &Path) -> PathBuf {
        cache.join("store-carts.ron")
    }

    /// Missing or broken: empty (a broken file never blocks launch).
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

    fn basket(&self, store: Store) -> &[BasketItem] {
        match store {
            Store::Juno => &self.juno,
            Store::Bleep => &self.bleep,
            _ => &[],
        }
    }

    fn basket_mut(&mut self, store: Store) -> Option<&mut Vec<BasketItem>> {
        match store {
            Store::Juno => Some(&mut self.juno),
            Store::Bleep => Some(&mut self.bleep),
            _ => None,
        }
    }

    /// The item, when it is in the store's basket.
    pub fn get(&self, store: Store, item: &str) -> Option<&BasketItem> {
        self.basket(store).iter().find(|b| b.item == item)
    }

    /// What an action's end says of the basket: a read replaces it; without one, an add
    /// counts the item in (at `price`) and a remove takes it out. Returns whether it changed.
    pub fn apply(&mut self, r: &CartResult, price: &str) -> bool {
        let Some(basket) = self.basket_mut(r.action.store) else {
            return false;
        };
        let before = basket.clone();
        match (&r.basket, r.outcome) {
            (Some(read), _) => *basket = read.clone(),
            (None, Outcome::Added | Outcome::AlreadyIn) => {
                if !basket.iter().any(|b| b.item == r.action.item) {
                    basket.push(BasketItem {
                        item: r.action.item.clone(),
                        price: price.to_owned(),
                    });
                }
            }
            (None, Outcome::Removed) => basket.retain(|b| b.item != r.action.item),
            _ => {}
        }
        *basket != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(id: u64, outcome: Outcome, basket: Option<Vec<BasketItem>>) -> ResultBody {
        ResultBody {
            id,
            outcome,
            basket,
        }
    }

    fn item(item: &str, price: &str) -> BasketItem {
        BasketItem {
            item: item.into(),
            price: price.into(),
        }
    }

    #[test]
    fn only_ids_of_reachable_stores_are_queued() {
        let (mut q, t) = (CartQueue::default(), Instant::now());
        assert_eq!(q.push(Store::Juno, CartKind::Add, "1170462-01", t), Some(1));
        assert_eq!(q.push(Store::Bleep, CartKind::Remove, "277377", t), Some(2));
        assert_eq!(q.push(Store::Bleep, CartKind::Read, "", t), Some(3));
        for (store, item) in [
            (Store::Beatport, "123"),
            (Store::Traxsource, "123"),
            (Store::Juno, "https://evil/1170462-01"),
            (Store::Bleep, "27/../x"),
            (Store::Bleep, ""),
        ] {
            assert_eq!(q.push(store, CartKind::Add, item, t), None, "{item}");
        }
        assert_eq!(q.push(Store::Juno, CartKind::Read, "1170462-01", t), None);
        let a = q.take(t).unwrap();
        assert_eq!(
            a.wire(),
            json!({"id": 1, "store": "juno", "kind": "add", "item": "1170462-01"})
        );
    }

    #[test]
    fn answers_must_fit_an_action_given_out() {
        let (mut q, t) = (CartQueue::default(), Instant::now());
        q.push(Store::Bleep, CartKind::Add, "277377", t);
        // Not given out yet, then forged, then the wrong kind of outcome.
        assert_eq!(q.answer(body(1, Outcome::Added, None)), None);
        q.take(t);
        assert_eq!(q.answer(body(9, Outcome::Added, None)), None);
        assert_eq!(q.answer(body(1, Outcome::Removed, None)), None);
        // A basket of another store's ids, or too many items, or a long price.
        let juno = Some(vec![item("1170462-01", "")]);
        assert_eq!(q.answer(body(1, Outcome::Added, juno)), None);
        let many = Some(vec![item("1", ""); MAX_BASKET + 1]);
        assert_eq!(q.answer(body(1, Outcome::Added, many)), None);
        let long = Some(vec![item("1", &"9".repeat(21))]);
        assert_eq!(q.answer(body(1, Outcome::Added, long)), None);
        let ok = q
            .answer(body(
                1,
                Outcome::Added,
                Some(vec![item("277377", "2.49 €")]),
            ))
            .unwrap();
        assert_eq!(
            (ok.action.item.as_str(), ok.outcome),
            ("277377", Outcome::Added)
        );
        // Answered once only.
        assert_eq!(q.answer(body(1, Outcome::Added, None)), None);
    }

    #[test]
    fn late_actions_expire() {
        let (mut q, t) = (CartQueue::default(), Instant::now());
        q.push(Store::Juno, CartKind::Add, "1170462-01", t);
        q.push(Store::Juno, CartKind::Add, "1170463-01", t);
        q.take(t);
        assert!(q.expire(t + Duration::from_secs(2)).is_empty());
        let out = q.expire(t + PICK_UP);
        assert_eq!(out.len(), 1);
        assert_eq!(
            (out[0].action.item.as_str(), out[0].outcome),
            ("1170463-01", Outcome::NoExtension)
        );
        let out = q.expire(t + RESULT);
        assert_eq!(
            (out[0].action.item.as_str(), out[0].outcome),
            ("1170462-01", Outcome::TimedOut)
        );
        assert!(q.expire(t + RESULT * 2).is_empty() && !q.has_waiting());
    }

    #[test]
    fn baskets_follow_results_and_survive_a_restart() {
        let dir = crate::test_dir("store-carts");
        let mut c = StoreCarts::load(&dir);
        assert_eq!(c, StoreCarts::default());
        let action = |kind, item: &str| CartAction {
            id: 1,
            store: Store::Juno,
            kind,
            item: item.into(),
        };
        let result = |kind, item, outcome, basket| CartResult {
            action: action(kind, item),
            outcome,
            basket,
        };
        assert!(c.apply(
            &result(CartKind::Add, "1170462-01", Outcome::Added, None),
            "€13,56"
        ));
        assert_eq!(c.get(Store::Juno, "1170462-01").unwrap().price, "€13,56");
        assert!(!c.apply(
            &result(CartKind::Add, "1170462-01", Outcome::AlreadyIn, None),
            ""
        ));
        // A read says otherwise: it wins.
        let read = Some(vec![item("1000001-01", "€9,99")]);
        assert!(c.apply(&result(CartKind::Read, "", Outcome::Read, read), ""));
        assert!(c.get(Store::Juno, "1170462-01").is_none());
        c.save(&dir).unwrap();
        assert_eq!(StoreCarts::load(&dir), c);
        assert!(c.apply(
            &result(CartKind::Remove, "1000001-01", Outcome::Removed, None),
            ""
        ));
        assert!(!c.apply(&result(CartKind::Add, "1", Outcome::Failed, None), ""));
        std::fs::write(StoreCarts::path(&dir), "not ron").unwrap();
        assert_eq!(StoreCarts::load(&dir), StoreCarts::default());
    }
}
