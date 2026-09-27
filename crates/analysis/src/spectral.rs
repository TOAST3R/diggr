//! Spectrograms: the bounded whole-track spectral overview built in the overview pass, the
//! shared STFT and log-frequency row mapping used by every view, and the content-cutoff check.
//!
//! The overview must not grow with track length, so columns start 1024 frames wide and merge in
//! pairs whenever there are [`MAX_COLUMNS`]; that also copes with tracks of unknown length. At
//! most [`FRAMES_PER_COLUMN`] FFTs land in a column, so a 2-hour mix costs about as much as a
//! 3-minute track.

use std::sync::Arc;

use realfft::num_complex::Complex;
use realfft::{RealFftPlanner, RealToComplex};

/// Log-spaced frequency rows in the overview.
pub const ROWS: usize = 256;
/// Columns merge in pairs on reaching this count.
pub const MAX_COLUMNS: usize = 4096;
/// Overview FFT size.
pub const FFT: usize = 4096;
/// Narrowest column, and the densest FFT spacing, in frames.
const MIN_COLUMN_FRAMES: u64 = 1024;
/// FFTs per column once columns are wider than [`MIN_COLUMN_FRAMES`].
const FRAMES_PER_COLUMN: u64 = 8;
/// Bottom of the frequency axis.
pub const MIN_HZ: f32 = 20.0;
/// Levels are stored as dB in [-FLOOR_DB, 0] mapped to 0..=255.
pub const FLOOR_DB: f32 = 120.0;
/// A frame counts towards the cutoff estimate only above this level (−30 dBFS RMS).
const LOUD_MEAN_SQUARE: f32 = 1e-3;

/// Which signal a spectrogram shows.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum Channel {
    /// (L + R) / 2.
    #[default]
    Mid,
    /// (L − R) / 2: stereo width.
    Side,
    Left,
    Right,
}

impl Channel {
    /// One sample of this channel from a stereo frame.
    pub fn of(self, l: f32, r: f32) -> f32 {
        match self {
            Channel::Mid => 0.5 * (l + r),
            Channel::Side => 0.5 * (l - r),
            Channel::Left => l,
            Channel::Right => r,
        }
    }
}

pub fn db_to_byte(db: f32) -> u8 {
    ((db + FLOOR_DB) / FLOOR_DB * 255.0)
        .round()
        .clamp(0.0, 255.0) as u8
}

pub fn byte_to_db(b: u8) -> f32 {
    b as f32 / 255.0 * FLOOR_DB - FLOOR_DB
}

pub fn power_to_byte(p: f32) -> u8 {
    db_to_byte(10.0 * p.max(1e-20).log10())
}

/// Frequency at fractional row `row` of `rows` log-spaced rows from [`MIN_HZ`] to `top_hz`
/// (row 0.0 is the bottom edge, `rows as f32` the top).
pub fn row_to_hz(row: f32, rows: usize, top_hz: f32) -> f32 {
    MIN_HZ * (top_hz / MIN_HZ).powf(row / rows as f32)
}

/// Inverse of [`row_to_hz`].
pub fn hz_to_row(hz: f32, rows: usize, top_hz: f32) -> f32 {
    (hz.max(1e-3) / MIN_HZ).ln() / (top_hz / MIN_HZ).ln() * rows as f32
}

/// A windowed FFT returning power normalized so a full-scale sine peaks at 0 dB.
pub struct Stft {
    n: usize,
    fft: Arc<dyn RealToComplex<f32>>,
    window: Vec<f32>,
    input: Vec<f32>,
    output: Vec<Complex<f32>>,
    power: Vec<f32>,
    scale: f32,
}

