//! Background show renders started from the playlist. The player only sees this interface; the
//! app provides the renderer (the visual engine), which keeps `ui` independent of it.

use std::path::PathBuf;
use std::sync::Arc;

use platform::TrackRef;

#[derive(Debug, Clone, PartialEq)]
pub enum JobStatus {
    Running {
        fraction: f64,
        eta_secs: Option<f64>,
    },
    Paused {
        fraction: f64,
    },
    Done {
        out: PathBuf,
    },
    Failed(String),
    Cancelled,
}

impl JobStatus {
    pub fn finished(&self) -> bool {
        matches!(
            self,
            JobStatus::Done { .. } | JobStatus::Failed(_) | JobStatus::Cancelled
        )
    }
}

pub trait RenderJob: Send + Sync {
    fn status(&self) -> JobStatus;
    fn cancel(&self);
    /// Holds the render (e.g. while fullscreen visuals need the GPU).
    fn set_paused(&self, paused: bool);
}

/// What to render: the same options as `winamp-native --render-show`.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderRequest {
    pub track: TrackRef,
    pub out: PathBuf,
    pub size: (u32, u32),
    pub fps: u32,
    /// Seconds of the track.
    pub from: f64,
    pub to: Option<f64>,
    /// The artist/title card at the start.
    pub overlay: bool,
    /// One look (`scene`, `variant`) with the director off.
    pub look: Option<(String, String)>,
}

/// Frame sizes the renderer accepts: the width divisible by 8 and the height even (YUV 4:2:0
/// packing and H.264).
pub fn size_ok(size: (u32, u32)) -> bool {
    size.0 >= 16 && size.1 >= 16 && size.0.is_multiple_of(8) && size.1.is_multiple_of(2)
}

pub trait ShowRenderer {
    /// The looks that can be pinned, as (scene, variant).
    fn looks(&self) -> Vec<(String, String)>;
    /// Starts rendering in the background.
    fn start(&self, request: RenderRequest) -> Arc<dyn RenderJob>;
}

/// Size presets offered in the dialog.
pub const SIZES: [(&str, (u32, u32)); 5] = [
    ("1920×1080 (Full HD)", (1920, 1080)),
    ("1280×720 (HD)", (1280, 720)),
    ("3840×2160 (4K)", (3840, 2160)),
    ("1080×1920 (vertical)", (1080, 1920)),
    ("1080×1080 (square)", (1080, 1080)),
];
pub const FRAME_RATES: [u32; 6] = [24, 25, 30, 50, 60, 120];

/// The "Render show" dialog's state: every command-line option, validated.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderDialog {
    pub track: TrackRef,
    /// Suggested file name (without extension).
    pub name: String,
    pub duration: Option<f64>,
    /// Index into [`SIZES`], or `SIZES.len()` for a custom size.
    pub size_choice: usize,
    pub custom_w: String,
    pub custom_h: String,
    pub fps: u32,
    pub whole_track: bool,
    pub from: String,
    pub to: String,
    pub overlay: bool,
    /// Index into the renderer's looks, or `None` for the director.
    pub look: Option<usize>,
    pub error: Option<String>,
}

/// What the dialog wants after a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogOutcome {
    Open,
    Cancel,
    Render,
}

impl RenderDialog {
    pub fn new(
        track: TrackRef,
        name: String,
        duration: Option<f64>,
        size: (u32, u32),
        fps: u32,
        overlay: bool,
    ) -> Self {
        let size_choice = SIZES
            .iter()
            .position(|(_, s)| *s == size)
            .unwrap_or(SIZES.len());
        Self {
            track,
            name,
            duration,
            size_choice,
            custom_w: size.0.to_string(),
            custom_h: size.1.to_string(),
            fps,
            whole_track: true,
            from: "0:00".into(),
            to: duration.map(crate::format::clock).unwrap_or_default(),
            overlay,
            look: None,
            error: None,
        }
    }

    pub fn size(&self) -> Result<(u32, u32), String> {
        if let Some((_, s)) = SIZES.get(self.size_choice) {
            return Ok(*s);
        }
        let parse = |t: &str| t.trim().parse::<u32>().ok();
        match (parse(&self.custom_w), parse(&self.custom_h)) {
            (Some(w), Some(h)) if size_ok((w, h)) => Ok((w, h)),
            _ => Err("Custom size: width divisible by 8, even height, at least 16×16".into()),
        }
    }

