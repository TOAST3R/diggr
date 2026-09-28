//! Pairing a browser: a short-lived 6-digit code shown in OPT ▸ Browser… buys a long random key.
//!
//! The code is single use, lasts 2 minutes, and 5 wrong guesses lock pairing for a minute, so
//! guessing it takes far longer than it lives. Keys are 256 random bits; only their SHA-256 is
//! kept, in `<config>/dig/bridge.ron` (readable only by the user), and every comparison takes
//! the same time whatever the input.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::clock::Clock;
use crate::config;

pub const FILE: &str = "bridge.ron";
pub const DEFAULT_PORT: u16 = 47800;
/// How long a pairing code works.
pub const CODE_VALID: Duration = Duration::from_secs(120);
/// Wrong codes in a row before pairing is locked…
pub const MAX_WRONG: u32 = 5;
/// …for this long.
pub const LOCKOUT: Duration = Duration::from_secs(60);

/// `bridge.ron`: the port and the hashes of the paired browsers' keys.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BridgeFile {
    pub port: u16,
    /// SHA-256 of each key, as 64 hex characters.
    pub keys: Vec<String>,
}

impl Default for BridgeFile {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            keys: Vec::new(),
        }
    }
}

pub fn load(config: &Path) -> BridgeFile {
    std::fs::read_to_string(config::dir(config).join(FILE))
        .ok()
        .and_then(|s| ron::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(config: &Path, f: &BridgeFile) -> std::io::Result<()> {
    let text = ron::ser::to_string_pretty(f, ron::ser::PrettyConfig::default())
        .map_err(std::io::Error::other)?;
    config::write_atomic(&config::dir(config).join(FILE), text.as_bytes(), true)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairError {
    /// The code isn't the one shown.
    Wrong,
    /// No code is shown (the dialog is closed, or the code expired or was used).
    NoCode,
    /// Too many wrong codes: wait.
    Locked,
    /// The new key couldn't be saved (or no randomness was available).
    Failed(String),
}

pub struct Pairing {
    clock: Arc<dyn Clock>,
    /// The config folder; `None` keeps keys in memory only.
    config: Option<PathBuf>,
    port: u16,
    keys: Vec<[u8; 32]>,
    /// The shown code and when it expires.
    code: Option<(String, Duration)>,
    wrong: u32,
    locked_until: Option<Duration>,
}

impl Pairing {
    /// Reads `bridge.ron` from `config` (nothing is read with `None`).
    pub fn load(config: Option<PathBuf>, clock: Arc<dyn Clock>) -> Self {
        let file = config.as_deref().map(load).unwrap_or_default();
        let keys = file.keys.iter().filter_map(|h| from_hex::<32>(h)).collect();
        Self {
            clock,
            config,
            port: file.port,
            keys,
            code: None,
            wrong: 0,
            locked_until: None,
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn set_port(&mut self, port: u16) -> Result<(), String> {
        self.port = port;
        self.save()
    }

    pub fn paired(&self) -> usize {
        self.keys.len()
    }

    /// The shown code and the time it has left, while it is valid.
    pub fn code(&self) -> Option<(&str, Duration)> {
        let now = self.clock.now();
        let (code, until) = self.code.as_ref().filter(|(_, until)| now < *until)?;
        Some((code.as_str(), *until - now))
    }

    /// The shown code, or a new one when there is none (used or expired).
    pub fn ensure_code(&mut self) -> Result<(String, Duration), String> {
        if let Some((c, left)) = self.code() {
            return Ok((c.to_owned(), left));
        }
        let code = new_code().map_err(|e| e.to_string())?;
        self.code = Some((code.clone(), self.clock.now() + CODE_VALID));
        Ok((code, CODE_VALID))
    }

    /// The dialog closed: no code works until it opens again.
    pub fn clear_code(&mut self) {
        self.code = None;
    }

    /// Seconds until pairing is unlocked, while it is locked.
    pub fn locked_for(&self) -> Option<Duration> {
        let now = self.clock.now();
        self.locked_until.filter(|&t| now < t).map(|t| t - now)
    }

    /// Trades the shown code for a new key (64 hex characters). The code then stops working.
    pub fn pair(&mut self, code: &str) -> Result<String, PairError> {
        if self.locked_for().is_some() {
            return Err(PairError::Locked);
        }
        let Some((shown, _)) = self.code() else {
            return Err(PairError::NoCode);
        };
        if !ct_eq(shown.as_bytes(), code.as_bytes()) {
            self.wrong += 1;
            if self.wrong >= MAX_WRONG {
                self.wrong = 0;
                self.locked_until = Some(self.clock.now() + LOCKOUT);
            }
            return Err(PairError::Wrong);
        }
        self.wrong = 0;
        self.code = None;
        let mut key = [0u8; 32];
        getrandom::fill(&mut key).map_err(|e| PairError::Failed(e.to_string()))?;
        let key = to_hex(&key);
        self.keys.push(hash(&key));
        self.save().map_err(PairError::Failed)?;
        Ok(key)
    }

    /// True when `key` belongs to a paired browser.
    pub fn check_key(&self, key: &str) -> bool {
        if key.len() != 64 {
            return false;
        }
        let h = hash(key);
        // Every stored hash is compared, so the time taken says nothing about which matched.
        self.keys.iter().fold(false, |ok, k| ct_eq(k, &h) | ok)
    }

    /// Forget browsers: every key stops working.
    pub fn forget_all(&mut self) -> Result<(), String> {
        self.keys.clear();
        self.save()
    }

    fn save(&self) -> Result<(), String> {
        let Some(c) = &self.config else {
            return Ok(());
        };
        let f = BridgeFile {
            port: self.port,
            keys: self.keys.iter().map(|k| to_hex(k)).collect(),
        };
        save(c, &f).map_err(|e| format!("Could not save {FILE}: {e}"))
    }
}

/// Six random digits, uniformly (numbers past the last whole million are drawn again).
fn new_code() -> Result<String, getrandom::Error> {
    const LIMIT: u32 = u32::MAX - u32::MAX % 1_000_000;
    loop {
        let n = getrandom::u32()?;
        if n < LIMIT {
            return Ok(format!("{:06}", n % 1_000_000));
        }
    }
}

fn hash(key: &str) -> [u8; 32] {
    Sha256::digest(key.as_bytes()).into()
}

/// Equal lengths are compared byte for byte without stopping early.
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn from_hex<const N: usize>(s: &str) -> Option<[u8; N]> {
    if s.len() != N * 2 {
        return None;
    }
    let mut out = [0u8; N];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(s.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::FakeClock;

    fn pairing(config: Option<PathBuf>) -> (Pairing, Arc<FakeClock>) {
        let clock = Arc::new(FakeClock::default());
        (Pairing::load(config, clock.clone()), clock)
    }

    #[test]
    fn codes_are_six_digits() {
        for _ in 0..100 {
            let c = new_code().unwrap();
            assert_eq!(c.len(), 6);
            assert!(c.bytes().all(|b| b.is_ascii_digit()), "{c}");
        }
    }

    #[test]
    fn hex_round_trips() {
        let b = [0u8, 1, 0xab, 0xff];
        assert_eq!(to_hex(&b), "0001abff");
        assert_eq!(from_hex::<4>("0001abff"), Some(b));
        assert_eq!(from_hex::<4>("0001abfg"), None);
        assert_eq!(from_hex::<4>("0001ab"), None);
        assert!(ct_eq(b"abc", b"abc"));
        assert!(!ct_eq(b"abc", b"abd"));
        assert!(!ct_eq(b"abc", b"ab"));
    }

    #[test]
    fn a_code_buys_one_key_that_survives_a_restart() {
        let d = crate::test_dir("bridge-pairing");
        let (mut p, clock) = pairing(Some(d.path().to_path_buf()));
        assert_eq!(p.pair("123456"), Err(PairError::NoCode));
        let (code, left) = p.ensure_code().unwrap();
        assert_eq!(left, CODE_VALID);
        clock.advance(Duration::from_secs(30));
        assert_eq!(
            p.ensure_code().unwrap().0,
            code,
            "the same code until it is used"
        );
        let key = p.pair(&code).unwrap();
        assert_eq!(key.len(), 64);
        assert_eq!(p.pair(&code), Err(PairError::NoCode), "single use");
        assert!(p.check_key(&key));
        assert!(!p.check_key(&"0".repeat(64)));
        assert!(!p.check_key("short"));

        let text = std::fs::read_to_string(d.join("dig/bridge.ron")).unwrap();
        assert!(!text.contains(&key), "only the hash is stored");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(d.join("dig/bridge.ron"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        let (mut again, _) = pairing(Some(d.path().to_path_buf()));
        assert!(again.check_key(&key));
        assert_eq!((again.port(), again.paired()), (DEFAULT_PORT, 1));
        again.forget_all().unwrap();
        let (again, _) = pairing(Some(d.path().to_path_buf()));
        assert!(!again.check_key(&key));
    }

    #[test]
    fn codes_expire_and_guessing_locks() {
        let (mut p, clock) = pairing(None);
        let (code, _) = p.ensure_code().unwrap();
        clock.advance(CODE_VALID);
        assert_eq!(p.pair(&code), Err(PairError::NoCode), "expired");
        let (code, _) = p.ensure_code().unwrap();
        let wrong = if code == "000000" { "000001" } else { "000000" };
        for _ in 0..MAX_WRONG {
            assert_eq!(p.pair(wrong), Err(PairError::Wrong));
        }
        assert_eq!(p.pair(&code), Err(PairError::Locked), "even the right code");
        clock.advance(LOCKOUT - Duration::from_millis(1));
        assert_eq!(p.pair(&code), Err(PairError::Locked));
        clock.advance(Duration::from_millis(1));
        assert!(p.pair(&code).is_ok());
    }
}
