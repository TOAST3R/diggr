//! Juno samples: one plain HTTPS request each, without yt-dlp.
//!
//! The address is rebuilt from a checked key (see [`crate::juno`]), never taken from a page.
//! Redirects are followed by hand, and only to Juno's own hosts. The answer must be MP3 audio
//! of a sane size, and is written to a `.part` file renamed when complete, as yt-dlp does, so
//! a half-downloaded sample is never played. An answer that isn't audio (a challenge page),
//! a 403 or a 429 means Juno is refusing: that is not the sample's fault, so it is reported
//! as limited, and the track waits instead of failing.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::fetcher::{FetchError, preview_path, remove_partial};
use crate::discogs::transport::USER_AGENT;
use crate::juno;

/// No sample is larger: a 2-minute 128 kbps MP3 is about 2 MB.
pub const MAX_BYTES: u64 = 10 << 20;
const MAX_REDIRECTS: usize = 3;

/// Downloads samples (Juno's today) into the preview folder.
pub trait SampleFetcher: Send + Sync {
    /// Fetches sample `key` into its [`preview_path`], reporting percent done, until done or
    /// `cancel`.
    fn fetch(
        &self,
        key: &str,
        dir: &Path,
        progress: &dyn Fn(u8),
        cancel: &AtomicBool,
    ) -> Result<PathBuf, FetchError>;
}

type UrlOf = Box<dyn Fn(&str) -> Option<String> + Send + Sync>;
type Allowed = Box<dyn Fn(&str) -> bool + Send + Sync>;

/// The real thing: `ureq`, with the app's own user agent.
pub struct JunoHttp {
    agent: ureq::Agent,
    /// The address of a key's sample.
    url_of: UrlOf,
    /// Whether an address (the first, or a redirect) may be requested.
    allowed: Allowed,
}

/// The host of an `https://` address with no user, port or odd characters.
fn https_host(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://")?;
    let host = rest.split(['/', '?', '#']).next()?;
    (!host.is_empty()
        && host
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-'))
    .then_some(host)
}

impl Default for JunoHttp {
    fn default() -> Self {
        Self::new(
            Box::new(juno::sample_url),
            Box::new(|url| https_host(url).is_some_and(juno::sample_host)),
        )
    }
}

impl JunoHttp {
    pub fn new(url_of: UrlOf, allowed: Allowed) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(10)))
            .timeout_recv_response(Some(Duration::from_secs(30)))
            .timeout_recv_body(Some(Duration::from_secs(60)))
            .http_status_as_error(false)
            .max_redirects(0)
            .max_redirects_will_error(false)
            .build()
            .into();
        Self {
            agent,
            url_of,
            allowed,
        }
    }
}

/// Where a redirect's `Location` points, made absolute against `from`.
fn resolve(from: &str, location: &str) -> String {
    if location.starts_with('/') && !location.starts_with("//") {
        let scheme_end = from.find("://").map_or(0, |i| i + 3);
        let origin_end = from[scheme_end..]
            .find('/')
            .map_or(from.len(), |i| scheme_end + i);
        format!("{}{location}", &from[..origin_end])
    } else {
        location.to_owned()
    }
}

/// Whether `head` starts like an MP3: an ID3 tag, or an MPEG audio frame sync.
fn looks_like_mp3(head: &[u8]) -> bool {
    head.starts_with(b"ID3") || (head.len() >= 2 && head[0] == 0xFF && head[1] & 0xE0 == 0xE0)
}

