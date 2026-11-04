//! The setup panel (`welcome-setup`): the steps that open up the rest of the app (the Discogs
//! account, the browser extension and, on Windows, previews). It opens at launch while a step
//! is open, until each is done or declined (or "Don't show at startup"), and on demand from
//! OPT ▸ Set up Diggr…. Whether a step is done is read from the app; only the user's "no" is
//! kept (`DigSettings::welcome`), so nobody is asked again about what they said no to.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use super::DiggrApp;
use super::digging::{BridgeSetup, Dig, YtDlpState};

/// Diggr Bridge on the Chrome Web Store.
pub(super) const STORE_URL: &str =
    "https://chromewebstore.google.com/detail/diggr-bridge/cineoflmmlmgdkefaegeocjgelnijfkk";
/// The extension's own page, where the pairing code is entered (the Web Store copy's ID).
pub(super) const OPTIONS_URL: &str =
    "chrome-extension://cineoflmmlmgdkefaegeocjgelnijfkk/options.html";
/// What connecting Discogs adds, explained on the site.
pub(super) const DISCOGS_WHY: &str = "https://diggr.es/discogs-player/";
/// Where a Discogs account is made.
const DISCOGS_SIGNUP: &str = "https://www.discogs.com/";
/// The longest the welcome waits at launch for the yt-dlp lookup before opening without it.
const LOOKUP_WAIT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Step {
    Previews,
    Discogs,
    Browser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum State {
    Open,
    Done,
    Declined,
}

/// The panel in this session.
#[derive(Debug, Default)]
pub(super) struct Welcome {
    /// The panel is showing.
    pub(super) open: bool,
    /// It opened by itself at launch: only open steps are listed.
    at_launch: bool,
    /// Whether to open at launch was decided (once a session).
    decided: bool,
    /// When the launch check began.
    since: Option<Instant>,
    /// Steps put off with Later this session (the launch panel leaves them out).
    later: HashSet<Step>,
    /// Get the extension was clicked: the step shows the pairing code.
    extension_asked: bool,
    /// A nudge was given this session (at most one).
    nudged: bool,
    /// Why Open the extension's page didn't work, shown in the step.
    page_error: Option<String>,
    /// Connect… went to Options ▸ Discogs…: the panel comes back when that closes, with the
    /// steps still open.
    resume: bool,
}

/// More records in a send than the anonymous rate allows a minute: connecting would speed it up.
const SLOW_SEND: usize = 25;

/// A step's state from what's done and what was declined.
fn state(done: bool, declined: bool) -> State {
    if done {
        State::Done
    } else if declined {
        State::Declined
    } else {
        State::Open
    }
}

impl Dig {
    /// A step's state, or `None` for a step this setup doesn't have (Previews outside the
    /// guided setup, the browser without the bridge) or can't tell yet (yt-dlp being looked for).
    pub(super) fn step(&self, s: Step) -> Option<State> {
        match s {
            Step::Previews => {
                if !self.setup.guided_ytdlp || self.setup.tools.is_none() {
                    return None;
                }
                match self.ytdlp {
                    YtDlpState::Unknown => None,
                    YtDlpState::Missing => Some(State::Open),
                    YtDlpState::Found { .. } => Some(State::Done),
                }
            }
            Step::Discogs => Some(state(
                self.token.is_some(),
                self.settings.welcome.no_discogs,
            )),
            Step::Browser => {
                if self.setup.bridge == BridgeSetup::Off {
                    return None;
                }
                let paired = self
                    .bridge_shared
                    .as_ref()
                    .is_some_and(|s| s.pairing().paired() > 0);
                Some(state(paired, self.settings.welcome.no_browser))
            }
        }
    }

    /// The steps this setup has, in order, with their states.
    pub(super) fn steps(&self) -> Vec<(Step, State)> {
        [Step::Previews, Step::Discogs, Step::Browser]
            .into_iter()
            .filter_map(|s| self.step(s).map(|st| (s, st)))
            .collect()
    }

    /// The user said they have no Discogs account (with the setup panel on).
    pub(super) fn no_discogs(&self) -> bool {
        self.setup.welcome && self.settings.welcome.no_discogs
    }

    /// Opens the panel by hand, every step listed.
    pub(super) fn open_welcome(&mut self) {
        self.welcome.open = true;
        self.welcome.at_launch = false;
    }
}

/// What a click in the panel asks for, done once the panel is drawn.
enum Act {
    Install,
    Connect,
    NoDiscogs(bool),
    SignUp,
    WhyDiscogs,
    GetExtension,
    OpenExtensionPage,
    NoBrowser(bool),
    Later(Step),
    AtLaunch(bool),
    Close,
}

impl DiggrApp {
    /// OPT ▸ Set up Diggr…, with how many steps are set up while one is still open
    /// ("Set up Diggr… (1 of 3)").
    pub(super) fn setup_label(&self) -> String {
        let steps = self.dig.as_ref().map(Dig::steps).unwrap_or_default();
        let done = steps.iter().filter(|(_, s)| *s == State::Done).count();
        if steps.iter().any(|(_, s)| *s == State::Open) {
            format!("Set up Diggr… ({done} of {})", steps.len())
        } else {
            "Set up Diggr…".into()
        }
    }

    /// A message about a page only the extension can send: with the Browser step open, it
    /// also says where to get the extension (once a session, with any other nudge).
    pub(super) fn nudge_extension(&mut self, text: String) {
        let nudge = self.dig.as_mut().is_some_and(|d| {
            let open = d.setup.welcome && d.step(Step::Browser) == Some(State::Open);
            open && !std::mem::replace(&mut d.welcome.nudged, true)
        });
        if nudge {
            self.notify(format!("{text}: get it in OPT › Set up Diggr…"));
        } else {
            self.notify(text);
        }
    }

    /// A send of `total` records without an account, slowed by the anonymous rate: once a
    /// session, says connecting would speed it up.
    pub(super) fn nudge_slow_send(&mut self, total: usize) {
        let nudge = self.dig.as_mut().is_some_and(|d| {
            let open = d.setup.welcome && d.step(Step::Discogs) == Some(State::Open);
            open && total > SLOW_SEND && !std::mem::replace(&mut d.welcome.nudged, true)
        });
        if nudge {
            self.notify("Sends are slower without an account: connecting Discogs doubles the speed (OPT › Set up Diggr…)");
        }
    }

    /// An action needs the Discogs account and there is none: the setup panel opens, or, when
    /// the user said they have no account (or there is no panel), the main window says so.
    pub(super) fn dig_needs_account(&mut self) {
        let Some(d) = &mut self.dig else { return };
        if d.setup.welcome && !d.settings.welcome.no_discogs {
            d.open_welcome();
        } else {
            self.notify("This needs a Discogs account (OPT › Discogs…)");
        }
    }

    /// At launch (after the first frame), opens the panel once if a step is open. With the
    /// guided setup it waits for the yt-dlp lookup, at most [`LOOKUP_WAIT`], so Previews can be
    /// part of it.
    fn welcome_at_launch(&mut self) {
        if !self.first_frame_done {
            return;
        }
        let Some(d) = &mut self.dig else { return };
        if !d.setup.welcome || d.welcome.decided {
            return;
        }
        let since = *d.welcome.since.get_or_insert_with(Instant::now);
        let looking =
            d.setup.guided_ytdlp && d.setup.tools.is_some() && d.ytdlp == YtDlpState::Unknown;
        if looking && since.elapsed() < LOOKUP_WAIT {
            return;
        }
        d.welcome.decided = true;
        let open = d.steps().iter().any(|(_, s)| *s == State::Open);
        if open && d.settings.welcome.at_launch {
            d.welcome.open = true;
            d.welcome.at_launch = true;
        }
    }

    pub(super) fn welcome_ui(&mut self, ctx: &egui::Context) {
        self.welcome_at_launch();
        self.welcome_resume();
        let Some(d) = &self.dig else { return };
        if !d.welcome.open {
            return;
        }
        let at_launch = d.welcome.at_launch;
        let steps: Vec<(Step, State)> = d
            .steps()
            .into_iter()
            .filter(|(s, st)| !at_launch || (*st == State::Open && !d.welcome.later.contains(s)))
            .collect();
        // Everything at launch got answered: nothing left to show.
        if at_launch && steps.is_empty() {
            self.close_welcome();
            return;
        }
        let installing = d.ytdlp_download;
        let error = d.ytdlp_error.clone();
        let extension_asked = d.welcome.extension_asked;
        let page_error = d.welcome.page_error.clone();
        let show_at_launch = d.settings.welcome.at_launch;
        let shared = d.bridge_shared.clone();
        let mut acts = Vec::new();
        egui::Modal::new(egui::Id::new("welcome")).show(ctx, |ui| {
            ui.set_max_width(440.0);
            ui.heading("Get the most out of Diggr");
            for (step, st) in &steps {
                ui.separator();
                match step {
                    Step::Previews => {
                        ui.strong("Previews");
                        match st {
                            State::Done => {
                                ui.label("yt-dlp is installed: previews play.");
                            }
                            _ => {
                                ui.label(format!(
                                    "Previews from Discogs, Bandcamp and the other shops need \
                                     yt-dlp, a free downloader ({} MB, with a Python and a \
                                     JavaScript runtime that only Diggr uses). Your own files \
                                     play without it.",
                                    dig::tools::SIZE_MB
                                ));
                                if let Some(p) = installing {
                                    ui.label(format!("Installing yt-dlp: {p}%"));
                                } else {
                                    if let Some(why) = &error {
                                        ui.label(format!("It couldn't be installed: {why}"));
                                    }
                                    ui.horizontal(|ui| {
                                        let label = if error.is_some() {
                                            "Try again"
                                        } else {
                                            "Install"
                                        };
                                        if ui.button(label).clicked() {
                                            acts.push(Act::Install);
                                        }
                                        if at_launch && ui.button("Later").clicked() {
                                            acts.push(Act::Later(*step));
                                        }
                                    });
                                }
                            }
                        }
                    }
                    Step::Discogs => {
                        ui.strong("Your Discogs account");
                        match st {
                            State::Done => {
                                ui.label("Connected: your wantlist and collection stay in sync.");
                            }
                            State::Declined => {
                                ui.label("– No Discogs account.");
                                ui.horizontal(|ui| {
                                    if ui.button("Connect after all…").clicked() {
                                        acts.push(Act::NoDiscogs(false));
                                        acts.push(Act::Connect);
                                    }
                                    if ui.link("Create a free account on discogs.com").clicked() {
                                        acts.push(Act::SignUp);
                                    }
                                });
                            }
                            State::Open => {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        "Your wantlist and collection kept in sync with Discogs, \
                                         the Discogs cart, and sends twice as fast.",
                                    );
                                    if ui.link("Why connect?").clicked() {
                                        acts.push(Act::WhyDiscogs);
                                    }
                                });
                                ui.horizontal(|ui| {
                                    if ui.button("Connect…").clicked() {
                                        acts.push(Act::Connect);
                                    }
                                    if ui.button("I don't have an account").clicked() {
                                        acts.push(Act::NoDiscogs(true));
                                    }
                                    if at_launch && ui.button("Later").clicked() {
                                        acts.push(Act::Later(*step));
                                    }
                                });
                            }
                        }
                    }
                    Step::Browser => {
                        ui.strong("The browser extension");
                        match st {
                            State::Done => {
                                ui.label("A browser is paired: send pages to Diggr from it.");
                            }
                            State::Declined => {
                                ui.label("– No extension.");
                                if ui.button("Get it after all").clicked() {
                                    acts.push(Act::NoBrowser(false));
                                    acts.push(Act::GetExtension);
                                }
                            }
                            State::Open => {
                                ui.label(
                                    "Send any Discogs, Bandcamp, Juno, Beatport, Traxsource or \
                                     Bleep page to Diggr with one click (Chrome, Edge, Brave and \
                                     other Chromium browsers).",
                                );
                                ui.horizontal(|ui| {
                                    if ui.button("Get the extension").clicked() {
                                        acts.push(Act::GetExtension);
                                    }
                                    if ui.button("I don't use Chrome").clicked() {
                                        acts.push(Act::NoBrowser(true));
                                    }
                                    if at_launch && ui.button("Later").clicked() {
                                        acts.push(Act::Later(*step));
                                    }
                                });
                                if extension_asked && let Some(shared) = &shared {
                                    let mut pairing = shared.pairing();
                                    pairing_code(
                                        ui,
                                        &mut pairing,
                                        page_error.as_deref(),
                                        &mut acts,
                                    );
                                }
                            }
                        }
                    }
                }
            }
            ui.separator();
            ui.horizontal(|ui| {
                let mut show = show_at_launch;
                if ui.checkbox(&mut show, "Show at startup").changed() {
                    acts.push(Act::AtLaunch(show));
                }
                if ui.button("Close").clicked() {
                    acts.push(Act::Close);
                }
            });
        });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            acts.push(Act::Close);
        }
        if extension_asked {
            // The code's countdown.
            ctx.request_repaint_after(Duration::from_secs(1));
        }
        for a in acts {
            self.welcome_act(a);
        }
    }

    /// Back from Options ▸ Discogs… (opened by Connect…): the panel opens again while a step
    /// is still open, listed as at launch.
    fn welcome_resume(&mut self) {
        let Some(d) = &mut self.dig else { return };
        if !d.welcome.resume || d.dialog.is_some() {
            return;
        }
        d.welcome.resume = false;
        if d.steps().iter().any(|(_, s)| *s == State::Open) {
            d.welcome.open = true;
            d.welcome.at_launch = true;
        }
    }

    fn welcome_act(&mut self, a: Act) {
        let Some(d) = &mut self.dig else { return };
        let mut save = false;
        match a {
            Act::Install => {
                d.ytdlp_error = None;
                d.download_ytdlp();
            }
            Act::Connect => {
                let back = d.welcome.open;
                self.close_welcome();
                if let Some(d) = &mut self.dig {
                    d.welcome.resume = back;
                }
                self.dig_open_dialog();
                if let Some(dialog) = self.dig.as_mut().and_then(|d| d.dialog.as_mut()) {
                    dialog.focus_token = 3;
                }
                return;
            }
            Act::NoDiscogs(no) => {
                d.settings.welcome.no_discogs = no;
                save = true;
            }
            Act::WhyDiscogs => {
                let err = d.setup.browser.open(DISCOGS_WHY).err();
                self.dig_notify(err.map(|e| format!("Could not open the browser: {e}")));
                return;
            }
            Act::SignUp => {
                let err = d.setup.browser.open(DISCOGS_SIGNUP).err();
                self.dig_notify(err.map(|e| format!("Could not open the browser: {e}")));
                return;
            }
            Act::GetExtension => {
                d.welcome.extension_asked = true;
                let err = d.setup.browser.open(STORE_URL).err();
                self.dig_notify(err.map(|e| format!("Could not open the browser: {e}")));
                return;
            }
            Act::OpenExtensionPage => {
                d.welcome.page_error = d.setup.browser.open_extension_page(OPTIONS_URL).err();
                return;
            }
            Act::NoBrowser(no) => {
                d.settings.welcome.no_browser = no;
                save = true;
            }
            Act::Later(s) => {
                d.welcome.later.insert(s);
            }
            Act::AtLaunch(on) => {
                d.settings.welcome.at_launch = on;
                save = true;
            }
            Act::Close => {
                self.close_welcome();
                return;
            }
        }
        if save {
            let e = d.save_settings();
            self.dig_notify(e);
        }
    }

    fn close_welcome(&mut self) {
        if let Some(d) = &mut self.dig {
            d.close_welcome();
        }
    }
}

