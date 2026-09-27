//! Opens the spectrogram window's contents for one file, without the player or an audio device:
//! the overview is built up front, clicks move the playhead, and zooming fetches detail.
//! Handy for checking the quality verdict on a real FLAC and a transcoded one.
//!
//! cargo run -p ui --example spectrogram_probe --release -- FILE [seconds-to-stay-open]

use std::sync::Arc;
use std::time::{Duration, Instant};

use analysis::detail::DetailService;
use analysis::overview::OverviewBuilder;
use audio::decode::TrackDecoder;
use platform::native::{NativeFileSource, NativeSpawner};
use platform::{FileSource, TrackRef};
use ui::spectrogram::{Action, Feed, SpectroSettings, SpectrogramWindow};

struct Probe {
    win: SpectrogramWindow,
    feed: Feed,
    until: Option<Instant>,
}

impl eframe::App for Probe {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        for a in self.win.take_actions() {
            if let Action::Seek(t) = a {
                self.feed.now = t;
            }
        }
        self.win.feed(self.feed.clone());
        self.win.ui(ui);
        if self.until.is_some_and(|t| Instant::now() >= t) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

fn main() -> eframe::Result {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: spectrogram_probe FILE [seconds]");
    let until = args
        .next()
        .and_then(|s| s.parse().ok())
        .map(|s| Instant::now() + Duration::from_secs(s));
    let track = TrackRef::new(path);
    let files: Arc<dyn FileSource> = Arc::new(NativeFileSource);
    let mut dec = TrackDecoder::open(&*files, &track).expect("playable file");
    let info = dec.info().clone();
    let mut b = OverviewBuilder::new(dec.src_rate());
    let mut buf = Vec::new();
    loop {
        buf.clear();
        let more = dec.decode_next(&mut buf).unwrap_or(false);
        b.push(&buf);
        if !more {
            break;
        }
    }
    let overview = Arc::new(b.finish());
    if let Some(c) = overview.spectral.as_ref().and_then(|s| s.cutoff.cutoff()) {
        println!(
            "{} (edge drop {:.0} dB)",
            c.describe(info.lossless),
            c.drop_db
        );
    }
    let detail = Arc::new(DetailService::new(Arc::new(NativeSpawner), files));
    let settings = SpectroSettings::default();
    let size = settings.size;
    let feed = Feed {
        title: format!("{} – {}", info.artist, info.title),
        track: Some(track),
        duration: Some(overview.seconds()),
        overview: Some(overview),
        now: 0.0,
        playing: false,
        fullscreen: false,
        lossless: info.lossless,
    };
    let probe = Probe {
        win: SpectrogramWindow::new(settings, Some(detail)),
        feed,
        until,
    };
    eframe::run_native(
        "Spectrogram probe",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([size.0, size.1]),
            ..Default::default()
        },
        Box::new(|_| Ok(Box::new(probe))),
    )
}