impl SampleFetcher for JunoHttp {
    fn fetch(
        &self,
        key: &str,
        dir: &Path,
        progress: &dyn Fn(u8),
        cancel: &AtomicBool,
    ) -> Result<PathBuf, FetchError> {
        if juno::parse_key(key).is_none() {
            return Err(FetchError::InvalidId);
        }
        let mut url = (self.url_of)(key).ok_or(FetchError::InvalidId)?;
        let mut redirects = 0;
        let resp = loop {
            if !(self.allowed)(&url) {
                return Err(FetchError::Failed(format!("refused to fetch {url}")));
            }
            let resp = self
                .agent
                .get(&url)
                .header("User-Agent", USER_AGENT)
                .call()
                .map_err(|e| FetchError::Failed(e.to_string()))?;
            let status = resp.status().as_u16();
            match status {
                301 | 302 | 303 | 307 | 308 if redirects < MAX_REDIRECTS => {
                    let Some(to) = resp.headers().get("Location").and_then(|v| v.to_str().ok())
                    else {
                        return Err(FetchError::Failed(format!(
                            "HTTP {status} without a Location"
                        )));
                    };
                    url = resolve(&url, to);
                    redirects += 1;
                }
                200 => break resp,
                403 | 429 => {
                    return Err(FetchError::Limited(format!("Juno answered HTTP {status}")));
                }
                _ => return Err(FetchError::Failed(format!("Juno answered HTTP {status}"))),
            }
        };
        let kind = resp
            .headers()
            .get("Content-Type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if kind.starts_with("text/html") {
            return Err(FetchError::Limited("Juno answered with a page".into()));
        }
        if !kind.starts_with("audio/mpeg") {
            return Err(FetchError::Failed(format!("not audio ({kind})")));
        }
        let total = resp.body().content_length();
        if total.is_some_and(|n| n > MAX_BYTES) {
            return Err(FetchError::Failed("sample too large".into()));
        }
        std::fs::create_dir_all(dir).map_err(|e| FetchError::Failed(e.to_string()))?;
        let dest = preview_path(dir, key);
        let part = dest.with_extension("mp3.part");
        let result = (|| {
            let mut file =
                std::fs::File::create(&part).map_err(|e| FetchError::Failed(e.to_string()))?;
            let mut body = resp
                .into_body()
                .into_with_config()
                .limit(MAX_BYTES + 1)
                .reader();
            let mut buf = vec![0u8; 64 << 10];
            let (mut done, mut head) = (0u64, Vec::with_capacity(4));
            loop {
                if cancel.load(Ordering::Acquire) {
                    return Err(FetchError::Cancelled);
                }
                let n = match body.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(e) => return Err(FetchError::Failed(e.to_string())),
                };
                if head.len() < 4 {
                    head.extend(buf[..n].iter().take(4 - head.len()));
                }
                done += n as u64;
                if done > MAX_BYTES {
                    return Err(FetchError::Failed("sample too large".into()));
                }
                file.write_all(&buf[..n])
                    .map_err(|e| FetchError::Failed(e.to_string()))?;
                if let Some(t) = total.filter(|t| *t > 0) {
                    progress((done * 100 / t).min(100) as u8);
                }
            }
            if done == 0 || !looks_like_mp3(&head) {
                return Err(FetchError::Failed("not an MP3".into()));
            }
            file.flush()
                .map_err(|e| FetchError::Failed(e.to_string()))?;
            std::fs::rename(&part, &dest).map_err(|e| FetchError::Failed(e.to_string()))
        })();
        match result {
            Ok(()) => Ok(dest),
            Err(e) => {
                remove_partial(dir, key);
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// A canned answer: (status, headers, body).
    type Answer = (u16, Vec<(&'static str, String)>, Vec<u8>);

    /// A server answering each request with the next canned answer.
    fn serve(answers: Vec<Answer>) -> (String, Arc<()>) {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let base = format!("http://{}", server.server_addr().to_ip().unwrap());
        let alive = Arc::new(());
        let weak = Arc::downgrade(&alive);
        std::thread::spawn(move || {
            for (status, headers, body) in answers {
                let Ok(Some(req)) = server.recv_timeout(Duration::from_secs(5)) else {
                    return;
                };
                if weak.upgrade().is_none() {
                    return;
                }
                let mut resp = tiny_http::Response::from_data(body).with_status_code(status);
                for (k, v) in headers {
                    resp.add_header(tiny_http::Header::from_bytes(k, v).unwrap());
                }
                let _ = req.respond(resp);
            }
        });
        (base, alive)
    }

    fn http(base: &str) -> JunoHttp {
        let b = base.to_owned();
        let allow = base.to_owned();
        JunoHttp::new(
            Box::new(move |key| {
                juno::parse_key(key).map(|(p, s)| format!("{b}/MP3/SF{p}-{s}.mp3"))
            }),
            Box::new(move |url| url.starts_with(&allow)),
        )
    }

    fn mp3() -> Vec<u8> {
        let mut v = b"ID3\x04\x00\x00\x00\x00\x00\x00".to_vec();
        // Under 32 KB: tiny_http sends larger bodies chunked, without a length.
        v.extend(std::iter::repeat_n(0x55u8, 20_000));
        v
    }

    const KEY: &str = "jn.1178909-01-01-01";

    fn fetch(h: &JunoHttp, dir: &Path) -> Result<PathBuf, FetchError> {
        h.fetch(KEY, dir, &|_| {}, &AtomicBool::new(false))
    }

    #[test]
    fn a_sample_is_downloaded_after_a_redirect() {
        let dir = crate::test_dir("sample-ok");
        let (base, _alive) = serve(vec![
            (
                302,
                vec![("Location", "/MP3/SF1178909-01-01-01.mp3".into())],
                vec![],
            ),
            (200, vec![("Content-Type", "audio/mpeg".into())], mp3()),
        ]);
        let pcts = std::sync::Mutex::new(Vec::new());
        let path = http(&base)
            .fetch(
                KEY,
                &dir,
                &|p| pcts.lock().unwrap().push(p),
                &AtomicBool::new(false),
            )
            .unwrap();
        assert_eq!(path, dir.join("jn.1178909-01-01-01.mp3"));
        assert_eq!(std::fs::read(&path).unwrap(), mp3());
        assert_eq!(pcts.lock().unwrap().last(), Some(&100));
        assert_eq!(
            std::fs::read_dir(&*dir).unwrap().count(),
            1,
            "no .part left"
        );
    }

    #[test]
    fn a_page_instead_of_audio_is_juno_refusing() {
        let dir = crate::test_dir("sample-html");
        let (base, _alive) = serve(vec![(
            200,
            vec![("Content-Type", "text/html; charset=UTF-8".into())],
            b"<html>Just a moment...</html>".to_vec(),
        )]);
        assert!(matches!(
            fetch(&http(&base), &dir),
            Err(FetchError::Limited(_))
        ));
        assert_eq!(std::fs::read_dir(&*dir).map_or(0, |d| d.count()), 0);
    }

    #[test]
    fn refusals_and_missing_samples() {
        let dir = crate::test_dir("sample-status");
        for (status, limited) in [(403, true), (429, true), (404, false), (500, false)] {
            let (base, _alive) = serve(vec![(status, vec![], vec![])]);
            let r = fetch(&http(&base), &dir);
            match limited {
                true => assert!(matches!(r, Err(FetchError::Limited(_))), "{status}: {r:?}"),
                false => assert!(matches!(r, Err(FetchError::Failed(_))), "{status}: {r:?}"),
            }
        }
    }

    #[test]
    fn a_redirect_elsewhere_is_not_followed() {
        let dir = crate::test_dir("sample-elsewhere");
        let (base, _alive) = serve(vec![(
            302,
            vec![("Location", "https://example.net/x.mp3".into())],
            vec![],
        )]);
        let r = fetch(&http(&base), &dir);
        assert!(
            matches!(&r, Err(FetchError::Failed(t)) if t.contains("example.net")),
            "{r:?}"
        );
    }

    #[test]
    fn what_isnt_an_mp3_is_not_kept() {
        let dir = crate::test_dir("sample-bad");
        let big = vec![0xFFu8; (MAX_BYTES + 10) as usize];
        let (base, _alive) = serve(vec![
            (
                200,
                vec![("Content-Type", "audio/mpeg".into())],
                b"not audio at all".to_vec(),
            ),
            (200, vec![("Content-Type", "audio/mpeg".into())], big),
            (200, vec![("Content-Type", "application/zip".into())], mp3()),
        ]);
        let h = http(&base);
        for _ in 0..3 {
            assert!(matches!(fetch(&h, &dir), Err(FetchError::Failed(_))));
        }
        assert_eq!(std::fs::read_dir(&*dir).map_or(0, |d| d.count()), 0);
    }

    #[test]
    fn bad_keys_never_make_a_request() {
        let dir = crate::test_dir("sample-key");
        let h = JunoHttp::new(
            Box::new(|_| panic!("no address for a bad key")),
            Box::new(|_| panic!("no request for a bad key")),
        );
        for key in ["jn.1178909-01-01", "jn.../x", "dQw4w9WgXcQ", "bc.12"] {
            assert_eq!(
                h.fetch(key, &dir, &|_| {}, &AtomicBool::new(false)),
                Err(FetchError::InvalidId),
                "{key}"
            );
        }
    }

    #[test]
    fn only_juno_hosts_over_https() {
        let real = JunoHttp::default();
        for ok in [
            "https://mp3.juno.co.uk/MP3/SF1-01-01-01.mp3",
            "https://www.juno.co.uk/MP3/SF1-01-01-01.mp3",
        ] {
            assert!((real.allowed)(ok), "{ok}");
        }
        for bad in [
            "http://mp3.juno.co.uk/MP3/x.mp3",
            "https://mp3.juno.co.uk.evil.net/x.mp3",
            "https://user@mp3.juno.co.uk/x.mp3",
            "https://mp3.juno.co.uk:8443/x.mp3",
            "https://example.net/x.mp3",
        ] {
            assert!(!(real.allowed)(bad), "{bad}");
        }
        assert_eq!(
            (real.url_of)(KEY).as_deref(),
            Some("https://mp3.juno.co.uk/MP3/SF1178909-01-01-01.mp3")
        );
        assert_eq!(
            resolve("https://www.juno.co.uk/MP3/a.mp3", "/MP3/b.mp3"),
            "https://www.juno.co.uk/MP3/b.mp3"
        );
        assert_eq!(
            resolve(
                "https://www.juno.co.uk/MP3/a.mp3",
                "https://mp3.juno.co.uk/MP3/a.mp3"
            ),
            "https://mp3.juno.co.uk/MP3/a.mp3"
        );
    }

    /// Needs the network: `cargo test -p dig -- --ignored real_juno`.
    #[test]
    #[ignore]
    fn real_juno_sample() {
        let dir = crate::test_dir("sample-real");
        let path = fetch(&JunoHttp::default(), &dir).expect("downloaded");
        assert!(std::fs::metadata(&path).unwrap().len() > 500_000);
    }
}
