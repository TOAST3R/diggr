//! Opening Discogs pages in the default browser, behind a seam so tests never open one.
//!
//! A browser extension's own page (`chrome-extension://…`) is opened differently: macOS and
//! Windows don't hand such an address to a browser by themselves, so the default browser is
//! looked up and started with it (only a Chromium one has the extension).

use std::sync::Mutex;

pub trait Browser: Send + Sync {
    fn open(&self, url: &str) -> Result<(), String>;
    /// Opens a browser extension's page in the default browser; an error says why it can't
    /// (not a Chromium browser, not found, not this system).
    fn open_extension_page(&self, url: &str) -> Result<(), String> {
        let _ = url;
        Err("this system can't open the extension's page".into())
    }
}

pub struct SystemBrowser;

impl Browser for SystemBrowser {
    fn open(&self, url: &str) -> Result<(), String> {
        webbrowser::open(url).map_err(|e| e.to_string())
    }

    fn open_extension_page(&self, url: &str) -> Result<(), String> {
        let (program, args) = default_browser_launch(url)?;
        crate::proc::command(&program)
            .args(&args)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("couldn't start {program}: {e}"))
    }
}

/// Default browsers that can't have the extension.
fn not_chromium(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    ["safari", "firefox", "librewolf", "waterfox", "zen", "orion"]
        .iter()
        .any(|b| n.contains(b))
}

/// How to start the default browser on `url`: macOS asks which app opens `https://`.
#[cfg(target_os = "macos")]
fn default_browser_launch(url: &str) -> Result<(String, Vec<String>), String> {
    let out = crate::proc::command("osascript")
        .args([
            "-l",
            "JavaScript",
            "-e",
            "ObjC.import('AppKit'); $.NSWorkspace.sharedWorkspace.URLForApplicationToOpenURL($.NSURL.URLWithString('https://example.com')).path.js",
        ])
        .output()
        .map_err(|e| format!("couldn't ask for the default browser: {e}"))?;
    let app = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    mac_launch(&app, url)
}

/// `open -a ‹app› ‹url›`, unless the app can't have the extension.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn mac_launch(app: &str, url: &str) -> Result<(String, Vec<String>), String> {
    if !app.ends_with(".app") {
        return Err("no default browser found".into());
    }
    let name = app
        .rsplit('/')
        .next()
        .unwrap_or(app)
        .trim_end_matches(".app");
    if not_chromium(name) {
        return Err(format!(
            "{name} can't have the extension: it needs Chrome or another Chromium browser"
        ));
    }
    Ok(("open".into(), vec!["-a".into(), app.into(), url.into()]))
}

/// How to start the default browser on `url`: Windows keeps the `https` handler's program in
/// the registry (the user's choice, then that handler's open command).
#[cfg(windows)]
fn default_browser_launch(url: &str) -> Result<(String, Vec<String>), String> {
    let reg = |args: &[&str]| -> Result<String, String> {
        let out = crate::proc::command("reg")
            .args(args)
            .output()
            .map_err(|e| format!("couldn't read the registry: {e}"))?;
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    };
    let choice = reg(&[
        "query",
        r"HKCU\Software\Microsoft\Windows\Shell\Associations\UrlAssociations\https\UserChoice",
        "/v",
        "ProgId",
    ])?;
    let prog = reg_value(&choice).ok_or("no default browser found")?;
    let command = reg(&["query", &format!(r"HKCR\{prog}\shell\open\command"), "/ve"])?;
    let exe = reg_value(&command)
        .as_deref()
        .and_then(command_exe)
        .ok_or("the default browser's program wasn't found")?;
    if not_chromium(&exe) || not_chromium(&prog) {
        return Err("the default browser can't have the extension: it needs Chrome or another Chromium browser".into());
    }
    Ok((exe, vec![url.into()]))
}

#[cfg(not(any(target_os = "macos", windows)))]
fn default_browser_launch(_url: &str) -> Result<(String, Vec<String>), String> {
    Err("this system can't open the extension's page".into())
}

/// The value in `reg query` output: the last `REG_SZ` field of its line.
#[cfg_attr(not(windows), allow(dead_code))]
fn reg_value(out: &str) -> Option<String> {
    out.lines()
        .find_map(|l| l.split_once("REG_SZ").map(|(_, v)| v.trim().to_owned()))
        .filter(|v| !v.is_empty())
}

/// The program of an open command: `"C:\…\chrome.exe" --single-argument %1` → its path.
#[cfg_attr(not(windows), allow(dead_code))]
fn command_exe(command: &str) -> Option<String> {
    let c = command.trim();
    let exe = match c.strip_prefix('"') {
        Some(rest) => rest.split('"').next()?,
        None => c.split_whitespace().next()?,
    };
    (!exe.is_empty()).then(|| exe.to_owned())
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

    fn open_extension_page(&self, url: &str) -> Result<(), String> {
        self.open(url)
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
    fn a_chromium_default_browser_is_started_on_the_page() {
        let url = "chrome-extension://abc/options.html";
        assert_eq!(
            mac_launch("/Applications/Dia.app", url).unwrap(),
            (
                "open".to_owned(),
                vec!["-a".into(), "/Applications/Dia.app".into(), url.into()]
            )
        );
        assert!(mac_launch("/Applications/Google Chrome.app", url).is_ok());
        let e = mac_launch("/Applications/Safari.app", url).unwrap_err();
        assert!(e.contains("Safari") && e.contains("Chromium"), "{e}");
        assert!(mac_launch("/Applications/Firefox.app", url).is_err());
        assert!(
            mac_launch("", url)
                .unwrap_err()
                .contains("no default browser")
        );
    }

    #[test]
    fn the_windows_registry_answers_are_read() {
        let choice =
            "\r\nHKEY_CURRENT_USER\\...\\UserChoice\r\n    ProgId    REG_SZ    ChromeHTML\r\n\r\n";
        assert_eq!(reg_value(choice).as_deref(), Some("ChromeHTML"));
        let command = "\r\nHKEY_CLASSES_ROOT\\ChromeHTML\\shell\\open\\command\r\n    (Default)    REG_SZ    \"C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe\" --single-argument %1\r\n";
        let exe = reg_value(command).and_then(|c| command_exe(&c));
        assert_eq!(
            exe.as_deref(),
            Some(r"C:\Program Files\Google\Chrome\Application\chrome.exe")
        );
        assert_eq!(
            command_exe(r"C:\Brave\brave.exe %1").as_deref(),
            Some(r"C:\Brave\brave.exe")
        );
        assert_eq!(reg_value("ERROR: not found"), None);
        assert!(not_chromium(
            r"C:\Program Files\Mozilla Firefox\firefox.exe"
        ));
        assert!(!not_chromium(
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe"
        ));
    }

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
