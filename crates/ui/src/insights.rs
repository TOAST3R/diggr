//! Insights: what the collection says about its owner, counted over its vinyl records.
//!
//! [`Stats::of`] reads the collection crate once (records by album key, a record's values
//! the way the playlist's filters read them, so a click on a bar picks exactly what it
//! counted) and the window draws only from that. Nothing here asks Discogs: label totals come
//! from the intake.

use std::collections::{BTreeMap, HashMap};

use crate::playlist::{AlbumKey, Entry, EntryId, Facet, Playlist};

/// Styles in the heatmap.
pub const HEAT_STYLES: usize = 8;
/// Labels and artists listed.
pub const TOP: usize = 10;
/// Dearest records listed.
pub const DEAREST: usize = 5;
/// Artist credits that say nothing about taste.
const NO_ARTIST: [&str; 2] = ["Various", "Unknown Artist"];

/// The collection's numbers, for its vinyl records.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stats {
    /// Vinyl records counted.
    pub records: usize,
    /// Records per release year (records with no year left out).
    pub years: BTreeMap<u16, usize>,
    /// Records per style, most first, then by name.
    pub styles: Vec<(String, usize)>,
    pub heat: Heat,
    /// The labels with the most records, with one release of theirs (to find the label).
    pub labels: Vec<TopLabel>,
    /// The artists with the most records.
    pub artists: Vec<(String, usize)>,
    pub value: Value,
    /// Analysed tracks per 10-BPM bucket (its lower bound), in order.
    pub bpm: Vec<(u16, usize)>,
    /// Tracks with a tempo.
    pub analysed: usize,
}

/// Records of the largest styles by decade.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Heat {
    pub styles: Vec<String>,
    /// First years of the decades, in order (1990 for the 90s).
    pub decades: Vec<u16>,
    /// `cells[style][decade]`.
    pub cells: Vec<Vec<usize>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TopLabel {
    /// As the label filter reads it.
    pub name: String,
    pub records: usize,
    /// A release of it in the collection.
    pub release: u64,
}

/// What the records sell for, at the lowest price on Discogs when they were last looked up.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Value {
    /// The currency most records are priced in; the sum and median are in it.
    pub currency: String,
    pub sum_cents: u64,
    /// Records priced in `currency`.
    pub priced: usize,
    /// Records priced in another currency (left out of the sum).
    pub other: usize,
    pub median_cents: u64,
    /// The dearest records: a track of each, its name, its price.
    pub dearest: Vec<(EntryId, String, u64)>,
    /// Records with at most one copy for sale.
    pub rare: usize,
    /// The oldest look at a price (seconds since the Unix epoch).
    pub oldest: Option<u64>,
}

/// A record: its first entry with record data, and its tracks' tempos.
struct Record<'a> {
    first: &'a Entry,
    bpm: Vec<u16>,
}

