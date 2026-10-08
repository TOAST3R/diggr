//! Top Sellers: a crate per seller, dug only on a double-click, and the user's cart.
//!
//! The seller list (`dig::sellers`) says which crate is whose, what criteria its last dig
//! used and when; the crates hold the copies and tracks. Every Discogs request goes through
//! the intake worker, so nothing here waits on the network: a click shows what is saved, and
//! answers come back as events.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use dig::discogs::cart::{AddResult, CartItem, CartSnapshot};
use dig::discogs::client::ApiError;
use dig::discogs::seller::{Copy, Criteria, LIMIT, Ranked, reachable};
use dig::discogs::url::{Page, PageKind};
use dig::intake::Command;
use dig::sellers::{DoubleClick, Seller, Source};

use super::DiggrApp;
use crate::crates::{CrateId, MAX_NAME};
use crate::playlist::SaleCopy;

/// The user's cart on discogs.com.
pub const CART_PAGE: &str = "https://www.discogs.com/sell/cart";

/// Typing waits this long before the search text is counted or a name looked up.
pub const TYPING_PAUSE: Duration = Duration::from_millis(500);

/// Top Sellers' state while the app runs: the dialog on screen and the refreshes running.
#[derive(Default)]
pub(super) struct SellerUi {
    pub(super) dialog: Option<SellerDialog>,
    /// Refreshes running: the copies read so far, applied when the refresh ends (a failed
    /// refresh changes nothing).
    refreshes: HashMap<CrateId, Vec<SaleCopy>>,
    /// Refreshes waiting for a count (to stay within the limit) before they start.
    counting_refresh: HashMap<CrateId, String>,
    /// Copies added to the cart and not answered yet: a cart read that comes first (from
    /// before the add) doesn't take their badge away.
    cart_pending: Vec<CartItem>,
}

pub(super) enum SellerDialog {
    /// Counting, then Dig (up to the limit) or Narrow down (beyond it).
    Dig(Box<DigDialog>),
    Add(AddDialog),
    /// Remove seller…, asking.
    Remove {
        crate_id: CrateId,
        seller: String,
    },
}

/// The Dig / Narrow down dialog.
pub(super) struct DigDialog {
    pub(super) crate_id: CrateId,
    pub(super) seller: String,
    pub(super) criteria: Criteria,
    /// The count Discogs gave for `criteria.query` (the search text it was asked for).
    pub(super) counted: Option<(String, Result<usize, String>)>,
    /// The search text was changed at this moment and isn't counted yet.
    pub(super) typed: Option<Instant>,
    /// The listings read for the criteria applied here.
    pub(super) scan: Option<ScanState>,
    pub(super) min_price: String,
    pub(super) max_price: String,
    pub(super) country: String,
    /// Opened by a refresh that would bring in too many copies.
    pub(super) from_refresh: bool,
}

pub(super) struct ScanState {
    /// The search text and "newest N" it reads for.
    pub(super) query: String,
    pub(super) newest: Option<u32>,
    pub(super) copies: Vec<Copy>,
    pub(super) read: u32,
    pub(super) pages: u32,
    pub(super) done: bool,
    pub(super) error: Option<String>,
}

impl DigDialog {
    fn new(crate_id: CrateId, seller: &str, criteria: Criteria) -> Self {
        let price = |c: Option<u32>| {
            c.map(|c| format!("{:.2}", c as f64 / 100.0))
                .unwrap_or_default()
        };
        Self {
            crate_id,
            seller: seller.to_owned(),
            min_price: price(criteria.min_cents),
            max_price: price(criteria.max_cents),
            country: criteria.ships_from.join(", "),
            criteria,
            counted: None,
            typed: None,
            scan: None,
            from_refresh: false,
        }
    }

    /// Copies for sale matching the search text, as Discogs counted them.
    pub(super) fn total(&self) -> Option<usize> {
        match &self.counted {
            Some((q, Ok(n))) if *q == self.criteria.query && self.typed.is_none() => Some(*n),
            _ => None,
        }
    }

    /// The scan fits the current search text and "newest N".
    fn scan_fits(&self) -> bool {
        self.scan.as_ref().is_some_and(|s| {
            s.query == self.criteria.query && s.newest == self.criteria.newest && s.error.is_none()
        })
    }

    /// Copies a dig would bring in, and whether that is the final number (not "so far").
    pub(super) fn matching(&self) -> Option<(usize, bool)> {
        let total = self.total()?;
        if !self.criteria.is_local() {
            return Some((reachable(total, &self.criteria), true));
        }
        let scan = self.scan.as_ref().filter(|_| self.scan_fits())?;
        let n = scan
            .copies
            .iter()
            .filter(|c| self.criteria.matches(c))
            .count();
        Some((n, scan.done))
    }

    /// Dig is offered: the number is final and within the limit.
    pub(super) fn can_dig(&self) -> bool {
        self.matching()
            .is_some_and(|(n, exact)| exact && (1..=LIMIT).contains(&n))
    }

    /// The criteria applied here need the listings, and none fit them yet.
    pub(super) fn needs_scan(&self) -> bool {
        self.criteria.is_local() && self.total().is_some() && !self.scan_fits()
    }

    /// Reads the price and country fields into the criteria.
    pub(super) fn read_fields(&mut self) {
        let cents = |t: &str| {
            let t = t.trim().replace(',', ".");
            (!t.is_empty())
                .then(|| t.parse::<f64>().ok())
                .flatten()
                .filter(|v| *v >= 0.0)
                .map(|v| (v * 100.0).round() as u32)
        };
        self.criteria.min_cents = cents(&self.min_price);
        self.criteria.max_cents = cents(&self.max_price);
        self.criteria.ships_from = self
            .country
            .split(',')
            .map(|c| c.trim().to_owned())
            .filter(|c| !c.is_empty())
            .collect();
    }
}