impl Stft {
    pub fn new(n: usize) -> Self {
        let fft = RealFftPlanner::<f32>::new().plan_fft_forward(n);
        // Periodic Hann; its coherent gain is 1/2, so a unit sine's bin reaches n/4.
        let window = (0..n)
            .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / n as f32).cos())
            .collect();
        Self {
            n,
            input: fft.make_input_vec(),
            output: fft.make_output_vec(),
            fft,
            window,
            power: vec![0.0; n / 2 + 1],
            scale: 1.0 / (n as f32 / 4.0).powi(2),
        }
    }

    pub fn len(&self) -> usize {
        self.n
    }

    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Power per bin (`n / 2 + 1` bins) of `samples`, which yields exactly `n` values.
    pub fn power(&mut self, samples: impl IntoIterator<Item = f32>) -> &[f32] {
        for ((x, s), w) in self.input.iter_mut().zip(samples).zip(&self.window) {
            *x = s * w;
        }
        if self.fft.process(&mut self.input, &mut self.output).is_ok() {
            for (p, c) in self.power.iter_mut().zip(&self.output) {
                *p = c.norm_sqr() * self.scale;
            }
        }
        &self.power
    }
}

/// Maps FFT bins onto log-spaced rows: rows that contain bin centres average them, narrower
/// rows interpolate between the neighbouring bins so none is empty.
#[derive(Debug, Clone)]
pub struct RowMap {
    /// Per row: first bin, one past the last, and for interpolated rows (`start == end`) the
    /// fraction towards `start + 1`.
    rows: Vec<(u32, u32, f32)>,
}

impl RowMap {
    pub fn new(fft_n: usize, sample_rate: u32, rows: usize, lo_hz: f32, hi_hz: f32) -> Self {
        let bin_hz = sample_rate as f32 / fft_n as f32;
        let last = (fft_n / 2) as u32;
        let edge = |r: usize| lo_hz * (hi_hz / lo_hz).powf(r as f32 / rows as f32);
        let rows = (0..rows)
            .map(|r| {
                let (a, b) = (edge(r), edge(r + 1));
                let start = ((a / bin_hz).ceil() as u32).min(last);
                let end = ((b / bin_hz).ceil() as u32).min(last + 1);
                if end > start {
                    (start, end, 0.0)
                } else {
                    let k = ((a * b).sqrt() / bin_hz).min(last as f32 - 1.0);
                    (k.floor() as u32, k.floor() as u32, k.fract())
                }
            })
            .collect();
        Self { rows }
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Writes one power value per row into `out`.
    pub fn map(&self, power: &[f32], out: &mut [f32]) {
        for (o, &(a, b, f)) in out.iter_mut().zip(&self.rows) {
            let (a, b) = (a as usize, b as usize);
            *o = if b > a {
                power[a..b].iter().sum::<f32>() / (b - a) as f32
            } else {
                let next = power.get(a + 1).copied().unwrap_or(power[a]);
                power[a] + (next - power[a]) * f
            };
        }
    }
}

/// What the cutoff estimate is built from: loud frames of the whole track.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CutoffStats {
    /// Bin width of `mean_power` and `hist`, from the overview FFT.
    pub bin_hz: f32,
    pub loud_frames: u64,
    /// Sum of power per bin over loud frames (both channels: (|L|² + |R|²) / 2).
    pub power_sum: Vec<f64>,
    /// Per-frame cutoff bin counts.
    pub hist: Vec<u32>,
}

/// Where a track's content ends.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cutoff {
    pub hz: f32,
    /// Level drop from just below the cutoff to 0.5–1 kHz above it.
    pub drop_db: f32,
    /// More than a 40 dB drop within 500 Hz: a filter, not a natural roll-off.
    pub steep: bool,
}

impl Cutoff {
    /// A lossless file whose content ends in a brick wall below 19.5 kHz was most likely made
    /// from a lossy one.
    pub fn likely_lossy(&self, lossless: bool) -> bool {
        lossless && self.steep && self.hz < 19_500.0
    }

    /// The one-line quality verdict shown in the spectrogram window.
    pub fn describe(&self, lossless: bool) -> String {
        let khz = self.hz / 1000.0;
        if self.likely_lossy(lossless) {
            format!(
                "Content ends at {khz:.1} kHz: likely from a lossy source ({})",
                bitrate_hint(self.hz)
            )
        } else {
            format!("Content ends at {khz:.1} kHz")
        }
    }
}