impl Stats {
    /// The numbers of `p`'s vinyl records: a record counts once, however many tracks it has,
    /// when "Vinyl" is among its formats.
    pub fn of(p: &Playlist) -> Stats {
        let mut slot: HashMap<AlbumKey, usize> = HashMap::new();
        let mut all: Vec<Record> = Vec::new();
        // A record's tracks are next to each other: only a new key is looked up.
        let mut last: Option<(AlbumKey, usize)> = None;
        for e in p.entries() {
            let Some(key) = e.album_key() else { continue };
            let i = match &last {
                Some((k, i)) if *k == key => *i,
                _ => {
                    let i = *slot.entry(key.clone()).or_insert_with(|| {
                        all.push(Record {
                            first: e,
                            bpm: Vec::new(),
                        });
                        all.len() - 1
                    });
                    last = Some((key, i));
                    i
                }
            };
            if all[i].first.origin.is_none() && e.origin.is_some() {
                all[i].first = e;
            }
            if let Some(b) = e.bpm {
                all[i].bpm.push(b);
            }
        }
        let records: Vec<Record> = all
            .into_iter()
            .filter(|r| r.first.values(Facet::Format).any(|f| f == "Vinyl"))
            .collect();

        let mut years: BTreeMap<u16, usize> = BTreeMap::new();
        let mut styles: HashMap<&str, usize> = HashMap::new();
        let mut labels: HashMap<&str, (usize, u64)> = HashMap::new();
        let mut artists: HashMap<&str, usize> = HashMap::new();
        let mut buckets: BTreeMap<u16, usize> = BTreeMap::new();
        let mut analysed = 0;
        for r in &records {
            let e = r.first;
            let o = e.origin.as_ref();
            if let Some(y) = o.and_then(|o| o.year) {
                *years.entry(y).or_default() += 1;
            }
            for s in e.values(Facet::Style) {
                *styles.entry(s).or_default() += 1;
            }
            if let Some(l) = e.values(Facet::Label).next() {
                let release = o.and_then(|o| o.release).unwrap_or(0);
                let slot = labels.entry(l).or_insert((0, release));
                slot.0 += 1;
            }
            if let Some(a) = e.values(Facet::Artist).next()
                && !NO_ARTIST.contains(&a)
            {
                *artists.entry(a).or_default() += 1;
            }
            for &b in &r.bpm {
                *buckets.entry(b / 10 * 10).or_default() += 1;
                analysed += 1;
            }
        }
        let styles = most_first(styles);
        let mut labels: Vec<TopLabel> = labels
            .into_iter()
            .map(|(name, (records, release))| TopLabel {
                name: name.to_owned(),
                records,
                release,
            })
            .collect();
        labels.sort_by(|a, b| b.records.cmp(&a.records).then_with(|| a.name.cmp(&b.name)));
        labels.truncate(TOP);
        let mut artists = most_first(artists);
        artists.truncate(TOP);
        let heat = heat(&records, &styles, &years);
        Stats {
            records: records.len(),
            years,
            styles,
            heat,
            labels,
            artists,
            value: value(&records),
            bpm: buckets.into_iter().collect(),
            analysed,
        }
    }
}

/// Counts as `(name, n)`, most first, then by name.
fn most_first(counts: HashMap<&str, usize>) -> Vec<(String, usize)> {
    let mut v: Vec<(String, usize)> = counts.into_iter().map(|(k, n)| (k.to_owned(), n)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v
}

fn heat(records: &[Record], styles: &[(String, usize)], years: &BTreeMap<u16, usize>) -> Heat {
    let (Some(&first), Some(&last)) = (years.keys().next(), years.keys().next_back()) else {
        return Heat::default();
    };
    let decades: Vec<u16> = (first / 10 * 10..=last / 10 * 10).step_by(10).collect();
    let styles: Vec<String> = styles
        .iter()
        .take(HEAT_STYLES)
        .map(|(s, _)| s.clone())
        .collect();
    let mut cells = vec![vec![0; decades.len()]; styles.len()];
    for r in records {
        let Some(y) = r.first.origin.as_ref().and_then(|o| o.year) else {
            continue;
        };
        let d = usize::from((y / 10 * 10 - decades[0]) / 10);
        for s in r.first.values(Facet::Style) {
            if let Some(i) = styles.iter().position(|x| x == s) {
                cells[i][d] += 1;
            }
        }
    }
    Heat {
        styles,
        decades,
        cells,
    }
}

fn value(records: &[Record]) -> Value {
    let priced: Vec<(&Record, u64, &str)> = records
        .iter()
        .filter_map(|r| {
            let f = r.first.origin.as_ref()?.for_sale.as_ref()?;
            Some((r, f.lowest_cents?, f.currency.as_str()))
        })
        .collect();
    let mut currencies: HashMap<&str, usize> = HashMap::new();
    for (_, _, c) in &priced {
        *currencies.entry(c).or_default() += 1;
    }
    let currency = currencies
        .into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(a.0)))
        .map(|(c, _)| c.to_owned())
        .unwrap_or_default();
    let mut main: Vec<(&Record, u64)> = priced
        .iter()
        .filter(|(_, _, c)| *c == currency)
        .map(|(r, cents, _)| (*r, *cents))
        .collect();
    main.sort_by_key(|m| std::cmp::Reverse(m.1));
    let median_cents = match main.len() {
        0 => 0,
        n if n % 2 == 1 => main[n / 2].1,
        n => (main[n / 2 - 1].1 + main[n / 2].1) / 2,
    };
    let for_sale = || {
        records
            .iter()
            .filter_map(|r| r.first.origin.as_ref()?.for_sale.as_ref())
    };
    Value {
        sum_cents: main.iter().map(|(_, c)| c).sum(),
        priced: main.len(),
        other: priced.len() - main.len(),
        median_cents,
        dearest: main
            .iter()
            .take(DEAREST)
            .map(|(r, c)| (r.first.id, record_name(r.first), *c))
            .collect(),
        rare: for_sale().filter(|f| f.count <= 1).count(),
        oldest: for_sale().map(|f| f.fetched_at).filter(|&t| t > 0).min(),
        currency,
    }
}

