//! Test helpers: a sink driven by the test itself, so engine behavior can be verified without a
//! device, and folders that tests clean up after themselves.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::{
    AudioSink, CallbackInfo, ErrorFn, PlatformError, RenderFn, SinkConfig, SinkError, SinkHandle,
    StreamFormat,
};

#[derive(Default)]
struct Shared {
    render: Option<RenderFn>,
    on_error: Option<ErrorFn>,
    starts: usize,
}

/// Call [`ManualSink::pull`] to run one render callback.
#[derive(Clone)]
pub struct ManualSink {
    format: Arc<Mutex<StreamFormat>>,
    shared: Arc<Mutex<Shared>>,
    now: Arc<AtomicU64>,
}

impl ManualSink {
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self {
            format: Arc::new(Mutex::new(StreamFormat {
                sample_rate,
                channels,
                buffer_frames: Some(512),
            })),
            shared: Arc::default(),
            now: Arc::default(),
        }
    }

    /// Set the fake monotonic time returned by [`AudioSink::now_ns`].
    pub fn set_now_ns(&self, ns: u64) {
        self.now.store(ns, Ordering::Relaxed);
    }

    /// Change the format that the next `start` will report (simulates a new device).
    pub fn set_format(&self, sample_rate: u32, channels: u16) {
        let mut f = self.format.lock().unwrap();
        f.sample_rate = sample_rate;
        f.channels = channels;
    }

    /// Run the render callback for `frames` frames. Returns `None` if no stream is running.
    pub fn pull(&self, frames: usize, info: CallbackInfo) -> Option<Vec<f32>> {
        let channels = self.format.lock().unwrap().channels as usize;
        let mut buf = vec![0.0; frames * channels];
        let mut shared = self.shared.lock().unwrap();
        let render = shared.render.as_mut()?;
        render(&mut buf, info);
        Some(buf)
    }

    /// Report an error from the "device" as the backend would.
    pub fn fail(&self, err: SinkError) {
        let mut shared = self.shared.lock().unwrap();
        shared.render = None;
        if let Some(on_error) = shared.on_error.as_mut() {
            on_error(err);
        }
    }

    pub fn is_running(&self) -> bool {
        self.shared.lock().unwrap().render.is_some()
    }

    /// How many times a stream has been started.
    pub fn starts(&self) -> usize {
        self.shared.lock().unwrap().starts
    }
}

struct ManualHandle {
    format: StreamFormat,
    shared: Arc<Mutex<Shared>>,
    generation: usize,
}

impl SinkHandle for ManualHandle {
    fn format(&self) -> StreamFormat {
        self.format
    }
}

impl Drop for ManualHandle {
    fn drop(&mut self) {
        let mut shared = self.shared.lock().unwrap();
        if shared.starts == self.generation {
            shared.render = None;
        }
    }
}

impl AudioSink for ManualSink {
    fn now_ns(&self) -> u64 {
        self.now.load(Ordering::Relaxed)
    }

    fn default_format(&self, _cfg: &SinkConfig) -> Result<StreamFormat, PlatformError> {
        Ok(*self.format.lock().unwrap())
    }

    fn start(
        &self,
        format: &StreamFormat,
        render: RenderFn,
        on_error: ErrorFn,
    ) -> Result<Box<dyn SinkHandle>, PlatformError> {
        let mut shared = self.shared.lock().unwrap();
        shared.render = Some(render);
        shared.on_error = Some(on_error);
        shared.starts += 1;
        Ok(Box::new(ManualHandle {
            format: *format,
            shared: self.shared.clone(),
            generation: shared.starts,
        }))
    }
}

/// A test's own folder, `<temp>/winamp_rust-tests/<name>-<pid>`, emptied when created and
/// deleted when dropped, also when the test fails. Test runs used to leave gigabytes of
/// synthesized audio behind. The process id keeps two runs at the same time (say debug and
/// release) apart.
///
/// A background worker can still write a file after its test's folder is gone (a download or an
/// analysis finishing), and a killed run deletes nothing. So the first `TestDir` of each run also
/// deletes test folders not touched for an hour: whatever is left never outlives the next run.
pub struct TestDir {
    path: PathBuf,
}

impl TestDir {
    pub fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join("winamp_rust-tests");
        static SWEEP: std::sync::Once = std::sync::Once::new();
        SWEEP.call_once(|| sweep(&root, std::time::Duration::from_secs(3600)));
        let path = root.join(format!("{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create the test folder");
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Deletes the folders in `root` last modified more than `age` ago.
fn sweep(root: &Path, age: std::time::Duration) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for e in entries.flatten() {
        let old = e
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|a| a > age);
        if old {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
}

impl std::ops::Deref for TestDir {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<Path> for TestDir {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod test_dir_tests {
    use super::TestDir;

    #[test]
    fn a_test_dir_is_fresh_and_goes_away_when_dropped() {
        let d = TestDir::new("platform-testdir");
        std::fs::write(d.join("x"), b"1").unwrap();
        let path = d.path().to_path_buf();
        drop(d);
        assert!(!path.exists());
        let d = TestDir::new("platform-testdir");
        assert!(d.exists() && std::fs::read_dir(&*d).unwrap().next().is_none());
    }

    #[test]
    fn folders_left_by_earlier_runs_are_swept() {
        let root = TestDir::new("platform-sweep");
        let (old, new) = (root.join("old-1"), root.join("new-2"));
        std::fs::create_dir_all(&old).unwrap();
        std::fs::create_dir_all(&new).unwrap();
        let hour_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(7200);
        std::fs::File::open(&old)
            .unwrap()
            .set_modified(hour_ago)
            .unwrap();
        super::sweep(&root, std::time::Duration::from_secs(3600));
        assert!(!old.exists() && new.exists());
    }

    #[test]
    fn it_is_deleted_when_the_test_panics() {
        let path = std::panic::catch_unwind(|| {
            let d = TestDir::new("platform-testdir-panic");
            std::fs::write(d.join("x"), b"1").unwrap();
            let p = d.path().to_path_buf();
            std::panic::panic_any(p);
        })
        .unwrap_err()
        .downcast::<std::path::PathBuf>()
        .unwrap();
        assert!(!path.exists());
    }
}