/// Typical lowpass of common MP3 encoder settings.
pub fn bitrate_hint(hz: f32) -> &'static str {
    match hz {
        h if h < 14_000.0 => "≈96 kbps MP3 or lower",
        h if h < 17_500.0 => "≈128 kbps MP3",
        h if h < 19_750.0 => "≈192 kbps MP3",
        _ => "≈256–320 kbps MP3",
    }
}

impl CutoffStats {
    fn new(bin_hz: f32, bins: usize) -> Self {
        Self {
            bin_hz,
            loud_frames: 0,
            power_sum: vec![0.0; bins],
            hist: vec![0; bins],
        }
    }

    fn mean_db(&self, from_hz: f32, to_hz: f32) -> Option<f32> {
        let a = (from_hz / self.bin_hz).ceil().max(1.0) as usize;
        let b = ((to_hz / self.bin_hz).floor() as usize + 1).min(self.power_sum.len());
        (b > a).then(|| {
            let mean = self.power_sum[a..b].iter().sum::<f64>()
                / ((b - a) as f64 * self.loud_frames as f64);
            10.0 * (mean.max(1e-20)).log10() as f32
        })
    }

    /// The 90th percentile of per-frame cutoffs, with the steepness of the edge there.
    pub fn cutoff(&self) -> Option<Cutoff> {
        const MIN_LOUD_FRAMES: u64 = 8;
        if self.loud_frames < MIN_LOUD_FRAMES {
            return None;
        }
        let total: u64 = self.hist.iter().map(|&n| n as u64).sum();
        let mut seen = 0;
        let bin = self.hist.iter().position(|&n| {
            seen += n as u64;
            seen * 10 >= total * 9
        })?;
        let hz = bin as f32 * self.bin_hz;
        let below = self.mean_db(hz - 300.0, hz);
        let above = self.mean_db(hz + 500.0, hz + 1000.0);
        let drop_db = match (below, above) {
            (Some(b), Some(a)) => b - a,
            _ => 0.0,
        };
        Some(Cutoff {
            hz,
            drop_db,
            steep: drop_db > 40.0,
        })
    }

    fn add_frame(&mut self, power: &[f32], scratch: &mut Vec<f32>) {
        self.loud_frames += 1;
        for (s, &p) in self.power_sum.iter_mut().zip(power) {
            *s += p as f64;
        }
        // The frame's cutoff: the highest bin within 30 dB of its 2–8 kHz median.
        let (a, b) = (
            (2000.0 / self.bin_hz) as usize,
            ((8000.0 / self.bin_hz) as usize).min(power.len()),
        );
        if b <= a {
            return;
        }
        scratch.clear();
        scratch.extend_from_slice(&power[a..b]);
        let mid = scratch.len() / 2;
        let median = *scratch.select_nth_unstable_by(mid, f32::total_cmp).1;
        let threshold = median * 1e-3;
        if let Some(k) = power.iter().rposition(|&p| p >= threshold) {
            self.hist[k] += 1;
        }
    }
}

/// The whole track's spectrogram at overview resolution, for mid and side.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Spectral {
    pub sample_rate: u32,
    /// Frames per column (1024 × a power of two).
    pub column_frames: u64,
    pub columns: usize,
    /// Column-major: `mid[c * ROWS + r]`, rows from low to high, dB bytes.
    pub mid: Vec<u8>,
    pub side: Vec<u8>,
    pub cutoff: CutoffStats,
}

impl Spectral {
    /// The top of the frequency axis.
    pub fn nyquist(&self) -> f32 {
        self.sample_rate as f32 / 2.0
    }

    /// Seconds per column.
    pub fn column_secs(&self) -> f64 {
        self.column_frames as f64 / self.sample_rate.max(1) as f64
    }

    /// One column's rows, or `None` past what has been built. Only mid and side are stored.
    pub fn column(&self, channel: Channel, c: usize) -> Option<&[u8]> {
        let data = match channel {
            Channel::Side => &self.side,
            _ => &self.mid,
        };
        data.get(c * ROWS..(c + 1) * ROWS)
    }