/// "Artist – Title" of a record.
fn record_name(e: &Entry) -> String {
    let o = e.origin.as_ref();
    let artist = o.map_or("", |o| o.artist.as_str());
    let title = o.map_or("", |o| o.album.as_str());
    [artist, title]
        .into_iter()
        .filter(|s| !s.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" – ")
}

/// What the window asks of the app.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Show the collection crate with these filter values only (every other filter cleared).
    Filter(Vec<(Facet, String)>),
    /// Show the collection crate unfiltered, with this record selected and in view.
    ShowRecord(EntryId),
    /// Follow this label (its name as a top label).
    Follow(String),
    Close,
}

/// What a top label's row knows of the label on Discogs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LabelInfo {
    /// Its total is being asked.
    Asking,
    /// Without a token or offline: no total.
    Unknown,
    Total {
        total: usize,
        following: bool,
    },
}

/// The skin's colours, as the window paints them.
#[derive(Debug, Clone, Copy)]
pub struct Look {
    pub bg: egui::Color32,
    pub text: egui::Color32,
    pub bars: egui::Color32,
    pub title: egui::Color32,
    /// How far the bars have grown since the window opened, 0 to 1 (eased).
    pub grow: f32,
}

/// Seconds a hover takes to light up or fade.
const HOVER_FADE: f32 = 0.15;

/// How lit `resp` is: 0 to 1, fading in and out with the pointer.
fn lit(ui: &egui::Ui, resp: &egui::Response) -> f32 {
    ui.ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), HOVER_FADE)
}

impl Look {
    /// The bar colour, toward the title colour as it is lit.
    fn bar(&self, lit: f32) -> egui::Color32 {
        self.bars.lerp_to_gamma(self.title, lit)
    }
    fn dim(&self) -> egui::Color32 {
        self.text.lerp_to_gamma(self.bg, 0.45)
    }
    fn shade(&self, part: f32) -> egui::Color32 {
        self.bg.lerp_to_gamma(self.bars, part.clamp(0.0, 1.0))
    }
}

/// Styles listed in their chart.
pub const STYLES_SHOWN: usize = 15;
const ROW: f32 = 16.0;
const NAME_W: f32 = 132.0;

/// The window's contents: every chart, each bar a click.
pub fn ui(
    ui: &mut egui::Ui,
    s: &Stats,
    label: &dyn Fn(&str) -> LabelInfo,
    look: &Look,
) -> Vec<Action> {
    let mut out = Vec::new();
    egui::Frame::NONE
        .fill(look.bg)
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.style_mut().visuals.override_text_color = Some(look.text);
            ui.label(
                egui::RichText::new(format!(
                    "INSIGHTS · {} vinyl records",
                    crate::format::thousands(s.records)
                ))
                .color(look.title)
                .strong(),
            );
            ui.add_space(4.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                let wide = ui.available_width() >= 640.0;
                let left = |ui: &mut egui::Ui, out: &mut Vec<Action>| {
                    years_chart(ui, s, look, out);
                    styles_chart(ui, s, look, out);
                    heat_chart(ui, s, look, out);
                };
                let right = |ui: &mut egui::Ui, out: &mut Vec<Action>| {
                    labels_chart(ui, s, label, look, out);
                    artists_chart(ui, s, look, out);
                    value_part(ui, s, look, out);
                    tempo_chart(ui, s, look);
                };
                if wide {
                    ui.columns(2, |cols| {
                        left(&mut cols[0], &mut out);
                        right(&mut cols[1], &mut out);
                    });
                } else {
                    left(ui, &mut out);
                    right(ui, &mut out);
                }
            });
        });
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        out.push(Action::Close);
    }
    out
}

fn heading(ui: &mut egui::Ui, text: &str, look: &Look) {
    ui.add_space(8.0);
    ui.label(egui::RichText::new(text).color(look.title).small().strong());
}

