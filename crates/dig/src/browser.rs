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