    /// The column covering `secs`, if built.
    pub fn column_at(&self, secs: f64) -> Option<usize> {
        let c = (secs / self.column_secs()).floor();
        (c >= 0.0 && (c as usize) < self.columns).then_some(c as usize)
    }

    pub fn bytes(&self) -> usize {
        self.mid.len()
            + self.side.len()
            + self.cutoff.power_sum.len() * 8
            + self.cutoff.hist.len() * 4
    }

    pub(crate) fn write_to(&self, w: &mut impl std::io::Write) -> std::io::Result<()> {
        w.write_all(&self.column_frames.to_le_bytes())?;
        w.write_all(&(self.columns as u64).to_le_bytes())?;
        w.write_all(&self.mid)?;
        w.write_all(&self.side)?;
        let c = &self.cutoff;
        w.write_all(&c.bin_hz.to_le_bytes())?;
        w.write_all(&c.loud_frames.to_le_bytes())?;
        w.write_all(&(c.power_sum.len() as u64).to_le_bytes())?;
        for p in &c.power_sum {
            w.write_all(&p.to_le_bytes())?;
        }
        for n in &c.hist {
            w.write_all(&n.to_le_bytes())?;
        }
        Ok(())
    }

    pub(crate) fn read_from(r: &mut impl std::io::Read, sample_rate: u32) -> std::io::Result<Self> {
        fn take<const N: usize>(r: &mut impl std::io::Read) -> std::io::Result<[u8; N]> {
            let mut b = [0u8; N];
            r.read_exact(&mut b)?;
            Ok(b)
        }
        let bad = || std::io::Error::new(std::io::ErrorKind::InvalidData, "bad spectral data");
        let column_frames = u64::from_le_bytes(take(r)?);
        let columns = u64::from_le_bytes(take(r)?) as usize;
        if columns > MAX_COLUMNS {
            return Err(bad());
        }
        let mut mid = vec![0u8; columns * ROWS];
        r.read_exact(&mut mid)?;
        let mut side = vec![0u8; columns * ROWS];
        r.read_exact(&mut side)?;
        let bin_hz = f32::from_le_bytes(take(r)?);
        let loud_frames = u64::from_le_bytes(take(r)?);
        let bins = u64::from_le_bytes(take(r)?) as usize;
        if bins > FFT / 2 + 1 {
            return Err(bad());
        }
        let power_sum = (0..bins)
            .map(|_| take(r).map(f64::from_le_bytes))
            .collect::<std::io::Result<_>>()?;
        let hist = (0..bins)
            .map(|_| take(r).map(u32::from_le_bytes))
            .collect::<std::io::Result<_>>()?;
        Ok(Self {
            sample_rate,
            column_frames,
            columns,
            mid,
            side,
            cutoff: CutoffStats {
                bin_hz,
                loud_frames,
                power_sum,
                hist,
            },
        })
    }
}

/// Builds a [`Spectral`] from stereo frames pushed in order.
pub(crate) struct SpectralBuilder {
    out: Spectral,
    stft: Stft,
    rows: RowMap,
    /// The last [`FFT`] frames of mid and side (circular, next write at `pos`).
    mid_hist: Vec<f32>,
    side_hist: Vec<f32>,
    pos: usize,
    /// Mean square of both channels over the history, kept as a running sum.
    energy: f64,
    frames: u64,
    until_fft: u64,
    /// Power sums of the column being filled.
    acc_mid: Vec<f32>,
    acc_side: Vec<f32>,
    acc_n: u32,
    rows_scratch: Vec<f32>,
    both: Vec<f32>,
    scratch: Vec<f32>,
}

/// dB byte → power, for merging columns.
fn byte_power(b: u8) -> f32 {
    10f32.powf(byte_to_db(b) / 10.0)
}