fn empty(ui: &mut egui::Ui, text: &str, look: &Look) {
    ui.label(egui::RichText::new(text).color(look.dim()));
}

/// A clickable row: name, a bar for `n` of `max`, then `tail` (the count by default).
fn bar_row(
    ui: &mut egui::Ui,
    name: &str,
    n: usize,
    max: usize,
    tail: &str,
    hover: &str,
    look: &Look,
) -> egui::Response {
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW), egui::Sense::click());
    let p = ui.painter_at(rect);
    let font = egui::FontId::proportional(11.0);
    let lit = lit(ui, &resp);
    if lit > 0.0 {
        p.rect_filled(rect, 2.0, look.shade(0.18 * lit));
    }
    let tail_w = 70.0;
    let bar_w = (rect.width() - NAME_W - tail_w - 8.0).max(10.0);
    p.text(
        rect.left_center() + egui::vec2(2.0 + 2.0 * lit, 0.0),
        egui::Align2::LEFT_CENTER,
        name,
        font.clone(),
        look.text.lerp_to_gamma(look.title, lit),
    );
    let w = bar_w * n as f32 / max.max(1) as f32 * look.grow;
    let bar = egui::Rect::from_min_size(
        egui::pos2(rect.left() + NAME_W, rect.top() + 3.0 - lit),
        egui::vec2(w.max(1.0), ROW - 6.0 + 2.0 * lit),
    );
    p.rect_filled(bar, 1.0, look.bar(lit));
    p.text(
        egui::pos2(rect.right() - 2.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        tail,
        font,
        look.dim().lerp_to_gamma(look.text, lit),
    );
    resp.on_hover_text(hover)
}

fn years_chart(ui: &mut egui::Ui, s: &Stats, look: &Look, out: &mut Vec<Action>) {
    heading(ui, "YEARS", look);
    let (Some(&first), Some(&last)) = (s.years.keys().next(), s.years.keys().next_back()) else {
        return empty(ui, "No years known", look);
    };
    let h = 96.0;
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), h + 16.0),
        egui::Sense::hover(),
    );
    let p = ui.painter_at(rect);
    let (d0, d1) = (first / 10 * 10, last / 10 * 10 + 9);
    let n = f32::from(d1 - d0 + 1);
    let step = rect.width() / n;
    let max = s.years.values().copied().max().unwrap_or(1);
    let font = egui::FontId::proportional(10.0);
    for d in (d0..=d1).step_by(10) {
        let x = rect.left() + f32::from(d - d0) * step;
        let band =
            egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(step * 10.0, h + 16.0));
        let records: usize = s.years.range(d..d + 10).map(|(_, n)| n).sum();
        let resp = ui
            .interact(
                egui::Rect::from_min_max(egui::pos2(band.left(), rect.top() + h), band.max),
                ui.id().with(("decade", d)),
                egui::Sense::click(),
            )
            .on_hover_text(format!(
                "{}: {} records · click to show them",
                decade_name(d),
                crate::format::thousands(records)
            ));
        let band_lit = lit(ui, &resp);
        let base = if (d / 10) % 2 == 1 { 0.07 } else { 0.0 };
        if base + band_lit > 0.0 {
            p.rect_filled(band, 0.0, look.shade(base + 0.13 * band_lit));
        }
        p.text(
            egui::pos2(band.center().x, rect.bottom() - 7.0),
            egui::Align2::CENTER_CENTER,
            decade_name(d),
            font.clone(),
            look.dim(),
        );
        if resp.clicked() {
            out.push(Action::Filter(decade_years(s, d)));
        }
    }
    for (&y, &n) in &s.years {
        let x = rect.left() + f32::from(y - d0) * step;
        let bh = h * n as f32 / max as f32 * look.grow;
        let hit = egui::Rect::from_min_max(
            egui::pos2(x, rect.top()),
            egui::pos2(x + step, rect.top() + h),
        );
        let resp = ui
            .interact(hit, ui.id().with(("year", y)), egui::Sense::click())
            .on_hover_text(format!("{y}: {n} records · click to show them"));
        let lit = lit(ui, &resp);
        let widen = step * 0.25 * lit;
        let bar = egui::Rect::from_min_max(
            egui::pos2(x + 0.5 - widen, rect.top() + h - bh.max(1.0)),
            egui::pos2(x + step - 0.5 + widen, rect.top() + h),
        );
        p.rect_filled(bar, 0.0, look.bar(lit));
        if lit > 0.0 {
            p.text(
                egui::pos2(bar.center().x, bar.top() - 7.0),
                egui::Align2::CENTER_CENTER,
                n.to_string(),
                egui::FontId::proportional(10.0),
                look.title.gamma_multiply(lit),
            );
        }
        if resp.clicked() {
            out.push(Action::Filter(vec![(Facet::Year, y.to_string())]));
        }
    }
}