/// A seller looked up: their username and copies for sale, or why not.
pub(super) type Lookup = Result<(String, usize), String>;

/// Add seller…
#[derive(Default)]
pub(super) struct AddDialog {
    pub(super) text: String,
    pub(super) typed: Option<Instant>,
    /// The name looked up, and the answer: the username and copies for sale.
    pub(super) found: Option<(String, Lookup)>,
}

impl AddDialog {
    /// The seller named by the field: a seller's page, or a bare username.
    pub(super) fn name(&self) -> Option<String> {
        let t = self.text.trim();
        if t.is_empty() {
            return None;
        }
        match dig::discogs::url::parse(t) {
            Ok(Page {
                kind: PageKind::Seller(n),
                ..
            }) => Some(n),
            Ok(_) => None,
            Err(_) if !t.contains(['/', ' ', '?', '#']) => Some(t.to_owned()),
            Err(_) => None,
        }
    }
}

/// A copy as a seller crate keeps it.
pub fn sale_copy(c: &Copy) -> SaleCopy {
    SaleCopy {
        listing: c.listing,
        release: c.release,
        cents: (c.price * 100.0).round() as u64,
        currency: c.currency.clone(),
        grades: c.grades(),
        ships_from: c.ships_from.clone(),
        posted: c.posted.clone(),
        comments: c.comments.clone(),
        sold: c.sold,
        was_cents: None,
    }
}

/// What a refresh changed: copies new, sold and cheaper.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RefreshSummary {
    pub new: usize,
    pub sold: usize,
    pub cheaper: usize,
}

/// The crate's copies after a refresh read `fresh`: new ones come in, those not listed any
/// more are kept as sold, prices follow.
pub fn merge_copies(old: &[SaleCopy], fresh: Vec<SaleCopy>) -> (Vec<SaleCopy>, RefreshSummary) {
    let mut sum = RefreshSummary::default();
    let mut out: Vec<SaleCopy> = Vec::with_capacity(old.len() + fresh.len());
    for o in old {
        match fresh.iter().find(|f| f.listing == o.listing) {
            Some(f) => {
                let mut c = f.clone();
                if f.cents != o.cents {
                    c.was_cents = Some(o.cents);
                    if f.cents < o.cents {
                        sum.cheaper += 1;
                    }
                }
                out.push(c);
            }
            None => {
                if !o.sold {
                    sum.sold += 1;
                }
                out.push(SaleCopy {
                    sold: true,
                    was_cents: None,
                    ..o.clone()
                });
            }
        }
    }
    for f in fresh {
        if !old.iter().any(|o| o.listing == f.listing) {
            sum.new += 1;
            out.push(f);
        }
    }
    (out, sum)
}

/// "decks.de: 14 new, 6 sold, 3 cheaper", or "decks.de: up to date".
pub fn summary_text(seller: &str, s: RefreshSummary) -> String {
    let mut parts = Vec::new();
    if s.new > 0 {
        parts.push(format!("{} new", s.new));
    }
    if s.sold > 0 {
        parts.push(format!("{} sold", s.sold));
    }
    if s.cheaper > 0 {
        parts.push(format!("{} cheaper", s.cheaper));
    }
    if parts.is_empty() {
        format!("{seller}: up to date")
    } else {
        format!("{seller}: {}", parts.join(", "))
    }
}

/// Seller crates are named "Seller: ‹username›".
pub const SELLER_PREFIX: &str = "Seller: ";

/// "Seller: ‹name›", cut to the crates' name limit.
pub fn seller_crate_name(username: &str) -> String {
    let name = format!("{SELLER_PREFIX}{username}");
    name.chars().take(MAX_NAME).collect()
}

/// What the sidebar and the seller dialogs ask for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SellerAction {
    /// Fold or unfold TOP SELLERS.
    ToggleFold,
    /// Add seller… (the dialog).
    Add,
    /// A double-click on a seller crate: count and ask, refresh, or just show.
    Dig(CrateId),
    /// Refresh seller, whenever it was last dug.
    Refresh(CrateId),
    /// Narrow down… with the saved criteria.
    Narrow(CrateId),
    /// Remove seller… (asks first).
    Remove(CrateId),
    /// A copy's listing on discogs.com.
    OpenCopy(u64),
    /// Add these copies (listings) of the shown crate to the cart, in one request.
    CartAdd(Vec<u64>),
    /// Take a copy out of the cart.
    CartRemove(u64),
    /// The cart on discogs.com, to pay.
    OpenCart,
}

/// A seller crate as the sidebar shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SellerRow {
    pub crate_id: CrateId,
    pub dug: bool,
    pub refreshing: bool,
}

impl DiggrApp {
    /// The seller crates in Top Sellers order, and whether TOP SELLERS is folded.
    pub(super) fn seller_rows(&self) -> (Vec<SellerRow>, bool) {
        let Some(d) = &self.dig else {
            return (Vec::new(), false);
        };
        let rows = d
            .sellers
            .sellers
            .iter()
            .filter(|s| self.crates.info(s.crate_id).is_some())
            .map(|s| SellerRow {
                crate_id: s.crate_id,
                dug: s.last_dug.is_some(),
                refreshing: self.seller_busy(s.crate_id),
            })
            .collect();
        (rows, d.sellers.folded)
    }

    /// A dig or refresh of this seller's crate is running.
    pub(super) fn seller_busy(&self, crate_id: CrateId) -> bool {
        self.dig.as_ref().is_some_and(|d| d.has_job_for(crate_id))
    }