impl Dig {
    fn close_welcome(&mut self) {
        self.welcome.open = false;
        // No pairing code works while nothing shows it (unless the Browser dialog does).
        if self.welcome.extension_asked
            && self.bridge_dialog.is_none()
            && let Some(shared) = &self.bridge_shared
        {
            shared.pairing().clear_code();
        }
        self.welcome.extension_asked = false;
    }
}

/// The pairing code, and where to enter it: Open the extension's page starts the default
/// browser on it; the address, to paste by hand, stays for when that can't work (another
/// browser, a copy loaded unpacked).
fn pairing_code(
    ui: &mut egui::Ui,
    pairing: &mut dig::bridge::Pairing,
    page_error: Option<&str>,
    acts: &mut Vec<Act>,
) {
    ui.label("Then pair it with this code:");
    if let Some(wait) = pairing.locked_for() {
        ui.label(format!(
            "Too many wrong codes: pairing resumes in {} s",
            wait.as_secs_f32().ceil()
        ));
        return;
    }
    match pairing.ensure_code() {
        Ok((code, left)) => {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("{} {}", &code[..3], &code[3..]))
                        .monospace()
                        .size(24.0),
                );
                let secs = left.as_secs_f32().ceil() as u64;
                ui.label(format!("valid for {}:{:02}", secs / 60, secs % 60));
            });
            ui.horizontal(|ui| {
                if ui.button("Open the extension's page").clicked() {
                    acts.push(Act::OpenExtensionPage);
                }
                if ui.button("Copy code").clicked() {
                    ui.ctx().copy_text(code.clone());
                }
            });
            ui.label("Paste the code there and click Pair.");
        }
        Err(e) => {
            ui.colored_label(egui::Color32::from_rgb(230, 90, 90), e);
        }
    }
    if let Some(e) = page_error {
        ui.label(format!("The page didn't open: {e}."));
    }
    ui.label("Or paste this address into your browser's address bar:");
    ui.horizontal(|ui| {
        ui.add(egui::Label::new(egui::RichText::new(OPTIONS_URL).monospace()).selectable(true));
        if ui.button("Copy").clicked() {
            ui.ctx().copy_text(OPTIONS_URL.to_owned());
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn done_wins_over_declined() {
        assert_eq!(state(true, true), State::Done);
        assert_eq!(state(false, true), State::Declined);
        assert_eq!(state(false, false), State::Open);
    }
}
