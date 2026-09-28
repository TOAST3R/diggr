//! Background reading of playlist metadata (tags and duration from headers only), on a
//! low-priority thread so big adds never disturb playback. After a batch's tags, the score
//! cache is asked for each track's tempo; that costs a content hash, never an analysis, and
//! comes second so titles fill in first.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use analysis::cache::{ScoreCache, content_hash};
use audio::TrackInfo;
use audio::decode::TrackDecoder;
use platform::{FileSource, Priority, Spawner, TrackRef};

/// A result for the key the track was requested with (the app uses crate and entry ids).
#[derive(Debug, Clone, PartialEq)]
pub enum MetaResult<K> {
    Info(K, TrackInfo),
    Failed(K),
    /// The analysed tempo of a track that was analysed before (from the score cache).
    Bpm(K, f64),
}

pub struct MetaWorker<K> {
    requests: Sender<Vec<(K, TrackRef)>>,
    results: Receiver<MetaResult<K>>,
}

/// How many results to deliver before waking the UI (limits repaints during big adds).
const WAKE_EVERY: usize = 25;

impl<K: Clone + Send + 'static> MetaWorker<K> {
    /// `wake` is called (from the worker thread) when new results are ready. With `scores`,
    /// tracks found in the score cache also report their tempo.
    pub fn start(
        spawner: &dyn Spawner,
        files: Arc<dyn FileSource>,
        scores: Option<ScoreCache>,
        wake: impl Fn() + Send + 'static,
    ) -> Result<Self, platform::PlatformError> {
        let (req_tx, req_rx) = channel::<Vec<(K, TrackRef)>>();
        let (res_tx, res_rx) = channel();
        spawner.spawn(
            "playlist-metadata",
            Priority::Low,
            Box::new(move || {
                while let Ok(batch) = req_rx.recv() {
                    let mut readable = Vec::new();
                    for (n, (id, track)) in batch.into_iter().enumerate() {
                        let result = match TrackDecoder::open(&*files, &track) {
                            Ok(mut dec) => {
                                let info = dec.complete_info(&*files, &track).clone();
                                readable.push((id.clone(), track));
                                MetaResult::Info(id, info)
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
                    let Some(scores) = &scores else { continue };
                    let mut found = false;
                    for (id, track) in readable {
                        let bpm = content_hash(&*files, &track)
                            .and_then(|h| scores.load(h))
                            .and_then(|s| s.dominant_bpm());
                        if let Some(bpm) = bpm {
                            found = true;
                            if res_tx.send(MetaResult::Bpm(id, bpm)).is_err() {
                                return;
                            }
                        }
                    }
                    if found {
                        wake();
                    }
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
        let worker = MetaWorker::start(
            &NativeSpawner,
            Arc::new(NativeFileSource),
            None,
            move || {
                w.fetch_add(1, Ordering::Relaxed);
            },
        )
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

    #[test]
    fn a_track_analysed_before_reports_its_tempo_from_the_cache() {
        let dir = platform::testing::TestDir::new("ui-meta-bpm");
        let scores = ScoreCache::new(dir.path());
        let files: Arc<dyn FileSource> = Arc::new(NativeFileSource);
        let analysed = fixture("tone.flac");
        scores
            .save(&analysis::SongScore {
                version: analysis::score::ALGORITHM_VERSION,
                content_hash: content_hash(&*files, &analysed).unwrap(),
                tempo_segments: vec![analysis::score::TempoSegment {
                    start: 0.0,
                    end: 2.0,
                    t0: 0.0,
                    period: 60.0 / 126.0,
                    confidence: 1.0,
                }],
                complete: true,
                ..Default::default()
            })
            .unwrap();
        let worker = MetaWorker::start(&NativeSpawner, files, Some(scores), || {}).unwrap();
        worker.request(vec![(1, analysed), (2, fixture("tone.wav"))]);
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut got = Vec::new();
        while !got.iter().any(|r| matches!(r, MetaResult::Bpm(..))) && Instant::now() < deadline {
            got.extend(worker.poll());
            std::thread::sleep(Duration::from_millis(5));
        }
        std::thread::sleep(Duration::from_millis(50));
        got.extend(worker.poll());
        let bpms: Vec<_> = got
            .iter()
            .filter_map(|r| match r {
                MetaResult::Bpm(k, b) => Some((*k, b.round())),
                _ => None,
            })
            .collect();
        assert_eq!(
            bpms,
            [(1, 126.0)],
            "only the analysed track, after the tags"
        );
        assert!(
            matches!(got[0], MetaResult::Info(1, _)) && matches!(got[1], MetaResult::Info(2, _))
        );
    }
}