    pub(super) fn seller_act(&mut self, a: SellerAction) {
        match a {
            SellerAction::ToggleFold => {
                let Some(d) = &mut self.dig else { return };
                d.sellers.folded = !d.sellers.folded;
                let e = d.save_sellers();
                self.dig_notify(e);
            }
            SellerAction::Add => {
                if !self.seller_needs_token() {
                    let Some(d) = &mut self.dig else { return };
                    d.seller_ui.dialog = Some(SellerDialog::Add(AddDialog::default()));
                }
            }
            SellerAction::Dig(id) => self.seller_double_click(id),
            SellerAction::Refresh(id) => self.seller_refresh(id),
            SellerAction::Narrow(id) => {
                let Some(seller) = self.seller_of_crate(id) else {
                    return;
                };
                if !self.seller_needs_token() {
                    self.seller_open_dig(id, &seller.username, seller.criteria.clone(), false);
                }
            }
            SellerAction::Remove(id) => {
                let Some(seller) = self.seller_of_crate(id) else {
                    return;
                };
                let Some(d) = &mut self.dig else { return };
                d.seller_ui.dialog = Some(SellerDialog::Remove {
                    crate_id: id,
                    seller: seller.username,
                });
            }
            SellerAction::OpenCopy(listing) => {
                let url = format!("https://www.discogs.com/sell/item/{listing}");
                let Some(d) = &self.dig else { return };
                if let Err(e) = d.open_url(&url) {
                    self.notify(format!("Couldn't open the browser: {e}"));
                }
            }
            SellerAction::OpenCart => {
                let Some(d) = &self.dig else { return };
                if let Err(e) = d.open_url(CART_PAGE) {
                    self.notify(format!("Couldn't open the browser: {e}"));
                }
            }
            SellerAction::CartAdd(listings) => self.cart_add(listings),
            SellerAction::CartRemove(listing) => self.cart_remove(listing),
        }
    }

    fn seller_of_crate(&self, id: CrateId) -> Option<Seller> {
        self.dig.as_ref()?.sellers.by_crate(id).cloned()
    }

    /// Without a token, the Connect to Discogs dialog opens instead (true).
    fn seller_needs_token(&mut self) -> bool {
        let Some(d) = &mut self.dig else { return true };
        if d.has_token() {
            return false;
        }
        d.connect = Some(super::digging::ConnectDialog {
            from_wantlist: false,
        });
        true
    }

    /// Never dug: count, then ask. Dug more than a day ago: refresh. Otherwise just shown.
    fn seller_double_click(&mut self, id: CrateId) {
        self.show_crate(id);
        let Some(seller) = self.seller_of_crate(id) else {
            return;
        };
        if self.seller_needs_token() {
            return;
        }
        match seller.on_double_click(dig::now_secs()) {
            DoubleClick::Count => {
                self.seller_open_dig(id, &seller.username, seller.criteria.clone(), false)
            }
            DoubleClick::Refresh => self.seller_refresh(id),
            DoubleClick::Show => {}
        }
    }

    /// Opens Dig / Narrow down with `criteria`, and asks for the count.
    fn seller_open_dig(
        &mut self,
        id: CrateId,
        seller: &str,
        criteria: Criteria,
        from_refresh: bool,
    ) {
        let Some(d) = &mut self.dig else { return };
        d.send_cmd(Command::CancelScan);
        d.send_cmd(Command::CountSeller {
            seller: seller.to_owned(),
            query: criteria.query.clone(),
        });
        let mut dialog = DigDialog::new(id, seller, criteria);
        dialog.from_refresh = from_refresh;
        d.seller_ui.dialog = Some(SellerDialog::Dig(Box::new(dialog)));
    }

    /// Refresh seller: with the saved criteria. Criteria Discogs applies are counted first,
    /// so a refresh never goes over the limit; the others are checked when it ends.
    fn seller_refresh(&mut self, id: CrateId) {
        let Some(seller) = self.seller_of_crate(id) else {
            return;
        };
        if self.seller_needs_token() || self.seller_busy(id) {
            return;
        }
        let Some(d) = &mut self.dig else { return };
        if d.seller_ui.counting_refresh.contains_key(&id) {
            return;
        }
        d.seller_ui
            .counting_refresh
            .insert(id, seller.criteria.query.clone());
        d.send_cmd(Command::CountSeller {
            seller: seller.username,
            query: seller.criteria.query,
        });
    }

    /// A count arrived: for the dialog, or for a refresh about to start.
    pub(super) fn seller_counted(
        &mut self,
        seller: &str,
        query: &str,
        result: Result<usize, ApiError>,
    ) {
        let Some(d) = &mut self.dig else { return };
        if let Ok(n) = result
            && query.is_empty()
            && let Some(s) = d.sellers.find_mut(seller)
        {
            s.total = n;
        }
        let refresh = d.sellers.find(seller).map(|s| s.crate_id).filter(|id| {
            d.seller_ui
                .counting_refresh
                .get(id)
                .is_some_and(|q| q == query)
        });
        if let Some(id) = refresh {
            d.seller_ui.counting_refresh.remove(&id);
            let Some(s) = d.sellers.by_crate(id).cloned() else {
                return;
            };
            match result {
                Ok(total) if !s.criteria.is_local() && reachable(total, &s.criteria) > LIMIT => {
                    self.seller_open_dig(id, &s.username, s.criteria.clone(), true);
                }
                Ok(_) => self.seller_start(id, s.criteria.clone(), true),
                Err(e) => self.notify(format!("{seller}: the refresh failed: {}", e.message())),
            }
            return;
        }
        if let Some(SellerDialog::Dig(dialog)) = &mut d.seller_ui.dialog
            && dialog.seller.eq_ignore_ascii_case(seller)
            && dialog.criteria.query == query
        {
            dialog.counted = Some((query.to_owned(), result.map_err(|e| e.message())));
        }
    }

