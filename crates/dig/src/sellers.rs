//! Top Sellers: the user's list of sellers, kept in `<config>/dig/sellers.ron`.
//!
//! Each seller has a crate ("Seller: ‹name›") whose entries are saved like any crate's; this
//! file holds what belongs to the seller rather than to the tracks: the order of the list,
//! the criteria a dig used (a refresh reuses them), when it was last dug and how many copies
//! the seller had. The list is filled once from the user's purchases (`seeded`), then it is
//! the user's: removing every seller never fills it again.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::discogs::seller::Criteria;

pub const FILE: &str = "sellers.ron";
/// A dug crate is refreshed by a double-click once its last dig is this old.
pub const STALE_SECS: u64 = 24 * 60 * 60;

/// How a seller came into the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Source {
    Purchases,
    Added,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seller {
    pub username: String,
    /// The crate holding its copies and tracks.
    pub crate_id: u64,
    pub source: Source,
    #[serde(default)]
    pub criteria: Criteria,
    /// Seconds since the Unix epoch; `None` until the first dig.
    #[serde(default)]
    pub last_dug: Option<u64>,
    /// Copies for sale at the last count.
    #[serde(default)]
    pub total: usize,
}

impl Seller {
    pub fn new(username: impl Into<String>, crate_id: u64, source: Source) -> Self {
        Self {
            username: username.into(),
            crate_id,
            source,
            criteria: Criteria::default(),
            last_dug: None,
            total: 0,
        }
    }

    /// What a double-click does now.
    pub fn on_double_click(&self, now: u64) -> DoubleClick {
        match self.last_dug {
            None => DoubleClick::Count,
            Some(t) if now.saturating_sub(t) > STALE_SECS => DoubleClick::Refresh,
            Some(_) => DoubleClick::Show,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoubleClick {
    /// Never dug: count the copies, then ask (Dig or Narrow down).
    Count,
    /// Dug more than a day ago: refresh with the saved criteria.
    Refresh,
    /// Dug within a day: just show it.
    Show,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SellerList {
    /// The first list was made from the purchases (it never is again).
    pub seeded: bool,
    /// TOP SELLERS is folded in the sidebar.
    pub folded: bool,
    /// In the sidebar's order.
    pub sellers: Vec<Seller>,
}

impl SellerList {
    pub fn path(config: &Path) -> PathBuf {
        crate::config::dir(config).join(FILE)
    }

    /// Missing or broken: an empty, unseeded list (a broken file never blocks launch).
    pub fn load(config: &Path) -> Self {
        std::fs::read_to_string(Self::path(config))
            .ok()
            .and_then(|s| ron::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, config: &Path) -> std::io::Result<()> {
        let text = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .map_err(std::io::Error::other)?;
        crate::config::write_atomic(&Self::path(config), text.as_bytes(), false)
    }

    pub fn find(&self, username: &str) -> Option<&Seller> {
        self.sellers
            .iter()
            .find(|s| s.username.eq_ignore_ascii_case(username))
    }

    pub fn find_mut(&mut self, username: &str) -> Option<&mut Seller> {
        self.sellers
            .iter_mut()
            .find(|s| s.username.eq_ignore_ascii_case(username))
    }

    pub fn by_crate(&self, crate_id: u64) -> Option<&Seller> {
        self.sellers.iter().find(|s| s.crate_id == crate_id)
    }

    pub fn by_crate_mut(&mut self, crate_id: u64) -> Option<&mut Seller> {
        self.sellers.iter_mut().find(|s| s.crate_id == crate_id)
    }

    /// Puts a seller first (an added one), unless it is there already.
    pub fn add_first(&mut self, seller: Seller) -> bool {
        if self.find(&seller.username).is_some() {
            return false;
        }
        self.sellers.insert(0, seller);
        true
    }

    pub fn remove_crate(&mut self, crate_id: u64) -> Option<Seller> {
        let i = self.sellers.iter().position(|s| s.crate_id == crate_id)?;
        Some(self.sellers.remove(i))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_survives_a_restart() {
        let d = crate::test_dir("sellers");
        let mut l = SellerList {
            seeded: true,
            folded: true,
            ..SellerList::default()
        };
        let mut s = Seller::new("decks.de", 12, Source::Purchases);
        s.criteria.query = "techno".into();
        s.last_dug = Some(1_790_000_000);
        s.total = 40_728;
        l.sellers.push(s);
        l.sellers.push(Seller::new("logon", 13, Source::Added));
        l.save(d.path()).unwrap();
        assert_eq!(SellerList::load(d.path()), l);
    }

    #[test]
    fn a_missing_or_broken_file_is_an_empty_list() {
        let d = crate::test_dir("sellers-broken");
        assert_eq!(SellerList::load(d.path()), SellerList::default());
        std::fs::create_dir_all(SellerList::path(d.path()).parent().unwrap()).unwrap();
        std::fs::write(SellerList::path(d.path()), "not ron").unwrap();
        assert!(!SellerList::load(d.path()).seeded);
    }

    #[test]
    fn double_click_counts_refreshes_or_shows() {
        let mut s = Seller::new("logon", 1, Source::Purchases);
        let now = 1_790_000_000;
        assert_eq!(s.on_double_click(now), DoubleClick::Count);
        s.last_dug = Some(now - 30 * 3600);
        assert_eq!(s.on_double_click(now), DoubleClick::Refresh);
        s.last_dug = Some(now - 2 * 3600);
        assert_eq!(s.on_double_click(now), DoubleClick::Show);
    }

    #[test]
    fn added_sellers_go_first_once() {
        let mut l = SellerList::default();
        l.sellers
            .push(Seller::new("decks.de", 1, Source::Purchases));
        assert!(l.add_first(Seller::new("logon", 2, Source::Added)));
        assert!(
            !l.add_first(Seller::new("LOGON", 3, Source::Added)),
            "names ignore case"
        );
        assert_eq!(l.sellers[0].username, "logon");
        assert_eq!(l.by_crate(1).unwrap().username, "decks.de");
        assert_eq!(l.remove_crate(2).unwrap().username, "logon");
        assert!(l.find("logon").is_none());
    }
}
