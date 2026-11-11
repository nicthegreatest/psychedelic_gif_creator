//! M4 export engine: container round trip, cancel, palette, delays, thread
//! determinism and byte parity with v2's gif-h on v2's own golden frames.

use std::fs;
use std::io::Cursor;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use gifcore::export::{
    bgr_to_rgb, export_gif, frame_delays, quantize, DelayMode, Encoder, ExportError, ExportOptions,
    Palette, MIN_DELAY_CS,
};
use gifcore::image::Bgra8;
use gifcore::render::FrameRenderer;
use gifcore::settings::GifSettings;
use image::codecs::gif::GifDecoder;
use image::AnimationDecoder;

/// Per-frame mean absolute difference (all channels, 0..255) allowed between
/// a decoded frame and `render_frame`. This is gif-h's 256-colour median-cut
/// quantisation error, not a decoding error (frame 0 is checked exactly
/// against the encoder's own mapping). The test image has a noisy red channel
/// (`x * y % 256`); measured at M4: 2.8..4.7 without dither, 3.4..5.8 with
/// (dither trades per-pixel error for smoother gradients).
const QUANT_TOLERANCE: f64 = 8.0;

fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("gifcore-export-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn gradient(w: u32, h: u32) -> Bgra8 {
    let mut img = Bgra8::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let px = img.pixel_mut(x, y);
            px[0] = ((255 * x) / w) as u8;
            px[1] = ((255 * y) / h) as u8;
            px[2] = ((x * y) % 256) as u8;
            px[3] = 255;
        }
    }
    img
}

fn small_settings(dither: bool) -> GifSettings {
    GifSettings {
        output_width: 64,
        output_height: 48,
        num_frames: 10,
        fps: 12,
        dither,
        ..GifSettings::default()
    }
}

fn renderer(settings: &GifSettings) -> FrameRenderer {
    FrameRenderer::new(settings, &gradient(160, 100))
}

fn dir_entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3)
    })
}

#[test]
fn export_round_trips_through_a_gif_decoder() {
    for dither in [false, true] {
        let settings = small_settings(dither);
        let r = renderer(&settings);
        let dir = scratch_dir(&format!("roundtrip-{dither}"));
        let path = dir.join("out.gif");
        let mut calls = Vec::new();
        let report = export_gif(&r, &ExportOptions::default(), &path, |done, total| {
            calls.push((done, total));
            ControlFlow::Continue(())
        })
        .unwrap();

        let n = settings.num_frames as usize;
        assert_eq!(report.frames, n);
        assert_eq!(calls, (1..=n).map(|d| (d, n)).collect::<Vec<_>>());
        assert_eq!(
            dir_entries(&dir),
            vec!["out.gif".to_string()],
            "no temp file left behind"
        );

        let bytes = fs::read(&path).unwrap();
        assert_eq!(report.bytes, bytes.len() as u64);
        assert!(bytes.starts_with(b"GIF89a"));
        assert!(
            bytes
                .windows(16)
                .any(|w| w == b"NETSCAPE2.0\x03\x01\x00\x00\x00"),
            "infinite loop block present"
        );
        assert_eq!(bytes.last(), Some(&0x3b));

        let delays = frame_delays(settings.fps, n, DelayMode::Accumulated);
        assert_eq!(
            report.total_delay_cs,
            delays.iter().map(|&d| u32::from(d)).sum::<u32>()
        );

        let frames = GifDecoder::new(Cursor::new(bytes))
            .unwrap()
            .into_frames()
            .collect_frames()
            .unwrap();
        assert_eq!(frames.len(), n);

        // The decoder must show exactly what the encoder mapped (frame 0 has
        // no delta, so it can be rebuilt from the public pieces).
        let rgb0 = bgr_to_rgb(&r.render_frame(0));
        let pal0 = Palette::build(&rgb0, dither);
        let mapped0 = if dither {
            quantize::dither(None, &rgb0, 64, &pal0)
        } else {
            quantize::threshold(None, &rgb0, &pal0)
        };
        let decoded0: Vec<[u8; 3]> = frames[0]
            .buffer()
            .pixels()
            .map(|p| [p.0[0], p.0[1], p.0[2]])
            .collect();
        assert_eq!(decoded0, mapped0.shown, "dither={dither}: decoded frame 0");
        for (i, (frame, &delay)) in frames.iter().zip(&delays).enumerate() {
            let (num, den) = frame.delay().numer_denom_ms();
            assert_eq!(num, u32::from(delay) * 10 * den, "frame {i} delay");
            let buf = frame.buffer();
            assert_eq!(buf.dimensions(), (64, 48));
            let want = r.render_frame(i as i32);
            let sum: u64 = buf
                .as_raw()
                .as_chunks::<4>()
                .0
                .iter()
                .zip(want.data.as_chunks::<3>().0)
                .map(|(got, bgr)| {
                    (0..3)
                        .map(|c| u64::from(got[c].abs_diff(bgr[2 - c])))
                        .sum::<u64>()
                })
                .sum();
            let mean = sum as f64 / (64.0 * 48.0 * 3.0);
            assert!(
                mean <= QUANT_TOLERANCE,
                "dither={dither} frame {i}: mean diff {mean}"
            );
        }
    }
}