    pub(super) fn seller_scanned(
        &mut self,
        seller: &str,
        copies: Vec<Copy>,
        (read, pages, _total, done): (u32, u32, usize, bool),
    ) {
        let Some(d) = &mut self.dig else { return };
        let Some(SellerDialog::Dig(dialog)) = &mut d.seller_ui.dialog else {
            return;
        };
        if !dialog.seller.eq_ignore_ascii_case(seller) {
            return;
        }
        if let Some(scan) = &mut dialog.scan {
            scan.copies.extend(copies);
            (scan.read, scan.pages, scan.done) = (read, pages, done);
        }
    }

    pub(super) fn seller_scan_failed(&mut self, seller: &str, error: &ApiError) {
        let Some(d) = &mut self.dig else { return };
        if let Some(SellerDialog::Dig(dialog)) = &mut d.seller_ui.dialog
            && dialog.seller.eq_ignore_ascii_case(seller)
            && let Some(scan) = &mut dialog.scan
        {
            scan.error = Some(error.message());
        }
    }

    /// Each frame with the Dig dialog open: the search text is counted once typing pauses,
    /// and the listings are read when the criteria need them.
    pub(super) fn seller_dialog_tick(&mut self) {
        let Some(d) = &mut self.dig else { return };
        let Some(SellerDialog::Dig(dialog)) = &mut d.seller_ui.dialog else {
            return;
        };
        let cmd = if dialog.typed.is_some_and(|t| t.elapsed() >= TYPING_PAUSE) {
            dialog.typed = None;
            dialog.counted = None;
            Command::CountSeller {
                seller: dialog.seller.clone(),
                query: dialog.criteria.query.clone(),
            }
        } else if dialog.needs_scan() && dialog.typed.is_none() {
            dialog.scan = Some(ScanState {
                query: dialog.criteria.query.clone(),
                newest: dialog.criteria.newest,
                copies: Vec::new(),
                read: 0,
                pages: 0,
                done: false,
                error: None,
            });
            Command::ScanInventory {
                seller: dialog.seller.clone(),
                query: dialog.criteria.query.clone(),
                newest: dialog.criteria.newest,
            }
        } else {
            return;
        };
        d.send_cmd(cmd);
    }

    /// Dig in the dialog: the criteria are kept with the seller and the copies come in.
    pub(super) fn seller_dig_confirmed(&mut self) {
        let Some(d) = &mut self.dig else { return };
        let Some(SellerDialog::Dig(dialog)) = d.seller_ui.dialog.take() else {
            return;
        };
        d.send_cmd(Command::CancelScan);
        let fresh = dialog.from_refresh
            || d.sellers
                .by_crate(dialog.crate_id)
                .is_some_and(|s| s.last_dug.is_some());
        let total = dialog.total();
        if let (Some(s), Some(n)) = (d.sellers.by_crate_mut(dialog.crate_id), total)
            && dialog.criteria.query.is_empty()
        {
            s.total = n;
        }
        self.seller_start(dialog.crate_id, dialog.criteria, fresh);
    }

    pub(super) fn seller_dialog_closed(&mut self) {
        let Some(d) = &mut self.dig else { return };
        if matches!(d.seller_ui.dialog, Some(SellerDialog::Dig(_))) {
            d.send_cmd(Command::CancelScan);
        }
        d.seller_ui.dialog = None;
    }

    /// Starts a dig of the seller's copies into its crate. A refresh (`fresh`) reads every
    /// page again and applies what it read when it ends.
    fn seller_start(&mut self, id: CrateId, criteria: Criteria, fresh: bool) {
        if !self.crates.load(id) {
            return self.notify(format!("Crate \"{}\" can't be read", self.crates.name(id)));
        }
        let Some(d) = &mut self.dig else { return };
        let Some(s) = d.sellers.by_crate_mut(id) else {
            return;
        };
        s.criteria = criteria.clone();
        s.last_dug = Some(dig::now_secs());
        let seller = s.username.clone();
        let e = d.save_sellers();
        if fresh {
            d.seller_ui.refreshes.insert(id, Vec::new());
        }
        self.dig_notify(e);
        self.show_crate(id);
        self.dig_send_to(
            Page::new(PageKind::Inventory {
                seller,
                criteria,
                fresh,
            }),
            id,
        );
    }

    /// Copies from a seller's dig: into the crate, or held until its refresh ends.
    pub(super) fn seller_copies(&mut self, target: CrateId, copies: Vec<Copy>) {
        let copies: Vec<SaleCopy> = copies.iter().map(sale_copy).collect();
        if let Some(buf) = self
            .dig
            .as_mut()
            .and_then(|d| d.seller_ui.refreshes.get_mut(&target))
        {
            buf.extend(copies);
            return;
        }
        if let Some(p) = self.crates.get_mut(target)
            && p.add_copies(copies) > 0
        {
            self.crates.touch(target);
        }
    }

    /// A send into `target` ended (or failed): a seller's refresh is applied (or dropped),
    /// and the cart is read again.
    pub(super) fn seller_job_ended(&mut self, target: CrateId, ok: bool) {
        let Some(d) = &mut self.dig else { return };
        let Some(seller) = d.sellers.by_crate(target).cloned() else {
            return;
        };
        d.send_cmd(Command::ReadCart { soon: true });
        let Some(fresh) = d.seller_ui.refreshes.remove(&target) else {
            return;
        };
        if !ok {
            return;
        }
        if fresh.len() > LIMIT {
            self.notify(format!(
                "{}: more than {LIMIT} copies match now; narrow it down",
                seller.username
            ));
            self.seller_open_dig(target, &seller.username, seller.criteria.clone(), true);
            return;
        }
        let Some(p) = self.crates.get_mut(target) else {
            return;
        };
        let (copies, sum) = merge_copies(p.copies(), fresh);
        p.set_copies(copies);
        self.crates.touch(target);
        self.notify(summary_text(&seller.username, sum));
    }

