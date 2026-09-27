//! Song structure on the beat grid: per-beat features, self-similarity novelty, phrase-snapped
//! boundaries, section kinds, labels for repeated sections, and the tension curve.

use crate::frontend::{Frame, MEL_BANDS};
use crate::rhythm::Onsets;
use crate::score::{Section, SectionKind};

/// Features of one beat (averaged over the frames between it and the next beat).
#[derive(Debug, Clone, PartialEq)]
pub struct BeatFeat {
    pub mel: [f32; MEL_BANDS],
    pub chroma: [f32; 12],
    /// Loudness in dB (RMS).
    pub energy: f32,
    pub bass: f32,
    pub mid: f32,
    pub treble: f32,
    pub brightness: f32,
    pub flatness: f32,
    /// A kick onset within ±40 ms of the beat.
    pub kick: bool,
    /// Onsets (kick + snare + hat) during the beat.
    pub density: f32,
}

pub fn beat_features(beats: &[f64], frames: &[Frame], onsets: &Onsets) -> Vec<BeatFeat> {
    let count = |list: &[(f64, f32)], a: f64, b: f64| {
        list.iter().filter(|(t, _)| *t >= a && *t < b).count()
    };
    beats
        .iter()
        .enumerate()
        .map(|(i, &a)| {
            let b = beats
                .get(i + 1)
                .copied()
                .unwrap_or_else(|| a + if i > 0 { a - beats[i - 1] } else { 0.5 });
            let i0 = frames.partition_point(|f| f.t < a);
            let i1 = frames
                .partition_point(|f| f.t < b)
                .max(i0 + 1)
                .min(frames.len());
            let fr = &frames[i0.min(frames.len().saturating_sub(1))..i1];
            let n = fr.len().max(1) as f32;
            let mut mel = [0.0; MEL_BANDS];
            let mut chroma = [0.0; 12];
            let (mut rms, mut bass, mut mid, mut treble, mut bright, mut flat) =
                (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
            for f in fr {
                for (m, v) in mel.iter_mut().zip(&f.mel) {
                    *m += v / n;
                }
                for (c, v) in chroma.iter_mut().zip(&f.chroma) {
                    *c += v / n;
                }
                rms += f.rms * f.rms / n;
                bass += f.bass / n;
                mid += f.mid / n;
                treble += f.treble / n;
                bright += f.centroid / n;
                flat += f.flatness / n;
            }
            let kick = onsets.kick.iter().any(|(t, _)| (t - a).abs() < 0.04);
            let density = (count(&onsets.kick, a, b)
                + count(&onsets.snare, a, b)
                + count(&onsets.hat, a, b)) as f32;
            BeatFeat {
                mel,
                chroma,
                energy: 10.0 * (rms + 1e-10).log10(),
                bass,
                mid,
                treble,
                brightness: bright,
                flatness: flat,
                kick,
                density,
            }
        })
        .collect()
}

/// Feature vectors for self-similarity: each group z-scored over the beats and weighted so
/// timbre, harmony, rhythm and loudness contribute comparably.
fn ssm_vectors(feats: &[BeatFeat]) -> Vec<Vec<f32>> {
    let raw: Vec<Vec<f32>> = feats
        .iter()
        .map(|f| {
            let mut v: Vec<f32> = f.mel.to_vec();
            v.extend_from_slice(&f.chroma);
            v.extend([f.kick as u8 as f32, f.density, f.energy]);
            v
        })
        .collect();
    let dims = raw.first().map_or(0, Vec::len);
    let n = raw.len().max(1) as f32;
    let mut mean = vec![0.0; dims];
    let mut sd = vec![0.0; dims];
    for v in &raw {
        for (m, x) in mean.iter_mut().zip(v) {
            *m += x / n;
        }
    }
    for v in &raw {
        for ((s, x), m) in sd.iter_mut().zip(v).zip(&mean) {
            *s += (x - m) * (x - m) / n;
        }
    }
    let weight = |d: usize| {
        if d < MEL_BANDS {
            1.0 / (MEL_BANDS as f32).sqrt()
        } else if d < MEL_BANDS + 12 {
            1.0 / 12f32.sqrt()
        } else {
            1.0
        }
    };
    raw.iter()
        .map(|v| {
            v.iter()
                .enumerate()
                .map(|(d, x)| (x - mean[d]) / (sd[d].sqrt() + 1e-6) * weight(d))
                .collect()
        })
        .collect()
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let (mut ab, mut aa, mut bb) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(b) {
        ab += x * y;
        aa += x * x;
        bb += y * y;
    }
    if aa < 1e-12 || bb < 1e-12 {
        0.0
    } else {
        ab / (aa * bb).sqrt()
    }
}

/// Foote novelty: a Gaussian-tapered checkerboard kernel of `half` beats slid along the
/// diagonal of the self-similarity matrix. High where the music changes character.
pub fn novelty(feats: &[BeatFeat], half: usize) -> Vec<f32> {
    let v = ssm_vectors(feats);
    let n = v.len();
    let sigma = half as f32 / 2.0;
    let g: Vec<f32> = (0..half)
        .map(|a| (-(a as f32 + 0.5).powi(2) / (2.0 * sigma * sigma)).exp())
        .collect();
    (0..n)
        .map(|i| {
            if i < half || i + half > n {
                return 0.0;
            }
            let mut s = 0.0;
            for a in 0..half {
                for b in 0..half {
                    let w = g[a] * g[b];
                    // Same side: similar is expected (+); across the center: dissimilar (−).
                    s += w * cosine(&v[i - 1 - a], &v[i - 1 - b]);
                    s += w * cosine(&v[i + a], &v[i + b]);
                    s -= w * cosine(&v[i - 1 - a], &v[i + b]);
                    s -= w * cosine(&v[i + a], &v[i - 1 - b]);
                }
            }
            s.max(0.0)
        })
        .collect()
}

/// Novelty peaks (beat indices): local maxima over ±2 bars that stand out from the lowest
/// novelty within ±4 bars by at least 40% of the mean novelty (prominence, so a very strong
/// boundary elsewhere does not hide a weaker real one), at least `min_gap` beats apart.
pub fn peaks(nov: &[f32], min_gap: usize) -> Vec<usize> {
    let n = nov.len() as f32;
    if n == 0.0 {
        return Vec::new();
    }
    let mean = nov.iter().sum::<f32>() / n;
    let mut out: Vec<usize> = Vec::new();
    for i in 0..nov.len() {
        let (a, b) = (i.saturating_sub(8), (i + 9).min(nov.len()));
        if nov[a..b].iter().any(|&x| x > nov[i]) {
            continue;
        }
        let (a, b) = (i.saturating_sub(16), (i + 17).min(nov.len()));
        let floor = nov[a..b].iter().copied().fold(f32::MAX, f32::min);
        if nov[i] - floor < 0.4 * mean || nov[i] <= mean {
            continue;
        }
        match out.last() {
            Some(&last) if i - last < min_gap => {
                if nov[i] > nov[last] {
                    *out.last_mut().expect("non-empty") = i;
                }
            }
            _ => out.push(i),
        }
    }
    out
}

/// The bar offset (mod 8) that puts the most boundaries on 8-bar phrase starts.
pub fn phrase_origin(boundary_bars: &[f64]) -> u8 {
    (0..8u8)
        .max_by_key(|&o| {
            let hits = boundary_bars
                .iter()
                .filter(|&&b| {
                    let d = (b - o as f64).rem_euclid(8.0);
                    d.min(8.0 - d) <= 1.0
                })
                .count();
            (hits, std::cmp::Reverse(o))
        })
        .unwrap_or(0)
}

/// Snaps a boundary (in bars, fractional) to the phrase grid when within ±2 bars, else to the
/// nearest bar.
pub fn snap(bar: f64, origin: u8) -> i64 {
    let phrase = ((bar - origin as f64) / 8.0).round() * 8.0 + origin as f64;
    if (bar - phrase).abs() <= 2.0 {
        phrase as i64
    } else {
        bar.round() as i64
    }
}

#[derive(Debug, Clone, Copy)]
struct SegStats {
    kick: f32,
    energy: f32,
    density_slope: f32,
    bright_slope: f32,
    flat_slope: f32,
}

fn slope(ys: &[f32]) -> f32 {
    let n = ys.len() as f32;
    if n < 2.0 {
        return 0.0;
    }
    let mx = (n - 1.0) / 2.0;
    let my = ys.iter().sum::<f32>() / n;
    let (mut num, mut den) = (0.0, 0.0);
    for (i, y) in ys.iter().enumerate() {
        num += (i as f32 - mx) * (y - my);
        den += (i as f32 - mx).powi(2);
    }
    // Change over the whole segment, relative to its mean level.
    num / den * n / my.abs().max(1e-6)
}

fn stats(f: &[BeatFeat]) -> SegStats {
    let n = f.len().max(1) as f32;
    SegStats {
        kick: f.iter().filter(|b| b.kick).count() as f32 / n,
        energy: f.iter().map(|b| b.energy).sum::<f32>() / n,
        density_slope: slope(&f.iter().map(|b| b.density + 1.0).collect::<Vec<_>>()),
        bright_slope: slope(&f.iter().map(|b| b.brightness).collect::<Vec<_>>()),
        flat_slope: slope(&f.iter().map(|b| b.flatness + 1e-3).collect::<Vec<_>>()),
    }
}

/// Section kinds for consecutive segments (`bounds` are beat indices, first 0, last n).
/// `complete`: the segment list reaches the end of the track (enables `outro`).
pub fn classify(feats: &[BeatFeat], bounds: &[usize], complete: bool) -> Vec<SectionKind> {
    let st: Vec<SegStats> = bounds
        .windows(2)
        .map(|w| stats(&feats[w[0]..w[1]]))
        .collect();
    let mut energies: Vec<f32> = st.iter().map(|s| s.energy).collect();
    energies.sort_by(f32::total_cmp);
    let pct = |p: f32| {
        energies
            .get(((energies.len() - 1) as f32 * p).round() as usize)
            .copied()
            .unwrap_or(0.0)
    };
    let (median, p75) = (pct(0.5), pct(0.75));
    let mut kinds: Vec<SectionKind> = Vec::with_capacity(st.len());
    for (i, s) in st.iter().enumerate() {
        let rising = s.density_slope > 0.3 || s.bright_slope > 0.15 || s.flat_slope > 0.3;
        let prev = kinds.last().copied();
        let kind = if s.kick < 0.5 && rising {
            SectionKind::Build
        } else if s.kick >= 0.8
            && s.energy >= p75 - 0.5
            && (matches!(prev, Some(SectionKind::Build | SectionKind::Breakdown))
                || s.energy >= energies[energies.len() - 1] - 0.5)
            && i > 0
        {
            SectionKind::Drop
        } else if s.kick <= 0.2 && s.energy < median + 0.5 {
            SectionKind::Breakdown
        } else if i == 0 {
            SectionKind::Intro
        } else if complete && i + 1 == st.len() && s.energy < p75 {
            SectionKind::Outro
        } else {
            SectionKind::Groove
        };
        kinds.push(kind);
    }
    kinds
}

/// Labels for segments: similar-sounding segments share a label, assigned in order of first
/// appearance. `prototypes` carries labels across calls (streaming).
pub fn label(feats: &[BeatFeat], bounds: &[usize], prototypes: &mut Vec<Vec<f32>>) -> Vec<u8> {
    const SAME: f32 = 0.92;
    bounds
        .windows(2)
        .map(|w| {
            let seg = &feats[w[0]..w[1]];
            let n = seg.len().max(1) as f32;
            let mut p = vec![0.0; MEL_BANDS + 12 + 1];
            for f in seg {
                for (d, v) in f.mel.iter().enumerate() {
                    p[d] += v / n;
                }
                for (d, v) in f.chroma.iter().enumerate() {
                    p[MEL_BANDS + d] += 8.0 * v / n;
                }
                p[MEL_BANDS + 12] += f.kick as u8 as f32 * 4.0 / n;
            }
            let mean = p[..MEL_BANDS].iter().sum::<f32>() / MEL_BANDS as f32;
            p[..MEL_BANDS].iter_mut().for_each(|x| *x -= mean);
            match prototypes.iter().position(|q| cosine(q, &p) >= SAME) {
                Some(i) => i as u8,
                None => {
                    prototypes.push(p);
                    (prototypes.len() - 1) as u8
                }
            }
        })
        .collect()
}

/// Per-beat tension: rises through each build that leads into a drop (ease-out, so it is near
/// its peak for the last bars) and is 0 elsewhere. A build still open at the end of the analyzed
/// range also rises (its drop is not known yet).
pub fn tension(sections: &[Section], beats: usize) -> Vec<f32> {
    let mut t = vec![0.0; beats];
    for (i, s) in sections.iter().enumerate() {
        let leads_to_drop = sections
            .get(i + 1)
            .is_none_or(|n| n.kind == SectionKind::Drop);
        if s.kind != SectionKind::Build || !leads_to_drop {
            continue;
        }
        let len = (s.end_beat - s.start_beat).max(1) as f32;
        let end = s.end_beat.min(beats);
        for (k, v) in t[s.start_beat.min(end)..end].iter_mut().enumerate() {
            let p = k as f32 / len;
            *v = 1.0 - (1.0 - p) * (1.0 - p);
        }
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::{FrontEnd, SR};
    use crate::rhythm;
    use crate::synth::{self, Pattern, part};

    struct Analyzed {
        s: synth::Synth,
        beats: Vec<f64>,
        phase: usize,
        feats: Vec<BeatFeat>,
    }

    fn analyze(parts: &[synth::Part]) -> Analyzed {
        let s = synth::render(parts, SR, 9);
        let mut fe = FrontEnd::new(0);
        let mut fr = Vec::new();
        fe.push(&s.samples, &mut fr);
        let on = rhythm::detect(&fr);
        let beats = rhythm::beats(&rhythm::tempo_segments(&fr, &on));
        let phase = rhythm::downbeat_phase(&beats, &fr, &on) as usize;
        let feats = beat_features(&beats, &fr, &on);
        Analyzed {
            s,
            beats,
            phase,
            feats,
        }
    }

    /// Boundaries in bars (snapped), as the analyzer computes them.
    fn boundaries(a: &Analyzed) -> Vec<i64> {
        let nov = novelty(&a.feats, 16);
        let raw: Vec<f64> = peaks(&nov, 16)
            .iter()
            .map(|&b| (b as f64 - a.phase as f64) / 4.0)
            .collect();
        let origin = phrase_origin(&raw);
        raw.iter().map(|&b| snap(b, origin)).collect()
    }

    fn truth_bars(a: &Analyzed) -> Vec<i64> {
        let beat = a.beats[1] - a.beats[0];
        a.s.sections[1..]
            .iter()
            .map(|&(t, _)| ((t - a.s.beats[0]) / beat / 4.0).round() as i64)
            .collect()
    }

    #[test]
    fn boundaries_land_on_phrases() {
        let a = analyze(&synth::standard_track(128.0));
        let found = boundaries(&a);
        let truth = truth_bars(&a);
        let hits = truth
            .iter()
            .filter(|t| found.iter().any(|f| (f - *t).abs() <= 1))
            .count();
        assert!(hits == truth.len(), "found {found:?}, truth {truth:?}");
        assert!(
            found.len() <= truth.len() + 1,
            "few spurious boundaries: {found:?}"
        );
    }

    fn sections_for(a: &Analyzed, complete: bool) -> Vec<Section> {
        let bars = boundaries(a);
        let mut bounds = vec![0];
        bounds.extend(bars.iter().map(|&b| (b * 4) as usize + a.phase));
        bounds.push(a.feats.len());
        bounds.dedup();
        let kinds = classify(&a.feats, &bounds, complete);
        let labels = label(&a.feats, &bounds, &mut Vec::new());
        bounds
            .windows(2)
            .zip(kinds.iter().zip(&labels))
            .map(|(w, (&kind, &label))| Section {
                start_beat: w[0],
                end_beat: w[1],
                label,
                kind,
                is_final: true,
            })
            .collect()
    }

    #[test]
    fn kinds_and_labels_of_a_standard_track() {
        for bpm in [124.0, 128.0, 140.0] {
            let a = analyze(&synth::standard_track(bpm));
            let secs = sections_for(&a, true);
            let kinds: Vec<SectionKind> = secs.iter().map(|s| s.kind).collect();
            let truth: Vec<SectionKind> = a.s.sections.iter().map(|s| s.1).collect();
            assert_eq!(kinds, truth, "{bpm} BPM");
            // Both drops share a label, and the breakdown differs from them.
            let drops: Vec<u8> = secs
                .iter()
                .filter(|s| s.kind == SectionKind::Drop)
                .map(|s| s.label)
                .collect();
            assert_eq!(drops[0], drops[1], "{bpm}: drops {drops:?}");
            let bd = secs
                .iter()
                .find(|s| s.kind == SectionKind::Breakdown)
                .unwrap()
                .label;
            assert_ne!(bd, drops[0]);
        }
    }

    #[test]
    fn tension_peaks_before_the_drop_and_resets() {
        let a = analyze(&synth::standard_track(128.0));
        let secs = sections_for(&a, true);
        let t = tension(&secs, a.feats.len());
        let drop = secs
            .iter()
            .find(|s| s.kind == SectionKind::Drop)
            .unwrap()
            .start_beat;
        assert!(
            t[drop - 8] > 0.8,
            "8 beats before the drop: {}",
            t[drop - 8]
        );
        assert!(t[drop - 8] > t[drop - 24], "rising through the build");
        assert_eq!(t[drop], 0.0, "released at the drop");
        let breakdown = secs
            .iter()
            .find(|s| s.kind == SectionKind::Breakdown)
            .unwrap();
        assert!(
            t[breakdown.start_beat..breakdown.end_beat]
                .iter()
                .all(|&x| x == 0.0)
        );
    }

    #[test]
    fn repeated_section_keeps_its_label_across_calls() {
        let a = analyze(&[
            part(Pattern::Drop, 16, 128.0),
            part(Pattern::Breakdown, 16, 128.0),
            part(Pattern::Drop, 16, 128.0),
        ]);
        let n = a.feats.len();
        let third = n / 3;
        let mut protos = Vec::new();
        let first = label(&a.feats, &[0, third], &mut protos);
        let rest = label(&a.feats, &[third, 2 * third, n], &mut protos);
        assert_eq!(
            first[0], rest[1],
            "second drop reuses the first drop's label"
        );
        assert_ne!(rest[0], first[0]);
    }
}
