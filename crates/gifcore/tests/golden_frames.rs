//! Frames from `tools/v2-golden` against `FrameRenderer`.
//!
//! Mean absolute difference ≤ 1.0, and the 99th percentile of absolute
//! difference ≤ 8 on each channel. The first five configs leave the M3
//! effects off. The rest are one effect each, plus "everything on,
//! letterboxed" and "radial 6, 2 turns". Those also have to loop: frame
//! `num_frames` matches frame 0, and that seam stays well below the step
//! from frame 0 to frame 1.

use std::fs;
use std::path::{Path, PathBuf};

use gifcore::image::{Bgr8, Bgra8};
use gifcore::render::FrameRenderer;
use gifcore::schema;
use gifcore::settings::GifSettings;

const CONFIGS: [&str; 5] = [
    "defaults",
    "tunnel 0.7 spin",
    "oscillating 0.3 spin (rounded)",
    "letterboxed",
    "transparent source",
];

const M3_CONFIGS: [&str; 10] = [
    "quadrant mirror",
    "stars random",
    "wave horizontal",
    "pixelate",
    "hue",
    "saturation",
    "invert flashes",
    "blur",
    "everything on, letterboxed",
    "radial 6, 2 turns",
];

fn golden_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn field(meta: &str, key: &str) -> String {
    meta.lines()
        .find_map(|line| {
            let (k, v) = line.split_once(':')?;
            (k.trim() == key).then(|| v.trim().to_string())
        })
        .unwrap_or_else(|| panic!("meta.txt missing {key}"))
}