    /// Remove seller…, confirmed: out of the list, and its crate deleted.
    pub(super) fn seller_remove_confirmed(&mut self, id: CrateId) {
        let Some(d) = &mut self.dig else { return };
        d.seller_ui.dialog = None;
        d.sellers.remove_crate(id);
        let e = d.save_sellers();
        self.dig_notify(e);
        self.delete_crate(id);
    }

    /// A seller's page was pasted or sent: the seller is added (and the dig flow opens), or
    /// refreshed when it is in the list already.
    pub(super) fn seller_page(&mut self, name: &str) {
        let known = self
            .dig
            .as_ref()
            .and_then(|d| d.sellers.find(name))
            .map(|s| s.crate_id);
        match known {
            Some(id) => {
                self.show_crate(id);
                self.seller_refresh(id);
            }
            None => {
                if self.seller_needs_token() {
                    return;
                }
                let Some(d) = &mut self.dig else { return };
                d.send_cmd(Command::LookupSeller(name.to_owned()));
                d.seller_ui.dialog = Some(SellerDialog::Add(AddDialog {
                    text: name.to_owned(),
                    typed: None,
                    found: None,
                }));
            }
        }
    }

    pub(super) fn seller_looked_up(
        &mut self,
        name: &str,
        result: Result<(String, usize), ApiError>,
    ) {
        let Some(d) = &mut self.dig else { return };
        if let Some(SellerDialog::Add(dialog)) = &mut d.seller_ui.dialog
            && dialog.name().is_some_and(|n| n.eq_ignore_ascii_case(name))
        {
            dialog.found = Some((
                name.to_owned(),
                result.map_err(|e| match e {
                    ApiError::NotFound => "No Discogs user by that name".to_owned(),
                    e => e.message(),
                }),
            ));
        }
    }

    /// Each frame with Add seller… open: the name is looked up once typing pauses.
    pub(super) fn seller_add_tick(&mut self) {
        let Some(d) = &mut self.dig else { return };
        let Some(SellerDialog::Add(dialog)) = &mut d.seller_ui.dialog else {
            return;
        };
        if dialog.typed.is_some_and(|t| t.elapsed() >= TYPING_PAUSE) {
            dialog.typed = None;
            dialog.found = None;
            if let Some(n) = dialog.name() {
                d.send_cmd(Command::LookupSeller(n));
            }
        }
    }

    /// A copy as a cart item: from the loaded seller crate that holds it.
    fn cart_item(&self, listing: u64) -> Option<(CrateId, CartItem)> {
        self.crates.loaded_ids().into_iter().find_map(|id| {
            let seller = self.crates.seller_of(id)?;
            let c = self.crates.get(id)?.copy(listing)?;
            Some((
                id,
                CartItem {
                    listing,
                    release: c.release,
                    seller: seller.to_owned(),
                    price: crate::format::price(c.cents, &c.currency),
                },
            ))
        })
    }

    /// Add to cart: the badges change at once; the answer says what really happened.
    fn cart_add(&mut self, listings: Vec<u64>) {
        if listings.is_empty() || self.seller_needs_token() {
            return;
        }
        let items: Vec<CartItem> = listings
            .iter()
            .filter_map(|&l| self.cart_item(l).map(|(_, i)| i))
            .collect();
        let Some(d) = &mut self.dig else { return };
        for i in items {
            d.cart.apply(i.clone(), true);
            d.seller_ui.cart_pending.push(i);
        }
        d.send_cmd(Command::CartAdd(listings));
    }

    /// Remove from cart: the badge goes at once.
    fn cart_remove(&mut self, listing: u64) {
        if self.seller_needs_token() {
            return;
        }
        let item = self.cart_item(listing).map(|(_, i)| i);
        let Some(d) = &mut self.dig else { return };
        match item {
            Some(i) => d.cart.apply(i, false),
            None => d.cart.items.retain(|i| i.listing != listing),
        }
        d.send_cmd(Command::CartRemove(listing));
    }

    /// Add to cart answered: added (or there already), sold, or opened on discogs.com instead.
    pub(super) fn cart_added(
        &mut self,
        listings: &[u64],
        result: Result<Vec<(u64, AddResult)>, ApiError>,
    ) {
        if let Some(d) = &mut self.dig {
            d.seller_ui
                .cart_pending
                .retain(|i| !listings.contains(&i.listing));
        }
        let outcomes: Vec<(u64, AddResult)> = match &result {
            Ok(r) => r.clone(),
            Err(_) => listings.iter().map(|&l| (l, AddResult::Failed)).collect(),
        };
        let (mut added, mut sold, mut opened) = (0, 0, 0);
        for (listing, outcome) in outcomes {
            match outcome {
                AddResult::Added | AddResult::AlreadyThere => added += 1,
                AddResult::Sold => {
                    sold += 1;
                    self.cart_forget(listing);
                    for id in self.crates.loaded_ids() {
                        if self
                            .crates
                            .get_mut(id)
                            .is_some_and(|p| p.mark_sold(listing))
                        {
                            self.crates.touch(id);
                        }
                    }
                }
                AddResult::Failed => {
                    opened += 1;
                    self.cart_forget(listing);
                    let Some(d) = &self.dig else { return };
                    let _ = d.open_url(&format!("https://www.discogs.com/sell/item/{listing}"));
                }
            }
        }
        let mut parts = Vec::new();
        match added {
            0 => {}
            1 => parts.push("Added to your cart".to_owned()),
            n => parts.push(format!("Added {n} to your cart")),
        }
        if sold > 0 {
            parts.push(if sold == 1 {
                "no longer for sale".to_owned()
            } else {
                format!("{sold} no longer for sale")
            });
        }
        if opened > 0 {
            let why = match &result {
                Err(e) => format!(" ({})", e.message()),
                Ok(_) => String::new(),
            };
            parts.push(format!(
                "the cart couldn't be reached{why}: opened on discogs.com instead"
            ));
        }
        if !parts.is_empty() {
            self.notify(parts.join("; "));
        }
    }

