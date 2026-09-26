//! The EQ response preview: a smooth curve through the ten band values.

use audio::eq::{BAND_COUNT, MAX_DB};

/// `width` curve values in dB (−12..=12), Catmull-Rom interpolated through the bands, which
/// sit at evenly spaced x positions (the first at x = 0, the last at x = width − 1).
pub fn curve(bands_db: &[f32; BAND_COUNT], width: usize) -> Vec<f32> {
    if width < 2 {
        return vec![bands_db[0]; width];
    }
    let at = |i: isize| bands_db[i.clamp(0, BAND_COUNT as isize - 1) as usize];
    (0..width)
        .map(|x| {
            let t = x as f32 / (width - 1) as f32 * (BAND_COUNT - 1) as f32;
            let i = (t.floor() as isize).min(BAND_COUNT as isize - 2);
            let u = t - i as f32;
            let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
            let v = 0.5
                * (2.0 * p1
                    + (p2 - p0) * u
                    + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * u * u
                    + (3.0 * p1 - p0 - 3.0 * p2 + p3) * u * u * u);
            v.clamp(-MAX_DB, MAX_DB)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_is_flat() {
        assert!(curve(&[0.0; 10], 113).iter().all(|&v| v == 0.0));
    }

    #[test]
    fn passes_through_every_band() {
        let bands = [6.0, -3.0, 0.0, 12.0, -12.0, 4.5, 2.0, -1.0, 8.0, -6.0];
        let width = 10 * 12 + 1 - 12; // 109: bands land exactly on integer x
        let c = curve(&bands, width);
        for (i, b) in bands.iter().enumerate() {
            let x = i * (width - 1) / 9;
            assert!((c[x] - b).abs() < 1e-4, "band {i}: {} vs {b}", c[x]);
        }
        assert!(c.iter().all(|v| v.abs() <= 12.0), "overshoot is clamped");
    }
}