fn decade_name(d: u16) -> String {
    format!("{:02}s", d % 100)
}

/// The year filter values of decade `d`: its years that have records.
fn decade_years(s: &Stats, d: u16) -> Vec<(Facet, String)> {
    s.years
        .range(d..d + 10)
        .map(|(y, _)| (Facet::Year, y.to_string()))
        .collect()
}

fn styles_chart(ui: &mut egui::Ui, s: &Stats, look: &Look, out: &mut Vec<Action>) {
    heading(ui, "STYLES", look);
    if s.styles.is_empty() {
        return empty(ui, "No styles known", look);
    }
    let max = s.styles[0].1;
    for (name, n) in s.styles.iter().take(STYLES_SHOWN) {
        let hover = format!("{name}: {n} records · click to show them");
        if bar_row(ui, name, *n, max, &n.to_string(), &hover, look).clicked() {
            out.push(Action::Filter(vec![(Facet::Style, name.clone())]));
        }
    }
}

fn heat_chart(ui: &mut egui::Ui, s: &Stats, look: &Look, out: &mut Vec<Action>) {
    heading(ui, "STYLE × DECADE", look);
    let h = &s.heat;
    if h.styles.is_empty() || h.decades.is_empty() {
        return empty(ui, "No styles known", look);
    }
    let max = h.cells.iter().flatten().copied().max().unwrap_or(1).max(1);
    let width = ui.available_width();
    let cell_w = ((width - NAME_W) / h.decades.len() as f32).max(14.0);
    let font = egui::FontId::proportional(10.0);
    let (head, _) = ui.allocate_exact_size(egui::vec2(width, ROW), egui::Sense::hover());
    for (j, &d) in h.decades.iter().enumerate() {
        ui.painter().text(
            egui::pos2(
                head.left() + NAME_W + (j as f32 + 0.5) * cell_w,
                head.center().y,
            ),
            egui::Align2::CENTER_CENTER,
            decade_name(d),
            font.clone(),
            look.dim(),
        );
    }
    for (i, style) in h.styles.iter().enumerate() {
        let (row, _) = ui.allocate_exact_size(egui::vec2(width, ROW), egui::Sense::hover());
        ui.painter().text(
            row.left_center() + egui::vec2(2.0, 0.0),
            egui::Align2::LEFT_CENTER,
            style,
            egui::FontId::proportional(11.0),
            look.text,
        );
        for (j, &d) in h.decades.iter().enumerate() {
            let n = h.cells[i][j];
            let cell = egui::Rect::from_min_size(
                egui::pos2(row.left() + NAME_W + j as f32 * cell_w, row.top()),
                egui::vec2(cell_w, ROW),
            )
            .shrink(1.0);
            let resp = ui
                .interact(cell, ui.id().with(("heat", i, j)), egui::Sense::click())
                .on_hover_text(format!(
                    "{style} · {}: {n} records · click to show them",
                    decade_name(d)
                ));
            let part = if n == 0 {
                0.0
            } else {
                0.15 + 0.85 * n as f32 / max as f32
            };
            let lit = lit(ui, &resp);
            let cell = cell.expand(1.5 * lit);
            ui.painter()
                .rect_filled(cell, 2.0, look.shade(part * look.grow + 0.12 * lit));
            if lit > 0.0 {
                ui.painter().rect_stroke(
                    cell,
                    2.0,
                    (1.0, look.title.gamma_multiply(lit)),
                    egui::StrokeKind::Inside,
                );
                if n > 0 {
                    ui.painter().text(
                        cell.center(),
                        egui::Align2::CENTER_CENTER,
                        n.to_string(),
                        egui::FontId::proportional(10.0),
                        look.text.gamma_multiply(lit),
                    );
                }
            }
            if resp.clicked() && n > 0 {
                let mut picks = decade_years(s, d);
                picks.push((Facet::Style, style.clone()));
                out.push(Action::Filter(picks));
            }
        }
    }
}

