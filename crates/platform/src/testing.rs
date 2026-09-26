//! A sink driven by the test itself, so engine behavior can be verified without a device.

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