    fn cart_forget(&mut self, listing: u64) {
        if let Some(d) = &mut self.dig
            && let Some(i) = d.cart.items.iter().find(|i| i.listing == listing).cloned()
        {
            d.cart.apply(i, false);
        }
    }

    /// Remove from cart answered: a failure says so (the next read puts the badge back).
    pub(super) fn cart_removed(&mut self, _listing: u64, result: Result<(), ApiError>) {
        if let Err(e) = result {
            self.notify(format!(
                "Couldn't take it out of your cart: {}",
                e.message()
            ));
        }
    }

    /// Add in Add seller…: first in the list, its crate shown, and the dig flow open (the
    /// count is known already).
    pub(super) fn seller_add_confirmed(&mut self) {
        let Some(d) = &mut self.dig else { return };
        let Some(SellerDialog::Add(dialog)) = d.seller_ui.dialog.take() else {
            return;
        };
        let Some((_, Ok((username, total)))) = dialog.found else {
            return;
        };
        if let Some(id) = d.sellers.find(&username).map(|s| s.crate_id) {
            self.show_crate(id);
            return self.seller_refresh(id);
        }
        let Some(id) = self.seller_crate(&username) else {
            return;
        };
        let Some(d) = &mut self.dig else { return };
        let mut s = Seller::new(username.clone(), id, Source::Added);
        s.total = total;
        d.sellers.add_first(s);
        let e = d.save_sellers();
        self.dig_notify(e);
        self.show_crate(id);
        if !self.settings.show_playlist {
            self.settings.show_playlist = true;
            self.mark_settings();
        }
        let Some(d) = &mut self.dig else { return };
        let mut dialog = DigDialog::new(id, &username, Criteria::default());
        dialog.counted = Some((String::new(), Ok(total)));
        d.seller_ui.dialog = Some(SellerDialog::Dig(Box::new(dialog)));
    }