impl SpectralBuilder {
    pub(crate) fn new(sample_rate: u32) -> Self {
        let bins = FFT / 2 + 1;
        Self {
            out: Spectral {
                sample_rate,
                column_frames: MIN_COLUMN_FRAMES,
                cutoff: CutoffStats::new(sample_rate as f32 / FFT as f32, bins),
                ..Default::default()
            },
            stft: Stft::new(FFT),
            rows: RowMap::new(FFT, sample_rate, ROWS, MIN_HZ, sample_rate as f32 / 2.0),
            mid_hist: vec![0.0; FFT],
            side_hist: vec![0.0; FFT],
            pos: 0,
            energy: 0.0,
            frames: 0,
            until_fft: MIN_COLUMN_FRAMES,
            acc_mid: vec![0.0; ROWS],
            acc_side: vec![0.0; ROWS],
            acc_n: 0,
            rows_scratch: vec![0.0; ROWS],
            both: vec![0.0; bins],
            scratch: Vec::new(),
        }
    }

    fn hop(&self) -> u64 {
        (self.out.column_frames / FRAMES_PER_COLUMN).max(MIN_COLUMN_FRAMES)
    }

    pub(crate) fn push_frame(&mut self, l: f32, r: f32) {
        let (m, s) = (0.5 * (l + r), 0.5 * (l - r));
        let old = self.mid_hist[self.pos].powi(2) + self.side_hist[self.pos].powi(2);
        self.energy += (m * m + s * s - old) as f64;
        self.mid_hist[self.pos] = m;
        self.side_hist[self.pos] = s;
        self.pos = (self.pos + 1) % FFT;
        self.frames += 1;
        self.until_fft -= 1;
        if self.until_fft == 0 {
            self.until_fft = self.hop();
            self.analyze();
        }
    }

    fn analyze(&mut self) {
        // The window ends at the newest frame; it counts towards the column holding its centre.
        let centre = self.frames.saturating_sub(FFT as u64 / 2);
        let col = (centre / self.out.column_frames) as usize;
        while col > self.out.columns && self.acc_n > 0 {
            self.close_column();
        }
        let (a, b) = self.mid_hist.split_at(self.pos);
        let pm = self.stft.power(b.iter().chain(a).copied());
        self.both[..pm.len()].copy_from_slice(pm);
        self.rows.map(pm, &mut self.rows_scratch);
        for (acc, v) in self.acc_mid.iter_mut().zip(&self.rows_scratch) {
            *acc += v;
        }
        let (a, b) = self.side_hist.split_at(self.pos);
        let ps = self.stft.power(b.iter().chain(a).copied());
        for (x, p) in self.both.iter_mut().zip(ps) {
            *x += p;
        }
        self.rows.map(ps, &mut self.rows_scratch);
        for (acc, v) in self.acc_side.iter_mut().zip(&self.rows_scratch) {
            *acc += v;
        }
        self.acc_n += 1;
        if (self.energy / FFT as f64) as f32 > LOUD_MEAN_SQUARE {
            self.out.cutoff.add_frame(&self.both, &mut self.scratch);
        }
    }

    fn close_column(&mut self) {
        let n = self.acc_n.max(1) as f32;
        for (acc, data) in [
            (&mut self.acc_mid, &mut self.out.mid),
            (&mut self.acc_side, &mut self.out.side),
        ] {
            data.extend(acc.iter().map(|p| power_to_byte(p / n)));
            acc.fill(0.0);
        }
        self.acc_n = 0;
        self.out.columns += 1;
        if self.out.columns == MAX_COLUMNS {
            for data in [&mut self.out.mid, &mut self.out.side] {
                let merged: Vec<u8> = data
                    .as_chunks::<{ 2 * ROWS }>()
                    .0
                    .iter()
                    .flat_map(|pair| {
                        let (a, b) = pair.split_at(ROWS);
                        a.iter()
                            .zip(b)
                            .map(|(&x, &y)| power_to_byte(0.5 * (byte_power(x) + byte_power(y))))
                            .collect::<Vec<_>>()
                    })
                    .collect();
                *data = merged;
            }
            self.out.columns /= 2;
            self.out.column_frames *= 2;
        }
    }

