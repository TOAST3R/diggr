//! Opening Discogs pages in the default browser, behind a seam so tests never open one.

use std::sync::Mutex;

pub trait Browser: Send + Sync {
    fn open(&self, url: &str) -> Result<(), String>;
}

pub struct SystemBrowser;

impl Browser for SystemBrowser {
    fn open(&self, url: &str) -> Result<(), String> {
        webbrowser::open(url).map_err(|e| e.to_string())
    }
}

/// Remembers what it was asked to open.
#[derive(Default)]
pub struct FakeBrowser {
    pub opened: Mutex<Vec<String>>,
}

impl Browser for FakeBrowser {
    fn open(&self, url: &str) -> Result<(), String> {
        self.opened.lock().unwrap().push(url.to_owned());
        Ok(())
    }
}

/// The page listing a release's copies for sale.
pub fn sell_url(release: u64) -> String {
    format!("https://www.discogs.com/sell/release/{release}")
}

/// The record's own page: its release, or its master when there is no release.
pub fn record_url(release: Option<u64>, master: Option<u64>) -> Option<String> {
    match (release, master) {
        (Some(r), _) => Some(format!("https://www.discogs.com/release/{r}")),
        (None, Some(m)) => Some(format!("https://www.discogs.com/master/{m}")),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_links_to_its_release_or_else_its_master() {
        assert_eq!(
            record_url(Some(123456), Some(99)).as_deref(),
            Some("https://www.discogs.com/release/123456")
        );
        assert_eq!(
            record_url(None, Some(99)).as_deref(),
            Some("https://www.discogs.com/master/99")
        );
        assert_eq!(record_url(None, None), None);
    }
}