    /// Dig / Narrow down, Add seller… and Remove seller…, while one is open.
    pub(super) fn seller_dialogs_ui(&mut self, ctx: &egui::Context) {
        enum Choice {
            Dig,
            Add,
            Remove(CrateId),
            Close,
        }
        let Some(d) = &mut self.dig else { return };
        let Some(dialog) = &mut d.seller_ui.dialog else {
            return;
        };
        let mut choice = None;
        let modal = match dialog {
            SellerDialog::Dig(dlg) => {
                egui::Modal::new(egui::Id::new("seller-dig")).show(ctx, |ui| {
                    ui.set_width(380.0);
                    dig_dialog_body(ui, dlg, &mut choice, || Choice::Dig, || Choice::Close);
                })
            }
            SellerDialog::Add(dlg) => {
                egui::Modal::new(egui::Id::new("seller-add")).show(ctx, |ui| {
                    ui.set_width(380.0);
                    ui.strong("Add seller");
                    ui.label("Paste a seller's Discogs page, or type their name:");
                    let field = ui.add(
                        egui::TextEdit::singleline(&mut dlg.text)
                            .hint_text("https://www.discogs.com/seller/…/profile")
                            .desired_width(f32::INFINITY),
                    );
                    field.request_focus();
                    if field.changed() {
                        dlg.typed = Some(Instant::now());
                        dlg.found = None;
                    }
                    let known = match &dlg.found {
                        Some((_, Ok((name, n)))) => {
                            ui.label(format!("Found: {name} · {} for sale", thousands(*n)));
                            Some(name.clone())
                        }
                        Some((_, Err(e))) => {
                            ui.colored_label(egui::Color32::from_rgb(230, 90, 90), e);
                            None
                        }
                        None if dlg.name().is_some() => {
                            ui.label("Looking it up…");
                            None
                        }
                        None => {
                            ui.label(" ");
                            None
                        }
                    };
                    ui.horizontal(|ui| {
                        let label = if known.is_some()
                            && d.sellers.find(known.as_deref().unwrap_or("")).is_some()
                        {
                            "Already in Top Sellers: Refresh"
                        } else {
                            "Add"
                        };
                        if ui
                            .add_enabled(known.is_some(), egui::Button::new(label))
                            .clicked()
                        {
                            choice = Some(Choice::Add);
                        }
                        if ui.button("Cancel").clicked() {
                            choice = Some(Choice::Close);
                        }
                    });
                })
            }
            SellerDialog::Remove { crate_id, seller } => {
                let id = *crate_id;
                egui::Modal::new(egui::Id::new("seller-remove")).show(ctx, |ui| {
                    ui.set_max_width(380.0);
                    ui.strong(format!("Remove {seller} from Top Sellers?"));
                    ui.label("Its crate is deleted. Nothing changes on Discogs.");
                    ui.horizontal(|ui| {
                        if ui.button("Remove").clicked() {
                            choice = Some(Choice::Remove(id));
                        }
                        if ui.button("Cancel").clicked() {
                            choice = Some(Choice::Close);
                        }
                    });
                })
            }
        };
        if modal.should_close() || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            choice = Some(Choice::Close);
        }
        match choice {
            Some(Choice::Dig) => self.seller_dig_confirmed(),
            Some(Choice::Add) => self.seller_add_confirmed(),
            Some(Choice::Remove(id)) => self.seller_remove_confirmed(id),
            Some(Choice::Close) => self.seller_dialog_closed(),
            None => {}
        }
    }

    /// The crate of `username`'s copies: the one marked theirs, else one by that name, else a
    /// new empty one. `None` only when no crate can be made.
    pub(super) fn seller_crate(&mut self, username: &str) -> Option<CrateId> {
        let marked = self
            .crates
            .list()
            .iter()
            .find(|c| {
                c.seller
                    .as_deref()
                    .is_some_and(|s| s.eq_ignore_ascii_case(username))
            })
            .map(|c| c.id);
        let name = seller_crate_name(username);
        let id = match marked.or_else(|| self.crates.find(&name)) {
            Some(id) => id,
            None => self.crates.create(&name).ok()?,
        };
        self.crates.set_seller(id, username);
        Some(id)
    }

    /// The first Top Sellers list arrived: each seller gets an empty crate, never dug. Once
    /// only; a failure leaves the list unseeded, and it is tried again at the next launch.
    pub(super) fn dig_first_sellers(&mut self, result: Result<Vec<(Ranked, usize)>, ApiError>) {
        let Ok(list) = result else { return };
        if self.dig.as_ref().is_none_or(|d| d.sellers.seeded) {
            return;
        }
        for (r, total) in list {
            if self
                .dig
                .as_ref()
                .is_some_and(|d| d.sellers.find(&r.username).is_some())
            {
                continue;
            }
            let Some(crate_id) = self.seller_crate(&r.username) else {
                continue;
            };
            let Some(d) = &mut self.dig else { return };
            let mut s = Seller::new(r.username, crate_id, Source::Purchases);
            s.total = total;
            d.sellers.sellers.push(s);
        }
        let Some(d) = &mut self.dig else { return };
        d.sellers.seeded = true;
        let err = d.save_sellers();
        self.dig_notify(err);
    }

    /// Each frame: the shown and playing crates know which releases are in the cart (their
    /// CART switch filters by it).
    pub(super) fn seller_frame(&mut self) {
        let Some(d) = &self.dig else { return };
        let releases = d.cart.releases();
        for id in [self.crates.shown_id(), self.crates.playing_id()] {
            let Some(p) = self.crates.get_mut(id) else {
                continue;
            };
            let before = p.rev();
            p.set_in_cart(&releases);
            if p.rev() != before && id == self.crates.shown_id() {
                self.filter_changed();
            }
        }
    }

    /// The cart was read: badges follow it, and it is cached for the next launch.
    pub(super) fn dig_cart_read(&mut self, result: Result<CartSnapshot, ApiError>) {
        let Ok(snap) = result else { return };
        let Some(d) = &mut self.dig else { return };
        d.cart = snap;
        d.save_cart();
        for i in d.seller_ui.cart_pending.clone() {
            d.cart.apply(i, true);
        }
    }
}

