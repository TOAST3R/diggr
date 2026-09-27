//! Text formatting for time displays and scrolling titles.

/// `m:ss` (or `h:mm:ss` from one hour on).
pub fn clock(secs: f64) -> String {
    let s = secs.max(0.0).floor() as u64;
    let (h, m, s) = (s / 3600, (s / 60) % 60, s % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// Parses `90`, `1:30` or `1:02:03.5` into seconds.
pub fn parse_clock(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    s.split(':').try_fold(0.0, |acc, part| {
        part.trim()
            .parse::<f64>()
            .ok()
            .filter(|v| *v >= 0.0 && v.is_finite())
            .map(|v| acc * 60.0 + v)
    })
}

/// The main LCD: minutes (at least two digits) and seconds, e.g. `("01", "51")`.
/// Minutes run past 99 rather than wrapping.
pub fn lcd(secs: f64) -> (String, String) {
    let s = secs.max(0.0).floor() as u64;
    (format!("{:02}", s / 60), format!("{:02}", s % 60))
}

/// Winamp's title line: `N. Artist - Title (m:ss)`.
pub fn title_line(number: usize, artist: &str, title: &str, duration: Option<f64>) -> String {
    let name = if artist.is_empty() {
        title.to_owned()
    } else {
        format!("{artist} - {title}")
    };
    match duration {
        Some(d) => format!("{number}. {name} ({})", clock(d)),
        None => format!("{number}. {name}"),
    }
}

/// A window of `width` characters into `text`, scrolled by `offset` characters. Text that fits
/// is returned unchanged; longer text loops with a `  ***  ` separator.
pub fn scroll(text: &str, width: usize, offset: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= width {
        return text.to_owned();
    }
    let looped: Vec<char> = chars.iter().copied().chain("  ***  ".chars()).collect();
    (0..width)
        .map(|i| looped[(offset + i) % looped.len()])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_formats() {
        assert_eq!(clock(0.0), "0:00");
        assert_eq!(clock(111.9), "1:51");
        assert_eq!(clock(3_725.0), "1:02:05");
        assert_eq!(clock(-3.0), "0:00");
    }

    #[test]
    fn parses_clock_times() {
        assert_eq!(parse_clock("90"), Some(90.0));
        assert_eq!(parse_clock(" 1:30 "), Some(90.0));
        assert_eq!(parse_clock("1:02:03.5"), Some(3723.5));
        assert_eq!(parse_clock("x"), None);
        assert_eq!(parse_clock(""), None);
        assert_eq!(parse_clock("-5"), None);
    }

    #[test]
    fn lcd_digits() {
        assert_eq!(lcd(111.0), ("01".into(), "51".into()));
        assert_eq!(lcd(6_000.0), ("100".into(), "00".into()));
    }

    #[test]
    fn title_lines() {
        assert_eq!(
            title_line(4, "Crusher-P", "Echo", Some(230.0)),
            "4. Crusher-P - Echo (3:50)"
        );
        assert_eq!(title_line(1, "", "untitled", None), "1. untitled");
    }

    #[test]
    fn scrolling() {
        assert_eq!(scroll("short", 10, 7), "short");
        assert_eq!(scroll("ABCDEFGH", 4, 0), "ABCD");
        assert_eq!(scroll("ABCDEFGH", 4, 6), "GH  ");
        // Loops back to the start after text + separator (8 + 7 chars).
        assert_eq!(scroll("ABCDEFGH", 4, 15), "ABCD");
    }
}
