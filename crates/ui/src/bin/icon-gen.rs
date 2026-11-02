//! Regenerates the app and extension icons from `assets/icon/logo-mark.svg`:
//! `cargo run -p ui --bin icon-gen`. The PNGs are committed so neither the build nor the app
//! needs an SVG renderer; a test keeps them in sync with this code.

use image::RgbaImage;
use std::path::{Path, PathBuf};

const MARK: &str = include_str!("../../../../assets/icon/logo-mark.svg");
const CREAM: &str = "#F5F5EC";

/// One PNG: its path from the repository root, its size, and the badge layout as fractions of
/// the canvas (badge side, corner radius) and of the badge (mark height).
struct Icon {
    path: &'static str,
    size: u32,
    badge: f32,
    radius: f32,
    mark: f32,
}

const ICONS: [Icon; 6] = [
    // macOS grid: an 824 px rounded square centred in 1024, corner radius ~185 px.
    Icon {
        path: "assets/icon/icon-1024.png",
        size: 1024,
        badge: 824.0 / 1024.0,
        radius: 185.0 / 1024.0,
        mark: 0.7,
    },
    // The Dock icon of development builds: the Dock's largest size, cheap to decode at launch.
    Icon {
        path: "assets/icon/icon-256.png",
        size: 256,
        badge: 824.0 / 1024.0,
        radius: 185.0 / 1024.0,
        mark: 0.7,
    },
    // Toolbar icons fill their square so the D stays legible at 16 px.
    Icon {
        path: "extensions/chrome/icons/icon16.png",
        size: 16,
        badge: 1.0,
        radius: 0.2,
        mark: 0.85,
    },
    Icon {
        path: "extensions/chrome/icons/icon32.png",
        size: 32,
        badge: 1.0,
        radius: 0.2,
        mark: 0.8,
    },
    Icon {
        path: "extensions/chrome/icons/icon48.png",
        size: 48,
        badge: 1.0,
        radius: 0.2,
        mark: 0.8,
    },
    Icon {
        path: "extensions/chrome/icons/icon128.png",
        size: 128,
        badge: 1.0,
        radius: 0.2,
        mark: 0.75,
    },
];

/// The Windows icon, `diggr.exe`'s in Explorer, the taskbar and the Start menu: the toolbar
/// layout (the mark fills its square) at the sizes Windows asks for.
const ICO_PATH: &str = "assets/icon/diggr.ico";
const ICO_FRAMES: [Icon; 4] = [
    Icon {
        path: ICO_PATH,
        size: 16,
        badge: 1.0,
        radius: 0.2,
        mark: 0.85,
    },
    Icon {
        path: ICO_PATH,
        size: 32,
        badge: 1.0,
        radius: 0.2,
        mark: 0.8,
    },
    Icon {
        path: ICO_PATH,
        size: 48,
        badge: 1.0,
        radius: 0.2,
        mark: 0.8,
    },
    Icon {
        path: ICO_PATH,
        size: 256,
        badge: 1.0,
        radius: 0.2,
        mark: 0.75,
    },
];

/// An `.ico` holding each image as a PNG (read by Windows since Vista), 256 px as size 0.
fn ico(frames: &[RgbaImage]) -> Vec<u8> {
    let pngs: Vec<Vec<u8>> = frames
        .iter()
        .map(|f| {
            let mut png = Vec::new();
            f.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
                .expect("encode png");
            png
        })
        .collect();
    let mut out = vec![0, 0, 1, 0];
    out.extend((frames.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * frames.len();
    for (f, png) in frames.iter().zip(&pngs) {
        let side = if f.width() >= 256 { 0 } else { f.width() as u8 };
        out.extend([side, side, 0, 0]);
        out.extend(1u16.to_le_bytes());
        out.extend(32u16.to_le_bytes());
        out.extend((png.len() as u32).to_le_bytes());
        out.extend((offset as u32).to_le_bytes());
        offset += png.len();
    }
    pngs.into_iter().for_each(|png| out.extend(png));
    out
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn render(icon: &Icon) -> RgbaImage {
    use resvg::{tiny_skia, usvg};
    let s = icon.size as f32;
    let side = s * icon.badge;
    let inset = (s - side) / 2.0;
    let badge = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{s}" height="{s}"><rect x="{inset}" y="{inset}" width="{side}" height="{side}" rx="{r}" fill="{CREAM}"/></svg>"#,
        r = s * icon.radius,
    );
    let opts = usvg::Options::default();
    let badge = usvg::Tree::from_str(&badge, &opts).expect("badge svg");
    let mark = usvg::Tree::from_str(MARK, &opts).expect("logo-mark.svg");
    let mut pixmap = tiny_skia::Pixmap::new(icon.size, icon.size).expect("pixmap");
    resvg::render(
        &badge,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    let size = mark.size();
    let scale = side * icon.mark / size.height();
    let (w, h) = (size.width() * scale, size.height() * scale);
    let at =
        tiny_skia::Transform::from_scale(scale, scale).post_translate((s - w) / 2.0, (s - h) / 2.0);
    resvg::render(&mark, at, &mut pixmap.as_mut());
    let rgba = pixmap.pixels().iter().flat_map(|p| {
        let c = p.demultiply();
        [c.red(), c.green(), c.blue(), c.alpha()]
    });
    RgbaImage::from_raw(icon.size, icon.size, rgba.collect()).expect("pixel count")
}

fn main() {
    for icon in &ICONS {
        let path = root().join(icon.path);
        render(icon).save(&path).expect("write png");
        println!("wrote {}", path.display());
    }
    let frames: Vec<_> = ICO_FRAMES.iter().map(render).collect();
    let path = root().join(ICO_PATH);
    std::fs::write(&path, ico(&frames)).expect("write ico");
    println!("wrote {}", path.display());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Anti-aliasing may differ by a step between CPUs (SIMD paths), never by more.
    fn same(committed: &RgbaImage, made: &RgbaImage, what: &str) {
        assert_eq!(committed.dimensions(), made.dimensions(), "{what}");
        let off = committed
            .as_raw()
            .iter()
            .zip(made.as_raw())
            .any(|(a, b)| a.abs_diff(*b) > 2);
        assert!(
            !off,
            "{what} is stale: run `cargo run -p ui --bin icon-gen`"
        );
    }

    #[test]
    fn committed_icons_match_the_generator() {
        for icon in &ICONS {
            let committed = image::open(root().join(icon.path))
                .expect(icon.path)
                .to_rgba8();
            same(&committed, &render(icon), icon.path);
        }
    }

    #[test]
    fn the_committed_windows_icon_matches_the_generator() {
        let bytes = std::fs::read(root().join(ICO_PATH)).expect(ICO_PATH);
        assert_eq!(bytes[..4], [0, 0, 1, 0], "not an icon file");
        let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
        assert_eq!(count, ICO_FRAMES.len(), "{ICO_PATH} has the wrong sizes");
        for (i, icon) in ICO_FRAMES.iter().enumerate() {
            let entry = &bytes[6 + 16 * i..22 + 16 * i];
            let size = if entry[0] == 0 { 256 } else { entry[0] as u32 };
            assert_eq!(size, icon.size, "{ICO_PATH} entry {i}");
            let len = u32::from_le_bytes(entry[8..12].try_into().unwrap()) as usize;
            let at = u32::from_le_bytes(entry[12..16].try_into().unwrap()) as usize;
            let committed =
                image::load_from_memory_with_format(&bytes[at..at + len], image::ImageFormat::Png)
                    .expect("png entry")
                    .to_rgba8();
            same(&committed, &render(icon), ICO_PATH);
        }
    }
}