fn load_bytes(path: &Path, expected: usize) -> Vec<u8> {
    let data = fs::read(path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    assert_eq!(data.len(), expected, "{}", path.display());
    data
}

/// 99th percentile by nearest rank: at least 99% of the samples are ≤ the result.
fn percentile_99(values: &mut [u8]) -> u8 {
    assert!(!values.is_empty());
    values.sort_unstable();
    let rank = (values.len() as f64 * 0.99).ceil() as usize;
    values[rank - 1]
}

struct ChannelStats {
    mean: f64,
    p99_b: u8,
    p99_g: u8,
    p99_r: u8,
}

fn mean_abs(a: &Bgr8, b: &Bgr8) -> f64 {
    let sum: u64 = a
        .data
        .iter()
        .zip(&b.data)
        .map(|(p, q)| u64::from(p.abs_diff(*q)))
        .sum();
    sum as f64 / a.data.len() as f64
}

fn compare(got: &Bgr8, rgba: &[u8]) -> ChannelStats {
    assert_eq!(rgba.len(), got.data.len() / 3 * 4);
    let n = (got.width * got.height) as usize;
    let mut sum = 0u64;
    let mut db = Vec::with_capacity(n);
    let mut dg = Vec::with_capacity(n);
    let mut dr = Vec::with_capacity(n);
    for (pixel, reference) in got
        .data
        .as_chunks::<3>()
        .0
        .iter()
        .zip(rgba.as_chunks::<4>().0)
    {
        // Reference is RGBA; the renderer returns BGR. Alpha is opaque.
        assert_eq!(reference[3], 255);
        let diffs = [
            pixel[0].abs_diff(reference[2]),
            pixel[1].abs_diff(reference[1]),
            pixel[2].abs_diff(reference[0]),
        ];
        db.push(diffs[0]);
        dg.push(diffs[1]);
        dr.push(diffs[2]);
        sum += u64::from(diffs[0]) + u64::from(diffs[1]) + u64::from(diffs[2]);
    }
    ChannelStats {
        mean: sum as f64 / (n * 3) as f64,
        p99_b: percentile_99(&mut db),
        p99_g: percentile_99(&mut dg),
        p99_r: percentile_99(&mut dr),
    }
}

#[test]
fn matches_v2_within_tolerance() {
    let root = golden_root();
    for name in CONFIGS.into_iter().chain(M3_CONFIGS) {
        let m3 = M3_CONFIGS.contains(&name);
        let dir = root.join(name);
        let meta =
            fs::read_to_string(dir.join("meta.txt")).unwrap_or_else(|e| panic!("{name} meta: {e}"));
        let settings_text = fs::read_to_string(dir.join("settings.txt")).unwrap();
        let settings = schema::from_text(&settings_text);
        let src_w: u32 = field(&meta, "source_width").parse().unwrap();
        let src_h: u32 = field(&meta, "source_height").parse().unwrap();
        let width: u32 = field(&meta, "width").parse().unwrap();
        let height: u32 = field(&meta, "height").parse().unwrap();
        let frames: i32 = field(&meta, "frames").parse().unwrap();
        let indices: Vec<i32> = field(&meta, "indices")
            .split_whitespace()
            .map(|s| s.parse().unwrap())
            .collect();
        assert_eq!(
            indices,
            vec![0, frames / 4, frames / 2],
            "{name}: expected frames 0, n/4, n/2"
        );
        assert_eq!(settings.output_width, width as i32, "{name}");
        assert_eq!(settings.output_height, height as i32, "{name}");
        assert_eq!(settings.num_frames, frames, "{name}");

        let source_rel = field(&meta, "source");
        let source_bytes = load_bytes(&dir.join(source_rel), (src_w * src_h * 4) as usize);
        let source = Bgra8::from_raw(src_w, src_h, source_bytes).unwrap();
        let renderer = FrameRenderer::new(&settings, &source);
        assert_eq!(renderer.frame_size(), (width, height));

        for index in indices {
            let reference = load_bytes(
                &dir.join(format!("{index}.rgba")),
                (width * height * 4) as usize,
            );
            let stats = compare(&renderer.render_frame(index), &reference);
            println!(
                "{name} frame {index}: mean {:.4}  p99 B/G/R {}/{}/{}",
                stats.mean, stats.p99_b, stats.p99_g, stats.p99_r
            );
            assert!(
                stats.mean <= 1.0,
                "{name} frame {index}: mean abs diff {:.4} > 1.0",
                stats.mean
            );
            assert!(
                stats.p99_b <= 8 && stats.p99_g <= 8 && stats.p99_r <= 8,
                "{name} frame {index}: p99 B/G/R {}/{}/{} exceeds 8",
                stats.p99_b,
                stats.p99_g,
                stats.p99_r
            );
        }
        if m3 {
            let f0 = renderer.render_frame(0);
            let f1 = renderer.render_frame(1);
            let seam = mean_abs(&f0, &renderer.render_frame(frames));
            let step = mean_abs(&f0, &f1);
            println!("{name} loop: seam {seam:.4}, frame-to-frame {step:.4}");
            assert!(seam < 0.05, "{name}: loop seam too visible ({seam:.4})");
            assert!(
                step > 10.0 * seam || step < 1e-9,
                "{name}: seam ({seam:.4}) not clearly below normal motion ({step:.4})"
            );
        }
    }
}

/// Settings text must actually turn the M3 effects off, or the comparison
/// would be against a different pipeline than the one the harness rendered.
#[test]
fn golden_settings_leave_m3_effects_off() {
    let root = golden_root();
    for name in CONFIGS {
        let text = fs::read_to_string(root.join(name).join("settings.txt")).unwrap();
        let s = schema::from_text(&text);
        assert_m3_off(&s, name);
    }
}

fn assert_m3_off(s: &GifSettings, name: &str) {
    use gifcore::settings::{KaleidoscopeMode, StarPattern, WaveDirection};
    assert_eq!(s.kaleidoscope_mode, KaleidoscopeMode::None, "{name}");
    assert_eq!(s.star_pattern, StarPattern::None, "{name}");
    assert_eq!(s.wave_direction, WaveDirection::None, "{name}");
    assert_eq!(s.hue_cycles, 0, "{name}");
    assert_eq!(s.saturation_pulse, 0.0, "{name}");
    assert_eq!(s.pixelation_level, 0, "{name}");
    assert_eq!(s.blur_radius, 0.0, "{name}");
    assert_eq!(s.invert_flashes, 0, "{name}");
}