/// "40,728".
pub fn thousands(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// The Dig / Narrow down dialog's contents.
fn dig_dialog_body<C>(
    ui: &mut egui::Ui,
    dlg: &mut DigDialog,
    choice: &mut Option<C>,
    dig: impl Fn() -> C,
    close: impl Fn() -> C,
) {
    use dig::discogs::model::Format;
    use dig::discogs::seller::{Condition, MAX_PAGES, PER_PAGE};
    let total = dlg.total();
    let narrow =
        dlg.from_refresh || dlg.criteria != Criteria::default() || total.is_some_and(|n| n > LIMIT);
    let title = if narrow { "Narrow down" } else { "Dig" };
    ui.strong(format!("{title}: {}", dlg.seller));
    match (&dlg.counted, total) {
        (_, Some(n)) if !narrow => {
            ui.label(format!(
                "{} {} for sale. They come in newest first, vinyl before the rest.",
                thousands(n),
                if n == 1 { "copy" } else { "copies" }
            ));
        }
        (Some((_, Err(e))), _) => {
            ui.colored_label(egui::Color32::from_rgb(230, 90, 90), e);
        }
        (_, None) => {
            ui.label("Counting…");
        }
        (_, Some(_)) => {}
    }
    if narrow {
        if dlg.from_refresh {
            ui.label(format!("More than {} copies match now.", thousands(LIMIT)));
        }
        if let Some(n) = total {
            ui.label(format!("{} for sale with this search", thousands(n)));
            let cap = (MAX_PAGES * PER_PAGE) as usize;
            if n > cap {
                ui.label(
                    egui::RichText::new(format!(
                        "Discogs only shows the first {}: search text or newest N narrow it on its side.",
                        thousands(cap)
                    ))
                    .weak(),
                );
            }
        }
        egui::Grid::new("seller-criteria")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Search");
                if ui
                    .add(egui::TextEdit::singleline(&mut dlg.criteria.query).desired_width(220.0))
                    .changed()
                {
                    dlg.typed = Some(Instant::now());
                }
                ui.end_row();
                ui.label("Newest");
                ui.horizontal(|ui| {
                    let mut on = dlg.criteria.newest.is_some();
                    if ui.checkbox(&mut on, "only the newest").changed() {
                        dlg.criteria.newest = on.then_some(500);
                    }
                    if let Some(n) = &mut dlg.criteria.newest {
                        ui.add(egui::DragValue::new(n).range(50..=1000).speed(10));
                    }
                });
                ui.end_row();
                ui.label("Format");
                ui.horizontal_wrapped(|ui| {
                    for f in Format::ALL {
                        let mut on = dlg.criteria.formats.contains(&f);
                        if ui.checkbox(&mut on, f.name()).changed() {
                            dlg.criteria.formats.retain(|x| *x != f);
                            if on {
                                dlg.criteria.formats.push(f);
                                dlg.criteria.formats.sort();
                            }
                        }
                    }
                });
                ui.end_row();
                ui.label("Price");
                ui.horizontal(|ui| {
                    let a = ui.add(
                        egui::TextEdit::singleline(&mut dlg.min_price)
                            .hint_text("from")
                            .desired_width(60.0),
                    );
                    ui.label("–");
                    let b = ui.add(
                        egui::TextEdit::singleline(&mut dlg.max_price)
                            .hint_text("to")
                            .desired_width(60.0),
                    );
                    if a.changed() || b.changed() {
                        dlg.read_fields();
                    }
                });
                ui.end_row();
                ui.label("Condition");
                let shown = dlg
                    .criteria
                    .min_condition
                    .map_or("Any".to_owned(), |c| format!("{} or better", c.abbr()));
                egui::ComboBox::from_id_salt("seller-condition")
                    .selected_text(shown)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut dlg.criteria.min_condition, None, "Any");
                        for c in Condition::ALL.into_iter().rev() {
                            ui.selectable_value(
                                &mut dlg.criteria.min_condition,
                                Some(c),
                                format!("{} or better", c.abbr()),
                            );
                        }
                    });
                ui.end_row();
                ui.label("Ships from");
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut dlg.country)
                            .hint_text("Germany, Spain")
                            .desired_width(220.0),
                    )
                    .changed()
                {
                    dlg.read_fields();
                }
                ui.end_row();
            });
        if let Some(scan) = dlg.scan.as_ref().filter(|s| !s.done) {
            match &scan.error {
                Some(e) => ui.colored_label(egui::Color32::from_rgb(230, 90, 90), e),
                None => ui.label(format!(
                    "Reading listings: {} of {} pages…",
                    scan.read,
                    scan.pages.max(1)
                )),
            };
        }
        match dlg.matching() {
            Some((n, true)) if n > LIMIT => {
                ui.label(format!(
                    "{} match: narrow it down to {} or fewer",
                    thousands(n),
                    thousands(LIMIT)
                ));
            }
            Some((0, true)) => {
                ui.label("No copies match");
            }
            Some((n, true)) => {
                ui.label(format!(
                    "{} {} match",
                    thousands(n),
                    if n == 1 { "copy" } else { "copies" }
                ));
            }
            Some((n, false)) => {
                ui.label(format!("{} match so far…", thousands(n)));
            }
            None => {}
        }
    }
    ui.horizontal(|ui| {
        if ui
            .add_enabled(dlg.can_dig(), egui::Button::new("Dig"))
            .clicked()
        {
            *choice = Some(dig());
        }
        if ui.button("Cancel").clicked() {
            *choice = Some(close());
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn copy(listing: u64, cents: u64) -> SaleCopy {
        SaleCopy {
            listing,
            release: 1001,
            cents,
            currency: "EUR".into(),
            ..SaleCopy::default()
        }
    }

    #[test]
    fn a_refresh_brings_new_keeps_sold_and_follows_prices() {
        let old = vec![copy(1, 900), copy(2, 1200), copy(3, 1800)];
        let fresh = vec![copy(1, 900), copy(3, 1500), copy(4, 700), copy(5, 800)];
        let (merged, sum) = merge_copies(&old, fresh);
        assert_eq!(
            sum,
            RefreshSummary {
                new: 2,
                sold: 1,
                cheaper: 1
            }
        );
        assert_eq!(
            summary_text("decks.de", sum),
            "decks.de: 2 new, 1 sold, 1 cheaper"
        );
        let two = merged.iter().find(|c| c.listing == 2).unwrap();
        assert!(two.sold, "kept, as sold");
        let three = merged.iter().find(|c| c.listing == 3).unwrap();
        assert_eq!((three.cents, three.was_cents), (1500, Some(1800)));
        assert_eq!(merged.len(), 5);
        let (again, sum) = merge_copies(
            &merged,
            vec![copy(1, 900), copy(3, 1500), copy(4, 700), copy(5, 800)],
        );
        assert_eq!(
            sum,
            RefreshSummary::default(),
            "a sold copy isn't counted twice"
        );
        assert_eq!(summary_text("decks.de", sum), "decks.de: up to date");
        assert!(again.iter().find(|c| c.listing == 2).unwrap().sold);
    }

    #[test]
    fn a_seller_is_named_by_page_or_name() {
        let named = |t: &str| {
            AddDialog {
                text: t.into(),
                ..AddDialog::default()
            }
            .name()
        };
        assert_eq!(
            named("https://www.discogs.com/seller/decks.de/profile").as_deref(),
            Some("decks.de")
        );
        assert_eq!(named(" logon ").as_deref(), Some("logon"));
        assert_eq!(
            named("https://www.discogs.com/release/1"),
            None,
            "not a seller"
        );
        assert_eq!(named("two words"), None);
        assert_eq!(named(""), None);
    }

    #[test]
    fn counts_read_with_thousands() {
        assert_eq!(thousands(40_728), "40,728");
        assert_eq!(thousands(159_183), "159,183");
        assert_eq!(thousands(999), "999");
    }

    #[test]
    fn crate_names_fit() {
        assert_eq!(seller_crate_name("decks.de"), "Seller: decks.de");
        assert_eq!(seller_crate_name(&"x".repeat(60)).chars().count(), MAX_NAME);
    }
}