    /// The time range, validated against the track length when known.
    pub fn range(&self) -> Result<(f64, Option<f64>), String> {
        if self.whole_track {
            return Ok((0.0, None));
        }
        let from = crate::format::parse_clock(&self.from).ok_or("From: a time like 1:00")?;
        let to = crate::format::parse_clock(&self.to).ok_or("To: a time like 1:30")?;
        if to <= from {
            return Err("To must be after From".into());
        }
        if let Some(d) = self.duration
            && from >= d
        {
            return Err(format!(
                "From is past the end of the track ({})",
                crate::format::clock(d)
            ));
        }
        Ok((from, Some(to)))
    }

    /// The request for `out`, or what is wrong with the options.
    pub fn request(
        &self,
        out: PathBuf,
        looks: &[(String, String)],
    ) -> Result<RenderRequest, String> {
        let size = self.size()?;
        let (from, to) = self.range()?;
        Ok(RenderRequest {
            track: self.track.clone(),
            out,
            size,
            fps: self.fps.clamp(1, 240),
            from,
            to,
            overlay: self.overlay,
            look: self.look.and_then(|i| looks.get(i).cloned()),
        })
    }

    /// Frames the render will have (for the summary line), when the length is known.
    pub fn frames(&self) -> Option<u64> {
        let (from, to) = self.range().ok()?;
        let end = to.or(self.duration)?.min(self.duration.unwrap_or(f64::MAX));
        Some(((end - from).max(0.0) * self.fps as f64) as u64)
    }

    /// Draws the dialog's contents.
    pub fn ui(&mut self, ui: &mut egui::Ui, looks: &[(String, String)]) -> DialogOutcome {
        let mut outcome = DialogOutcome::Open;
        ui.heading("Render show");
        ui.label(egui::RichText::new(&self.name).weak());
        ui.add_space(6.0);
        egui::Grid::new("render-options")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label("Size");
                ui.horizontal(|ui| {
                    let label = SIZES.get(self.size_choice).map_or("Custom", |(l, _)| *l);
                    egui::ComboBox::from_id_salt("render-size")
                        .selected_text(label)
                        .show_ui(ui, |ui| {
                            for (i, (l, _)) in SIZES.iter().enumerate() {
                                ui.selectable_value(&mut self.size_choice, i, *l);
                            }
                            ui.selectable_value(&mut self.size_choice, SIZES.len(), "Custom");
                        });
                    if self.size_choice == SIZES.len() {
                        ui.add(egui::TextEdit::singleline(&mut self.custom_w).desired_width(48.0));
                        ui.label("×");
                        ui.add(egui::TextEdit::singleline(&mut self.custom_h).desired_width(48.0));
                    }
                });
                ui.end_row();

                ui.label("Frame rate");
                egui::ComboBox::from_id_salt("render-fps")
                    .selected_text(format!("{} fps", self.fps))
                    .show_ui(ui, |ui| {
                        for f in FRAME_RATES {
                            ui.selectable_value(&mut self.fps, f, format!("{f} fps"));
                        }
                    });
                ui.end_row();

                ui.label("Range");
                ui.vertical(|ui| {
                    ui.checkbox(&mut self.whole_track, "Whole track");
                    if !self.whole_track {
                        ui.horizontal(|ui| {
                            ui.label("From");
                            ui.add(egui::TextEdit::singleline(&mut self.from).desired_width(56.0));
                            ui.label("To");
                            ui.add(egui::TextEdit::singleline(&mut self.to).desired_width(56.0));
                        });
                    }
                });
                ui.end_row();

                ui.label("Title card");
                ui.checkbox(&mut self.overlay, "Show artist and title at the start");
                ui.end_row();

                ui.label("Look");
                let current = self
                    .look
                    .and_then(|i| looks.get(i))
                    .map_or("Automatic (director)".to_string(), |(s, v)| {
                        format!("{s} / {v}")
                    });
                egui::ComboBox::from_id_salt("render-look")
                    .selected_text(current)
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.look, None, "Automatic (director)");
                        for (i, (s, v)) in looks.iter().enumerate() {
                            ui.selectable_value(&mut self.look, Some(i), format!("{s} / {v}"));
                        }
                    });
                ui.end_row();
            });
        ui.add_space(6.0);
        let problem = self.size().err().or_else(|| self.range().err());
        match (&problem, self.frames()) {
            (Some(p), _) => {
                ui.colored_label(egui::Color32::from_rgb(230, 90, 80), p);
            }
            (None, Some(n)) => {
                ui.label(egui::RichText::new(format!("{n} frames")).weak());
            }
            (None, None) => {}
        }
        if let Some(e) = &self.error {
            ui.colored_label(egui::Color32::from_rgb(230, 90, 80), e);
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(problem.is_none(), egui::Button::new("Render…"))
                .clicked()
            {
                outcome = DialogOutcome::Render;
            }
            if ui.button("Cancel").clicked() {
                outcome = DialogOutcome::Cancel;
            }
        });
        outcome
    }
}