fn labels_chart(
    ui: &mut egui::Ui,
    s: &Stats,
    label: &dyn Fn(&str) -> LabelInfo,
    look: &Look,
    out: &mut Vec<Action>,
) {
    heading(ui, "TOP LABELS", look);
    let Some(max) = s.labels.first().map(|l| l.records) else {
        return empty(ui, "No labels known", look);
    };
    for l in &s.labels {
        let info = label(&l.name);
        let tail = match info {
            LabelInfo::Asking => format!("{} of …", l.records),
            LabelInfo::Unknown => format!("{} records", l.records),
            LabelInfo::Total { total, .. } => {
                format!(
                    "{} of {} releases",
                    l.records,
                    crate::format::thousands(total)
                )
            }
        };
        ui.horizontal(|ui| {
            let w = ui.available_width() - 74.0;
            ui.allocate_ui(egui::vec2(w.max(60.0), ROW), |ui| {
                let hover = format!("{}: {tail} · click to show them", l.name);
                if bar_row(ui, &l.name, l.records, max, &tail, &hover, look).clicked() {
                    out.push(Action::Filter(vec![(Facet::Label, l.name.clone())]));
                }
            });
            match info {
                LabelInfo::Total {
                    following: true, ..
                } => {
                    ui.label(egui::RichText::new("Following").color(look.dim()).small());
                }
                LabelInfo::Total { .. }
                    if ui
                        .small_button("Follow")
                        .on_hover_text("Follow this label: a crate of its releases")
                        .clicked() =>
                {
                    out.push(Action::Follow(l.name.clone()));
                }
                _ => {}
            }
        });
    }
}

fn artists_chart(ui: &mut egui::Ui, s: &Stats, look: &Look, out: &mut Vec<Action>) {
    heading(ui, "TOP ARTISTS", look);
    let Some(max) = s.artists.first().map(|a| a.1) else {
        return empty(ui, "No artists known", look);
    };
    for (name, n) in &s.artists {
        let hover = format!("{name}: {n} records · click to show them");
        if bar_row(ui, name, *n, max, &n.to_string(), &hover, look).clicked() {
            out.push(Action::Filter(vec![(Facet::Artist, name.clone())]));
        }
    }
}

fn value_part(ui: &mut egui::Ui, s: &Stats, look: &Look, out: &mut Vec<Action>) {
    heading(ui, "VALUE", look);
    let v = &s.value;
    if v.priced == 0 {
        return empty(ui, "No prices known yet", look);
    }
    let price = |c| crate::format::price(c, &v.currency);
    ui.label(format!(
        "≈ {} for {} priced records · median {}",
        price(v.sum_cents),
        crate::format::thousands(v.priced),
        price(v.median_cents)
    ));
    if v.other > 0 {
        ui.label(
            egui::RichText::new(format!("and {} priced in other currencies", v.other))
                .color(look.dim()),
        );
    }
    for (id, name, cents) in &v.dearest {
        let (rect, resp) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW), egui::Sense::click());
        let p = ui.painter_at(rect);
        let lit = lit(ui, &resp);
        if lit > 0.0 {
            p.rect_filled(rect, 2.0, look.shade(0.18 * lit));
        }
        let font = egui::FontId::proportional(11.0);
        p.text(
            rect.left_center() + egui::vec2(2.0 + 2.0 * lit, 0.0),
            egui::Align2::LEFT_CENTER,
            name,
            font.clone(),
            look.text.lerp_to_gamma(look.title, lit),
        );
        p.text(
            rect.right_center() - egui::vec2(2.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            price(*cents),
            font,
            look.bar(lit),
        );
        if resp
            .on_hover_text("Click to show it in the collection")
            .clicked()
        {
            out.push(Action::ShowRecord(*id));
        }
    }
    ui.label(format!(
        "{} records with one copy for sale, or none",
        crate::format::thousands(v.rare)
    ));
    if let Some(t) = v.oldest {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        ui.label(
            egui::RichText::new(format!(
                "Lowest prices at the last look, the oldest {}",
                crate::format::ago(now.saturating_sub(t))
            ))
            .color(look.dim())
            .small(),
        );
    }
}