    /// What has been built so far (whole columns only).
    pub(crate) fn snapshot(&self) -> Spectral {
        self.out.clone()
    }

    /// Closes the last partial column.
    pub(crate) fn finish(mut self) -> Spectral {
        if self.acc_n > 0 {
            self.close_column();
        }
        self.out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: u32 = 44_100;

    fn push_mono(b: &mut SpectralBuilder, x: impl IntoIterator<Item = f32>) {
        for v in x {
            b.push_frame(v, v);
        }
    }

    fn sine(hz: f32, secs: f32, amp: f32) -> impl Iterator<Item = f32> {
        (0..(secs * SR as f32) as usize)
            .map(move |i| amp * (std::f32::consts::TAU * hz * i as f32 / SR as f32).sin())
    }

    /// Noise with exactly the magnitude spectrum `gain(hz)` up to `top_hz` and nothing above:
    /// random phases in one long inverse FFT, scaled to −14 dBFS RMS.
    pub(crate) fn shaped_noise(secs: f32, top_hz: f32, gain: impl Fn(f32) -> f32) -> Vec<f32> {
        let n = ((secs * SR as f32) as usize).next_power_of_two();
        let ifft = RealFftPlanner::<f32>::new().plan_fft_inverse(n);
        let mut spec = ifft.make_input_vec();
        let mut seed = 0x2545_f491_4f6c_dd1du64;
        for (k, c) in spec.iter_mut().enumerate().skip(1) {
            let hz = k as f32 * SR as f32 / n as f32;
            if hz > top_hz || k == n / 2 {
                continue;
            }
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let phase = (seed >> 40) as f32 / (1u64 << 24) as f32 * std::f32::consts::TAU;
            *c = Complex::from_polar(gain(hz), phase);
        }
        let mut out = ifft.make_output_vec();
        ifft.process(&mut spec, &mut out).unwrap();
        let rms = (out.iter().map(|x| x * x).sum::<f32>() / n as f32).sqrt();
        out.iter_mut().for_each(|x| *x *= 0.2 / rms);
        out
    }

    #[test]
    fn rows_are_log_spaced_and_invertible() {
        let top = SR as f32 / 2.0;
        assert!((row_to_hz(0.0, ROWS, top) - 20.0).abs() < 1e-3);
        assert!((row_to_hz(ROWS as f32, ROWS, top) - top).abs() < 0.1);
        let r = hz_to_row(1000.0, ROWS, top);
        assert!((row_to_hz(r, ROWS, top) - 1000.0).abs() < 0.01);
        assert_eq!(byte_to_db(db_to_byte(-60.0)).round(), -60.0);
    }

    #[test]
    fn every_row_gets_a_value() {
        let map = RowMap::new(FFT, SR, ROWS, MIN_HZ, SR as f32 / 2.0);
        let power: Vec<f32> = (0..=FFT / 2).map(|k| 1.0 + k as f32).collect();
        let mut out = vec![0.0; ROWS];
        map.map(&power, &mut out);
        assert!(out.iter().all(|&v| v >= 1.0), "no empty rows");
        assert!(
            out.windows(2).all(|w| w[1] >= w[0]),
            "rising power maps to rising rows"
        );
    }

    #[test]
    fn a_full_scale_sine_reads_0_db() {
        let mut stft = Stft::new(FFT);
        let k = 100;
        let hz = k as f32 * SR as f32 / FFT as f32;
        let p = stft.power(sine(hz, 1.0, 1.0).take(FFT))[k];
        assert!((10.0 * p.log10()).abs() < 0.1, "{} dB", 10.0 * p.log10());
    }

    #[test]
    fn a_tone_lands_on_its_row() {
        let mut b = SpectralBuilder::new(SR);
        push_mono(&mut b, sine(1000.0, 5.0, 0.5));
        let s = b.finish();
        // The strongest row's band contains 1 kHz, to within one FFT bin (10.8 Hz).
        let bin = SR as f32 / FFT as f32;
        for c in 4..s.columns - 4 {
            let col = s.column(Channel::Mid, c).unwrap();
            let top = (0..ROWS).max_by_key(|&r| col[r]).unwrap();
            let (lo, hi) = (
                row_to_hz(top as f32, ROWS, s.nyquist()),
                row_to_hz(top as f32 + 1.0, ROWS, s.nyquist()),
            );
            assert!(
                lo - bin <= 1000.0 && 1000.0 < hi + bin,
                "column {c}: {lo}–{hi} Hz"
            );
            assert!(
                col[top] > db_to_byte(-20.0),
                "loud: {}",
                byte_to_db(col[top])
            );
        }
        assert!(
            s.column(Channel::Side, 10).unwrap().iter().all(|&v| v < 10),
            "identical channels leave side silent"
        );
    }

    #[test]
    fn size_is_bounded_for_any_length() {
        let quiet = |secs: u64| {
            let mut b = SpectralBuilder::new(SR);
            for _ in 0..secs * SR as u64 {
                b.push_frame(0.01, -0.01);
            }
            b.finish()
        };
        for secs in [180, 600] {
            let s = quiet(secs);
            assert!(
                (MAX_COLUMNS / 2..MAX_COLUMNS).contains(&s.columns),
                "{secs} s: {} columns",
                s.columns
            );
            let covered = s.columns as f64 * s.column_secs();
            assert!(
                (covered - secs as f64).abs() < s.column_secs() * 2.0,
                "{covered}"
            );
            assert!(s.bytes() <= (2 << 20) + (64 << 10), "{} bytes", s.bytes());
        }
    }

    #[test]
    fn brick_wall_at_16_khz_is_flagged() {
        let mut b = SpectralBuilder::new(SR);
        push_mono(&mut b, shaped_noise(4.0, 16_000.0, |_| 1.0));
        let c = b.finish().cutoff.cutoff().expect("loud enough");
        assert!((c.hz - 16_000.0).abs() < 150.0, "{c:?}");
        assert!(c.steep, "{c:?}");
        assert!(c.likely_lossy(true));
        assert!(!c.likely_lossy(false), "lossy codecs are never flagged");
        assert_eq!(
            c.describe(true),
            "Content ends at 16.0 kHz: likely from a lossy source (≈128 kbps MP3)"
        );
    }

    #[test]
    fn gradual_roll_off_is_not_flagged() {
        // Flat to 2 kHz, then −24 dB per octave: dark, but no filter edge.
        let mut b = SpectralBuilder::new(SR);
        let gain = |hz: f32| {
            if hz < 2000.0 {
                1.0
            } else {
                (2000.0 / hz).powi(4)
            }
        };
        push_mono(&mut b, shaped_noise(4.0, 21_000.0, gain));
        let c = b.finish().cutoff.cutoff().expect("loud enough");
        assert!(c.hz < 19_500.0, "a cutoff is found: {c:?}");
        assert!(!c.steep && c.drop_db < 20.0, "{c:?}");
        assert!(!c.likely_lossy(true));
    }

    #[test]
    fn full_band_content_is_not_flagged() {
        let mut b = SpectralBuilder::new(SR);
        push_mono(&mut b, shaped_noise(3.0, 21_000.0, |_| 1.0));
        let c = b.finish().cutoff.cutoff().unwrap();
        assert!(c.hz > 20_000.0, "{c:?}");
        assert!(!c.likely_lossy(true));
    }

    #[test]
    fn silence_has_no_cutoff() {
        let mut b = SpectralBuilder::new(SR);
        push_mono(&mut b, std::iter::repeat_n(0.0, 3 * SR as usize));
        assert_eq!(b.finish().cutoff.cutoff(), None);
    }

    #[test]
    fn round_trip() {
        let mut b = SpectralBuilder::new(SR);
        push_mono(&mut b, shaped_noise(2.0, 10_000.0, |_| 1.0));
        let s = b.finish();
        let mut bytes = Vec::new();
        s.write_to(&mut bytes).unwrap();
        let back = Spectral::read_from(&mut bytes.as_slice(), SR).unwrap();
        assert_eq!(back, s);
    }
}