#[test]
fn cancel_midway_leaves_no_file() {
    let r = renderer(&small_settings(false));
    let dir = scratch_dir("cancel");
    let path = dir.join("out.gif");
    let mut last = 0;
    let err = export_gif(
        &r,
        &ExportOptions {
            threads: 3,
            ..Default::default()
        },
        &path,
        |done, _| {
            last = done;
            if done == 3 {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        },
    )
    .unwrap_err();
    assert!(matches!(err, ExportError::Cancelled), "{err:?}");
    assert_eq!(err.to_string(), "GIF generation cancelled.");
    assert_eq!(last, 3);
    assert!(!path.exists());
    assert!(
        dir_entries(&dir).is_empty(),
        "left: {:?}",
        dir_entries(&dir)
    );
}

#[test]
fn cancel_keeps_an_existing_file_untouched() {
    let r = renderer(&small_settings(false));
    let dir = scratch_dir("cancel-existing");
    let path = dir.join("out.gif");
    fs::write(&path, b"keep me").unwrap();
    let err = export_gif(&r, &ExportOptions::default(), &path, |_, _| {
        ControlFlow::Break(())
    })
    .unwrap_err();
    assert!(matches!(err, ExportError::Cancelled));
    assert_eq!(fs::read(&path).unwrap(), b"keep me");
    assert_eq!(dir_entries(&dir), vec!["out.gif".to_string()]);
}

#[test]
fn unwritable_target_fails_cleanly() {
    let r = renderer(&small_settings(false));
    let dir = scratch_dir("missing-dir");
    let path = dir.join("no such dir").join("out.gif");
    let err = export_gif(&r, &ExportOptions::default(), &path, |_, _| {
        ControlFlow::Continue(())
    })
    .unwrap_err();
    assert!(matches!(err, ExportError::Create(_)), "{err:?}");
    assert_eq!(
        err.to_string(),
        "Could not open the output file for writing."
    );
    assert!(dir_entries(&dir).is_empty());
}

#[test]
fn uniform_colour_hits_the_palette_exactly() {
    let colour = [200u8, 100, 50];
    let pixels = vec![colour; 40 * 30];
    for dither in [false, true] {
        let pal = Palette::build(&pixels, dither);
        let (ind, diff) = pal.closest(colour.map(i32::from), (1, 1_000_000));
        assert_ne!(ind, 0, "never the transparent index");
        assert_eq!(diff, 0);
        assert_eq!(pal.colors[usize::from(ind)], colour);
        assert_eq!(pal.colors[0], [0, 0, 0]);

        let mapped = if dither {
            quantize::dither(None, &pixels, 40, &pal)
        } else {
            quantize::threshold(None, &pixels, &pal)
        };
        assert!(mapped.shown.iter().all(|&p| p == colour));
        assert!(mapped.indices.iter().all(|&i| i == ind));

        // A repeat of the same frame is all transparent (delta encoding).
        let again = quantize::threshold(Some(&mapped.shown), &pixels, &pal);
        assert!(again.indices.iter().all(|&i| i == 0));
        assert!(again.shown.iter().all(|&p| p == colour));
    }
}

#[test]
fn uniform_frames_decode_to_the_same_colour() {
    let colour = [17u8, 230, 99];
    let pixels = vec![colour; 8 * 6];
    let mut enc = Encoder::new(Vec::new(), 8, 6, false).unwrap();
    enc.add_frame(&pixels, 10).unwrap();
    enc.add_frame(&pixels, 10).unwrap();
    let bytes = enc.finish().unwrap();
    let frames = GifDecoder::new(Cursor::new(bytes))
        .unwrap()
        .into_frames()
        .collect_frames()
        .unwrap();
    assert_eq!(frames.len(), 2);
    for f in &frames {
        assert!(f
            .buffer()
            .pixels()
            .all(|p| p.0 == [colour[0], colour[1], colour[2], 255]));
    }
}

#[test]
fn delays_track_the_requested_rate() {
    for fps in 2..=50 {
        let ideal = 100.0 / f64::from(fps);
        for n in [1usize, 2, 3, 7, 10, 24, 36, 100, 240] {
            let delays = frame_delays(fps, n, DelayMode::Accumulated);
            assert_eq!(delays.len(), n);
            assert!(delays.iter().all(|&d| d >= MIN_DELAY_CS));
            assert!(delays
                .iter()
                .all(|&d| f64::from(d) >= ideal.floor() && f64::from(d) <= ideal.ceil()));
            // After every frame the elapsed time is within half a centisecond of ideal.
            let mut elapsed = 0u32;
            for (k, &d) in delays.iter().enumerate() {
                elapsed += u32::from(d);
                let target = 100.0 * (k + 1) as f64 / f64::from(fps);
                assert!(
                    (f64::from(elapsed) - target).abs() <= 0.5,
                    "fps {fps} n {n} k {k}"
                );
            }
            if 100 % fps == 0 {
                assert!(delays.iter().all(|&d| u32::from(d) == 100 / fps as u32));
            }
        }
    }
}

#[test]
fn constant_delays_match_v2() {
    for fps in [-3, 0, 1, 2, 3, 7, 12, 25, 30, 40, 50, 60, 200] {
        let settings = GifSettings {
            fps,
            ..GifSettings::default()
        };
        let want = settings.frame_delay_cs() as u16;
        assert_eq!(
            frame_delays(fps, 5, DelayMode::Constant),
            vec![want; 5],
            "fps {fps}"
        );
    }
}

#[test]
fn delays_never_drop_below_the_minimum() {
    for fps in [51, 60, 100, 1000] {
        assert!(frame_delays(fps, 30, DelayMode::Accumulated)
            .iter()
            .all(|&d| d >= MIN_DELAY_CS));
    }
}

#[test]
fn output_is_identical_for_any_thread_count() {
    for dither in [false, true] {
        let r = renderer(&small_settings(dither));
        let dir = scratch_dir(&format!("threads-{dither}"));
        let read = |threads: usize| {
            let path = dir.join(format!("t{threads}.gif"));
            export_gif(
                &r,
                &ExportOptions {
                    threads,
                    ..Default::default()
                },
                &path,
                |_, _| ControlFlow::Continue(()),
            )
            .unwrap();
            fs::read(path).unwrap()
        };
        let one = read(1);
        assert_eq!(one, read(3));
        assert_eq!(one, read(8));
    }
}

/// v2's gif-h fed v2's own golden RGBA frames. Expected hashes and lengths
/// come from `tools/v2-golden/gif_hash.cpp` (see its header for the commands).
#[test]
fn encoder_is_byte_identical_to_v2_gif_h() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let cases = [
        (
            "defaults",
            160u16,
            120u16,
            false,
            32881usize,
            0xbcdd_005c_1cef_166bu64,
        ),
        ("defaults", 160, 120, true, 40428, 0xfc73_7b1f_8ce7_6af9),
        (
            "everything on, letterboxed",
            64,
            64,
            false,
            11736,
            0x5255_3478_4263_4478,
        ),
        (
            "everything on, letterboxed",
            64,
            64,
            true,
            12504,
            0x1b32_265b_68ff_4e01,
        ),
    ];
    for (name, w, h, dither, len, hash) in cases {
        let mut enc = Encoder::new(Vec::new(), w, h, dither).unwrap();
        for index in ["0", "6", "12"] {
            let rgba = fs::read(root.join(name).join(format!("{index}.rgba"))).unwrap();
            let rgb: Vec<[u8; 3]> = rgba
                .as_chunks::<4>()
                .0
                .iter()
                .map(|&[r, g, b, _]| [r, g, b])
                .collect();
            enc.add_frame(&rgb, 8).unwrap();
        }
        let bytes = enc.finish().unwrap();
        assert_eq!(
            (bytes.len(), fnv1a(&bytes)),
            (len, hash),
            "{name} dither={dither}"
        );
    }
}

#[test]
fn bgr_to_rgb_swaps_channels() {
    let frame = gifcore::image::Bgr8::from_raw(2, 1, vec![1, 2, 3, 4, 5, 6]).unwrap();
    assert_eq!(bgr_to_rgb(&frame), vec![[3, 2, 1], [6, 5, 4]]);
}