fn tempo_chart(ui: &mut egui::Ui, s: &Stats, look: &Look) {
    heading(ui, "TEMPO", look);
    if s.analysed == 0 {
        return empty(ui, "No tracks analysed yet", look);
    }
    let h = 64.0;
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), h + 14.0),
        egui::Sense::hover(),
    );
    let p = ui.painter_at(rect);
    let max = s.bpm.iter().map(|b| b.1).max().unwrap_or(1);
    let step = rect.width() / s.bpm.len().max(1) as f32;
    let font = egui::FontId::proportional(10.0);
    for (i, &(b, n)) in s.bpm.iter().enumerate() {
        let x = rect.left() + i as f32 * step;
        let bh = h * n as f32 / max as f32 * look.grow;
        let resp = ui
            .interact(
                egui::Rect::from_min_max(
                    egui::pos2(x, rect.top()),
                    egui::pos2(x + step, rect.top() + h),
                ),
                ui.id().with(("bpm", b)),
                egui::Sense::hover(),
            )
            .on_hover_text(format!("{b}–{} BPM: {n} tracks", b + 9));
        let lit = lit(ui, &resp);
        let bar = egui::Rect::from_min_max(
            egui::pos2(x + 1.0 - lit, rect.top() + h - bh.max(1.0)),
            egui::pos2(x + step - 1.0 + lit, rect.top() + h),
        );
        p.rect_filled(bar, 1.0, look.bar(lit));
        if lit > 0.0 {
            p.text(
                egui::pos2(bar.center().x, bar.top() - 7.0),
                egui::Align2::CENTER_CENTER,
                n.to_string(),
                font.clone(),
                look.title.gamma_multiply(lit),
            );
        }
        p.text(
            egui::pos2(bar.center().x, rect.bottom() - 6.0),
            egui::Align2::CENTER_CENTER,
            b.to_string(),
            font.clone(),
            look.dim().lerp_to_gamma(look.text, lit),
        );
    }
    ui.label(
        egui::RichText::new(format!(
            "{} tracks analysed",
            crate::format::thousands(s.analysed)
        ))
        .color(look.dim())
        .small(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playlist::{ForSale, Origin, WaitKind};

    /// A record of `tracks` entries.
    struct Rec<'a> {
        release: u64,
        formats: &'a str,
        year: Option<u16>,
        styles: &'a str,
        label: &'a str,
        artist: &'a str,
        price: Option<(u64, &'a str, u32)>,
        tracks: usize,
    }

    fn rec(release: u64, formats: &str) -> Rec<'_> {
        Rec {
            release,
            formats,
            year: Some(1995),
            styles: "Techno",
            label: "Warp Records",
            artist: "Aphex Twin",
            price: None,
            tracks: 1,
        }
    }

    fn playlist(recs: &[Rec]) -> Playlist {
        let mut p = Playlist::default();
        for r in recs {
            for t in 0..r.tracks {
                p.add_waiting(
                    r.artist,
                    format!("T{t}"),
                    None,
                    Some(Origin {
                        release: Some(r.release),
                        formats: r.formats.into(),
                        year: r.year,
                        styles: r.styles.into(),
                        label: r.label.into(),
                        artist: r.artist.into(),
                        album: format!("Album {}", r.release),
                        for_sale: r.price.map(|(cents, cur, count)| ForSale {
                            count,
                            lowest_cents: Some(cents),
                            currency: cur.into(),
                            fetched_at: 1_790_000_000 + r.release,
                        }),
                        ..Default::default()
                    }),
                    WaitKind::Listed,
                );
            }
        }
        p
    }

    #[test]
    fn only_vinyl_records_count_and_each_once() {
        let p = playlist(&[
            Rec {
                tracks: 9,
                ..rec(1, "Vinyl")
            },
            rec(2, "Vinyl, CD"),
            rec(3, "CD"),
            rec(4, "File"),
            rec(5, ""),
        ]);
        let s = Stats::of(&p);
        assert_eq!(
            s.records, 2,
            "9 tracks are one record; CD, File and unknown don't count"
        );
        assert_eq!(s.styles, [("Techno".to_owned(), 2)]);
    }

    #[test]
    fn years_styles_labels_and_artists() {
        let p = playlist(&[
            Rec {
                year: Some(1991),
                styles: "Techno, Acid",
                ..rec(1, "Vinyl")
            },
            Rec {
                year: Some(1999),
                styles: "Acid",
                label: "R & S Records",
                artist: "Various",
                ..rec(2, "Vinyl")
            },
            Rec {
                year: Some(2021),
                styles: "Electro",
                ..rec(3, "Vinyl")
            },
            Rec {
                year: None,
                ..rec(4, "Vinyl")
            },
        ]);
        let s = Stats::of(&p);
        assert_eq!(
            s.years.iter().map(|(y, n)| (*y, *n)).collect::<Vec<_>>(),
            [(1991, 1), (1999, 1), (2021, 1)],
            "no year, no bar"
        );
        assert_eq!(
            s.styles,
            [
                ("Acid".to_owned(), 2),
                ("Techno".to_owned(), 2),
                ("Electro".to_owned(), 1)
            ],
            "most first, then by name"
        );
        assert_eq!(s.labels[0].name, "Warp Records");
        assert_eq!((s.labels[0].records, s.labels[0].release), (3, 1));
        assert_eq!(s.artists, [("Aphex Twin".to_owned(), 3)], "no Various");
        assert_eq!(s.heat.decades, [1990, 2000, 2010, 2020]);
        let acid = s.heat.styles.iter().position(|x| x == "Acid").unwrap();
        assert_eq!(s.heat.cells[acid], [2, 0, 0, 0]);
    }

    #[test]
    fn value_in_the_main_currency_median_dearest_and_rare() {
        let p = playlist(&[
            Rec {
                price: Some((1000, "EUR", 5)),
                ..rec(1, "Vinyl")
            },
            Rec {
                price: Some((3000, "EUR", 1)),
                ..rec(2, "Vinyl")
            },
            Rec {
                price: Some((2000, "EUR", 0)),
                ..rec(3, "Vinyl")
            },
            Rec {
                price: Some((99_900, "USD", 9)),
                ..rec(4, "Vinyl")
            },
            rec(5, "Vinyl"),
        ]);
        let v = Stats::of(&p).value;
        assert_eq!(v.currency, "EUR");
        assert_eq!((v.sum_cents, v.priced, v.other), (6000, 3, 1));
        assert_eq!(v.median_cents, 2000);
        assert_eq!(
            v.dearest
                .iter()
                .map(|d| (d.1.as_str(), d.2))
                .collect::<Vec<_>>(),
            [
                ("Aphex Twin – Album 2", 3000),
                ("Aphex Twin – Album 3", 2000),
                ("Aphex Twin – Album 1", 1000)
            ]
        );
        assert_eq!(v.rare, 2, "one copy, or none, for sale");
        assert_eq!(v.oldest, Some(1_790_000_001));
    }

    #[test]
    fn tempos_by_ten_bpm() {
        let mut p = playlist(&[Rec {
            tracks: 3,
            ..rec(1, "Vinyl")
        }]);
        for (e, b) in p.entries_mut().zip([124, 128, 172]) {
            e.bpm = Some(b);
        }
        let s = Stats::of(&p);
        assert_eq!(s.bpm, [(120, 2), (170, 1)]);
        assert_eq!(s.analysed, 3);
    }

    #[test]
    fn a_big_collection_is_counted_within_a_frame() {
        let recs: Vec<Rec> = (0..5000u64)
            .map(|i| Rec {
                tracks: 8,
                year: Some(1960 + (i % 66) as u16),
                styles: ["Techno, Acid", "Electro", "Drum n Bass, Jungle"][i as usize % 3],
                price: Some((500 + i, "EUR", (i % 7) as u32)),
                ..rec(i + 1, "Vinyl")
            })
            .collect();
        let p = playlist(&recs);
        assert_eq!(p.entries().len(), 40_000);
        let best = (0..3)
            .map(|_| {
                let t = std::time::Instant::now();
                let s = Stats::of(&p);
                assert_eq!(s.records, 5000);
                t.elapsed()
            })
            .min()
            .unwrap();
        // A frame in the shipped (optimized) build; an unoptimized test build is ~10x slower,
        // and slower still while the workspace's tests share the machine.
        let budget = if cfg!(debug_assertions) { 200 } else { 16 };
        assert!(best < std::time::Duration::from_millis(budget), "{best:?}");
    }
}
