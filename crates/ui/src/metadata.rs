//! Background reading of playlist metadata (tags and duration from headers only), on a
//! low-priority thread so big adds never disturb playback.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use audio::TrackInfo;
use audio::decode::TrackDecoder;
use platform::{FileSource, Priority, Spawner, TrackRef};

/// A result for the key the track was requested with (the app uses crate and entry ids).
#[derive(Debug, Clone, PartialEq)]
pub enum MetaResult<K> {
    Info(K, TrackInfo),
    Failed(K),
}

pub struct MetaWorker<K> {
    requests: Sender<Vec<(K, TrackRef)>>,
    results: Receiver<MetaResult<K>>,
}

/// How many results to deliver before waking the UI (limits repaints during big adds).
const WAKE_EVERY: usize = 25;

impl<K: Send + 'static> MetaWorker<K> {
    /// `wake` is called (from the worker thread) when new results are ready.
    pub fn start(
        spawner: &dyn Spawner,
        files: Arc<dyn FileSource>,
        wake: impl Fn() + Send + 'static,
    ) -> Result<Self, platform::PlatformError> {
        let (req_tx, req_rx) = channel::<Vec<(K, TrackRef)>>();
        let (res_tx, res_rx) = channel();
        spawner.spawn(
            "playlist-metadata",
            Priority::Low,
            Box::new(move || {
                while let Ok(batch) = req_rx.recv() {
                    for (n, (id, track)) in batch.into_iter().enumerate() {
                        let result = match TrackDecoder::open(&*files, &track) {
                            Ok(mut dec) => {
                                MetaResult::Info(id, dec.complete_info(&*files, &track).clone())
                            }
                            Err(_) => MetaResult::Failed(id),
                        };
                        if res_tx.send(result).is_err() {
                            return;
                        }
                        if n % WAKE_EVERY == WAKE_EVERY - 1 {
                            wake();
                        }
                    }
                    wake();
                }
            }),
        )?;
        Ok(Self {
            requests: req_tx,
            results: res_rx,
        })
    }

    pub fn request(&self, items: Vec<(K, TrackRef)>) {
        if !items.is_empty() {
            let _ = self.requests.send(items);
        }
    }

    pub fn poll(&self) -> Vec<MetaResult<K>> {
        self.results.try_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform::native::{NativeFileSource, NativeSpawner};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    fn fixture(name: &str) -> TrackRef {
        TrackRef::new(format!(
            "{}/../audio/tests/fixtures/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
    }

    #[test]
    fn reads_tags_and_reports_failures() {
        let wakes = Arc::new(AtomicUsize::new(0));
        let w = wakes.clone();
        let worker = MetaWorker::start(&NativeSpawner, Arc::new(NativeFileSource), move || {
            w.fetch_add(1, Ordering::Relaxed);
        })
        .unwrap();
        worker.request(vec![
            (1, fixture("tone.flac")),
            (2, fixture("garbage.mp3")),
            (3, fixture("tone.wav")),
        ]);
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut got = Vec::new();
        while got.len() < 3 && Instant::now() < deadline {
            got.extend(worker.poll());
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(got.len(), 3);
        match &got[0] {
            MetaResult::Info(1, info) => {
                assert_eq!(
                    (info.artist.as_str(), info.title.as_str()),
                    ("M83", "Midnight_City")
                );
                assert!((info.duration_secs.unwrap() - 2.0).abs() < 0.01);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(got[1], MetaResult::Failed(2));
        assert!(
            matches!(&got[2], MetaResult::Info(3, i) if i.artist == "M83"),
            "WAV INFO tags"
        );
        assert!(wakes.load(Ordering::Relaxed) >= 1);
    }
}
