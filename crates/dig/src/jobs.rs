//! Unfinished sends, kept in `<config>/dig/jobs.ron` so quitting halfway through a label
//! loses nothing: when the crate is next shown or played, the listing goes on from its next
//! page and the records still waiting for their details are fetched.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::discogs::model::{Listed, RecordKey};
use crate::discogs::url::Page;

pub const FILE: &str = "jobs.ron";

pub type JobId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Filters {
    pub skip_passed: bool,
}

impl Default for Filters {
    fn default() -> Self {
        Self { skip_passed: true }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Job {
    pub id: JobId,
    pub page: Page,
    /// The crate its entries go to.
    pub target: u64,
    pub filters: Filters,
    /// The next listing page to fetch (from 1); past `pages` once the listing is done.
    pub next_page: u32,
    /// Listing pages in all, once the first has arrived.
    pub pages: Option<u32>,
    /// Records in the listing, as Discogs counts them.
    pub total: usize,
    /// Records finished (expanded, or left out by a filter).
    pub done: usize,
    /// Listed records still waiting for their details, in crate order.
    pub pending: Vec<Listed>,
    /// The page's name ("Label: Lowtide Tapes"); from the address until the API answers.
    pub name: String,
    /// The bare name ("Lowtide Tapes") once the API gave it; a remix credit is checked
    /// against an artist's.
    #[serde(default)]
    pub subject: Option<String>,
}

impl Job {
    pub fn new(id: JobId, page: Page, target: u64, filters: Filters) -> Self {
        let name = page.provisional_name();
        Self {
            id,
            page,
            target,
            filters,
            next_page: 1,
            pages: None,
            total: 0,
            done: 0,
            pending: Vec::new(),
            name,
            subject: None,
        }
    }

    pub fn listing_done(&self) -> bool {
        self.pages.is_some_and(|p| self.next_page > p)
    }

    pub fn is_finished(&self) -> bool {
        self.listing_done() && self.pending.is_empty()
    }

    pub fn has_pending(&self, key: RecordKey) -> bool {
        self.pending.iter().any(|l| l.key == key)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Jobs {
    pub next_id: JobId,
    pub jobs: Vec<Job>,
}

impl Jobs {
    pub fn path(config: &Path) -> PathBuf {
        crate::config::dir(config).join(FILE)
    }

    /// Missing or broken: no jobs (a broken file never blocks launch).
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

    pub fn alloc_id(&mut self) -> JobId {
        self.next_id += 1;
        self.next_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discogs::url::parse;

    #[test]
    fn jobs_round_trip() {
        let d = crate::test_dir("jobs");
        assert_eq!(Jobs::load(&d), Jobs::default());
        let mut jobs = Jobs::default();
        let id = jobs.alloc_id();
        let mut job = Job::new(
            id,
            parse("https://www.discogs.com/label/12345-Lowtide-Tapes").unwrap(),
            7,
            Filters::default(),
        );
        job.pending.push(Listed::new(RecordKey::Release(1001)));
        job.pages = Some(3);
        job.next_page = 2;
        jobs.jobs.push(job);
        jobs.save(&d).unwrap();
        let back = Jobs::load(&d);
        assert_eq!(back, jobs);
        assert_eq!(back.jobs[0].name, "Label: Lowtide Tapes");
        assert!(!back.jobs[0].listing_done());
        std::fs::write(Jobs::path(&d), "garbage(").unwrap();
        assert_eq!(Jobs::load(&d), Jobs::default());
    }
}
