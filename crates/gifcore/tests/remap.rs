use gifcore::image::Bgra8;
use gifcore::kernels::remap::{remap, remap_nearest};
use gifcore::kernels::warp::Border;

const BORDERS: [Border; 3] = [
    Border::Constant([9, 8, 7, 6]),
    Border::Replicate,
    Border::Reflect101,
];

fn solid(w: u32, h: u32, px: [u8; 4]) -> Bgra8 {
    Bgra8::from_raw(w, h, px.repeat((w * h) as usize)).unwrap()
}

/// Same pattern as the OpenCV golden generator.
fn pattern(w: u32, h: u32) -> Bgra8 {
    let data = (0..h)
        .flat_map(|y| (0..w).flat_map(move |x| (0..4).map(move |c| (x, y, c))))
        .map(|(x, y, c)| ((x * 37 + y * 91 + c * 53 + x * y * 13) % 256) as u8)
        .collect();
    Bgra8::from_raw(w, h, data).unwrap()
}

/// FNV-1a 64 over the pixel bytes (matches the golden generator).
fn fnv(img: &Bgra8) -> u64 {
    img.data.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Build maps from a per-pixel function `(x, y) -> (map_x, map_y)`.
fn maps(w: u32, h: u32, f: impl Fn(i32, i32) -> (f32, f32)) -> (Vec<f32>, Vec<f32>) {
    (0..h as i32)
        .flat_map(|y| (0..w as i32).map(move |x| (x, y)))
        .map(|(x, y)| f(x, y))
        .unzip()
}

#[test]
fn output_has_requested_size() {
    let src = pattern(7, 5);
    for (w, h) in [(7, 5), (11, 3), (1, 1), (0, 4)] {
        let (mx, my) = maps(w, h, |x, y| (x as f32, y as f32));
        let d = remap(&src, &mx, &my, w, h, Border::Replicate);
        assert_eq!(
            (d.width, d.height, d.data.len()),
            (w, h, (w * h * 4) as usize)
        );
        let d = remap_nearest(&src, &mx, &my, w, h, Border::Replicate);
        assert_eq!((d.width, d.height), (w, h));
    }
}

#[test]
fn identity_map_equals_source() {
    for (w, h) in [(7, 5), (1, 1), (2, 9)] {
        let src = pattern(w, h);
        let (mx, my) = maps(w, h, |x, y| (x as f32, y as f32));
        for b in BORDERS {
            assert_eq!(remap(&src, &mx, &my, w, h, b), src, "linear {w}x{h} {b:?}");
            assert_eq!(
                remap_nearest(&src, &mx, &my, w, h, b),
                src,
                "nearest {w}x{h} {b:?}"
            );
        }
    }
}

#[test]
fn uniform_colour_stays_uniform_with_replicate() {
    let px = [12, 34, 56, 78];
    let src = solid(9, 6, px);
    let (mx, my) = maps(13, 8, |x, y| {
        (x as f32 * 1.37 - 2.2, y as f32 * 0.61 + (x as f32) * 0.3)
    });
    let d = remap(&src, &mx, &my, 13, 8, Border::Replicate);
    assert!(d.data.chunks(4).all(|p| p == px));
    let d = remap_nearest(&src, &mx, &my, 13, 8, Border::Replicate);
    assert!(d.data.chunks(4).all(|p| p == px));
}

#[test]
fn integer_shift_map_shifts_pixels_exactly() {
    let src = pattern(8, 6);
    let border = [1, 2, 3, 4];
    let (mx, my) = maps(8, 6, |x, y| ((x - 2) as f32, (y + 1) as f32));
    for d in [
        remap(&src, &mx, &my, 8, 6, Border::Constant(border)),
        remap_nearest(&src, &mx, &my, 8, 6, Border::Constant(border)),
    ] {
        for y in 0..6i32 {
            for x in 0..8i32 {
                let (sx, sy) = (x - 2, y + 1);
                let expected = if (0..8).contains(&sx) && (0..6).contains(&sy) {
                    src.pixel(sx as u32, sy as u32)
                } else {
                    &border[..]
                };
                assert_eq!(d.pixel(x as u32, y as u32), expected, "({x},{y})");
            }
        }
    }
}

#[test]
fn constant_border_fills_pixels_outside_the_source() {
    let src = pattern(5, 4);
    let border = [10, 20, 30, 40];
    let (mx, my) = maps(6, 3, |x, y| (x as f32 + 40.5, y as f32 - 20.25));
    let d = remap(&src, &mx, &my, 6, 3, Border::Constant(border));
    assert!(d.data.chunks(4).all(|p| p == border));
    let d = remap_nearest(&src, &mx, &my, 6, 3, Border::Constant(border));
    assert!(d.data.chunks(4).all(|p| p == border));
}

#[test]
fn quadrant_mirror_map_mirrors_top_left() {
    // v2's QuadrantMirror map with nearest sampling.
    let (w, h) = (7, 6);
    let src = pattern(w, h);
    let (wi, hi) = (w as i32, h as i32);
    let (mx, my) = maps(w, h, |x, y| {
        (x.min(wi - 1 - x) as f32, y.min(hi - 1 - y) as f32)
    });
    let d = remap_nearest(&src, &mx, &my, w, h, Border::Reflect101);
    for y in 0..h {
        for x in 0..w {
            assert_eq!(d.pixel(x, y), src.pixel(x.min(w - 1 - x), y.min(h - 1 - y)));
        }
    }
}

#[test]
fn empty_source_gives_border_colour() {
    let (mx, my) = maps(3, 2, |x, y| (x as f32, y as f32));
    let d = remap(
        &Bgra8::new(0, 0),
        &mx,
        &my,
        3,
        2,
        Border::Constant([5, 6, 7, 8]),
    );
    assert!(d.data.chunks(4).all(|p| p == [5, 6, 7, 8]));
    let d = remap_nearest(&Bgra8::new(0, 0), &mx, &my, 3, 2, Border::Replicate);
    assert!(d.data.iter().all(|&v| v == 0));
}

#[test]
#[should_panic(expected = "map_x")]
fn wrong_map_length_panics() {
    let src = pattern(3, 3);
    remap(&src, &[0.0; 5], &[0.0; 6], 3, 2, Border::Replicate);
}

/// FNV-1a hashes of `cv::remap` output (two `CV_32FC1` maps) on
/// `pattern(23, 17)`, produced by a scratch C++ program linked against
/// Fedora 44's OpenCV 4.13.0. The /64 and /2 maps hit exact rounding ties.
#[test]
fn matches_opencv_golden_hashes() {
    let src = pattern(23, 17);
    assert_eq!(fnv(&src), 0x7ef6_3276_d40e_18b1, "pattern");
    let (w, h) = (19, 13);
    let ties = maps(w, h, |x, y| {
        (
            (x * 29 + y * 7 - 150) as f32 / 64.0,
            (x * 5 - y * 31 + 400) as f32 / 64.0,
        )
    });
    let sevenths = maps(w, h, |x, y| {
        (
            (x * 29 + y * 7 - 150) as f32 / 7.0,
            (x * 5 - y * 31 + 400) as f32 / 7.0,
        )
    });
    let halves = maps(w, h, |x, y| {
        ((x * 13 - 40) as f32 / 2.0, (y * 9 - 30) as f32 / 2.0)
    });
    let c = Border::Constant([5, 6, 7, 8]);

    let linear = [
        ("R1", &ties, Border::Reflect101, 0x7694_de20_4b18_4f1c),
        ("R4", &ties, c, 0x4a7f_dcf4_c099_fe55),
        ("R2", &sevenths, Border::Replicate, 0x027c_f0db_5ca7_c286),
    ];
    for (name, (mx, my), b, hash) in linear {
        assert_eq!(fnv(&remap(&src, mx, my, w, h, b)), hash, "{name}");
    }
    let nearest = [
        ("R3", Border::Reflect101, 0xedd8_4c00_461b_cc19),
        ("R5", c, 0x1efc_bd1d_165a_cc09),
        ("R6", Border::Replicate, 0x8eef_bc61_744d_c6ed),
    ];
    let (mx, my) = &halves;
    for (name, b, hash) in nearest {
        assert_eq!(fnv(&remap_nearest(&src, mx, my, w, h, b)), hash, "{name}");
    }
}