/// One status line for the main window.
pub fn status_line(name: &str, s: &JobStatus) -> String {
    match s {
        JobStatus::Running { fraction, eta_secs } => {
            let eta = eta_secs.map_or_else(|| "…".into(), crate::format::clock);
            format!("Rendering show {:.0}% · {eta} left", fraction * 100.0)
        }
        JobStatus::Paused { fraction } => format!(
            "Show render paused at {:.0}% (fullscreen)",
            fraction * 100.0
        ),
        JobStatus::Done { .. } => format!("Show saved: {name}"),
        JobStatus::Failed(e) => format!("Show render failed: {e}"),
        JobStatus::Cancelled => "Show render cancelled".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dialog() -> RenderDialog {
        RenderDialog::new(
            TrackRef::new("t.flac"),
            "Artist - Title".into(),
            Some(200.0),
            (1280, 720),
            30,
            true,
        )
    }

    #[test]
    fn dialog_starts_from_the_remembered_options() {
        let d = dialog();
        assert_eq!(
            (d.size_choice, d.fps, d.overlay, d.whole_track),
            (1, 30, true, true)
        );
        assert_eq!(d.to, "3:20");
        let custom =
            RenderDialog::new(TrackRef::new("t"), "n".into(), None, (1000, 600), 60, false);
        assert_eq!(
            (custom.size_choice, custom.size()),
            (SIZES.len(), Ok((1000, 600)))
        );
    }

    #[test]
    fn dialog_builds_the_same_request_as_the_command_line() {
        let looks = vec![
            ("flame".to_string(), "silk".to_string()),
            ("julia_tunnel".to_string(), "solar".to_string()),
        ];
        let mut d = dialog();
        d.whole_track = false;
        d.from = "1:00".into();
        d.to = "1:30".into();
        d.look = Some(1);
        let r = d.request("out.mp4".into(), &looks).unwrap();
        assert_eq!(r.size, (1280, 720));
        assert_eq!(
            (r.fps, r.from, r.to, r.overlay),
            (30, 60.0, Some(90.0), true)
        );
        assert_eq!(r.look, Some(("julia_tunnel".into(), "solar".into())));
        assert_eq!(d.frames(), Some(900));
    }

    #[test]
    fn dialog_rejects_bad_options() {
        let mut d = dialog();
        d.size_choice = SIZES.len();
        d.custom_w = "1284".into();
        assert!(
            d.request("o".into(), &[])
                .unwrap_err()
                .contains("divisible by 8")
        );
        let mut d = dialog();
        d.whole_track = false;
        d.from = "2:00".into();
        d.to = "1:00".into();
        assert!(d.request("o".into(), &[]).unwrap_err().contains("after"));
        d.from = "4:00".into();
        d.to = "5:00".into();
        assert!(
            d.request("o".into(), &[])
                .unwrap_err()
                .contains("past the end")
        );
        d.from = "abc".into();
        assert!(d.request("o".into(), &[]).is_err());
        assert!(size_ok((1080, 1920)) && !size_ok((1082, 1920)) && !size_ok((8, 8)));
    }

    #[test]
    fn dialog_draws_headless() {
        let ctx = egui::Context::default();
        let mut d = dialog();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            assert_eq!(
                d.ui(ui, &[("flame".into(), "silk".into())]),
                DialogOutcome::Open
            );
        });
        out.textures_delta.clear();
    }

    #[test]
    fn status_lines() {
        assert_eq!(
            status_line(
                "a.mp4",
                &JobStatus::Running {
                    fraction: 0.42,
                    eta_secs: Some(70.0)
                }
            ),
            "Rendering show 42% · 1:10 left"
        );
        assert_eq!(
            status_line(
                "a.mp4",
                &JobStatus::Done {
                    out: "a.mp4".into()
                }
            ),
            "Show saved: a.mp4"
        );
        assert!(JobStatus::Cancelled.finished() && !JobStatus::Paused { fraction: 0.1 }.finished());
    }
}
