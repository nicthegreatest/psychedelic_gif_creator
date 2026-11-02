use gifcore::image::Bgra8;
use gifcore::kernels::warp::{rotation_matrix, warp_affine, Border};

const IDENTITY: [[f64; 3]; 2] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
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

#[test]
fn output_has_requested_size() {
    let src = pattern(7, 5);
    for (w, h) in [(7, 5), (11, 3), (1, 1), (0, 4)] {
        let d = warp_affine(&src, &IDENTITY, w, h, Border::Replicate);
        assert_eq!(
            (d.width, d.height, d.data.len()),
            (w, h, (w * h * 4) as usize)
        );
    }
}

#[test]
fn identity_gives_identical_pixels() {
    for (w, h) in [(7, 5), (1, 1), (2, 9)] {
        let src = pattern(w, h);
        for b in BORDERS {
            assert_eq!(warp_affine(&src, &IDENTITY, w, h, b), src, "{w}x{h} {b:?}");
        }
    }
}

#[test]
fn uniform_colour_stays_uniform_with_replicate() {
    let px = [12, 34, 56, 78];
    let src = solid(9, 6, px);
    for (angle, scale) in [(33.0, 0.9), (-71.5, 1.7), (180.0, 0.3)] {
        let m = rotation_matrix((4.0, 2.5), angle, scale);
        let d = warp_affine(&src, &m, 13, 8, Border::Replicate);
        assert!(d.data.chunks(4).all(|p| p == px), "{angle} {scale}");
    }
}

#[test]
fn integer_translation_shifts_pixels_exactly() {
    let src = pattern(8, 6);
    let border = [1, 2, 3, 4];
    for (tx, ty) in [(2i32, 1i32), (-3, 2), (0, -4)] {
        let m = [[1.0, 0.0, f64::from(tx)], [0.0, 1.0, f64::from(ty)]];
        let d = warp_affine(&src, &m, 8, 6, Border::Constant(border));
        for y in 0..6i32 {
            for x in 0..8i32 {
                let (sx, sy) = (x - tx, y - ty);
                let expected = if (0..8).contains(&sx) && (0..6).contains(&sy) {
                    src.pixel(sx as u32, sy as u32)
                } else {
                    &border[..]
                };
                assert_eq!(
                    d.pixel(x as u32, y as u32),
                    expected,
                    "t=({tx},{ty}) at ({x},{y})"
                );
            }
        }
    }
}

#[test]
fn rotation_by_90_is_a_pixel_permutation() {
    // getRotationMatrix2D: +90 is counter-clockwise, so dst(x, y) = src(n-1-y, x).
    let n = 5;
    let src = pattern(n, n);
    let c = f64::from(n - 1) / 2.0;
    let d = warp_affine(
        &src,
        &rotation_matrix((c, c), 90.0, 1.0),
        n,
        n,
        Border::Constant([0; 4]),
    );
    for y in 0..n {
        for x in 0..n {
            assert_eq!(d.pixel(x, y), src.pixel(n - 1 - y, x), "({x},{y})");
        }
    }
}

#[test]
fn rotation_by_180_is_a_pixel_permutation() {
    let (w, h) = (6, 4);
    let src = pattern(w, h);
    let center = (f64::from(w - 1) / 2.0, f64::from(h - 1) / 2.0);
    let d = warp_affine(
        &src,
        &rotation_matrix(center, 180.0, 1.0),
        w,
        h,
        Border::Replicate,
    );
    for y in 0..h {
        for x in 0..w {
            assert_eq!(d.pixel(x, y), src.pixel(w - 1 - x, h - 1 - y), "({x},{y})");
        }
    }
}

#[test]
fn constant_border_fills_pixels_outside_the_source() {
    let src = pattern(5, 4);
    let border = [10, 20, 30, 40];
    let m = [[1.0, 0.0, 50.0], [0.0, 1.0, -30.0]];
    let d = warp_affine(&src, &m, 6, 6, Border::Constant(border));
    assert!(d.data.chunks(4).all(|p| p == border));
}

#[test]
fn empty_source_gives_border_colour() {
    let d = warp_affine(
        &Bgra8::new(0, 0),
        &IDENTITY,
        3,
        2,
        Border::Constant([5, 6, 7, 8]),
    );
    assert!(d.data.chunks(4).all(|p| p == [5, 6, 7, 8]));
    let d = warp_affine(&Bgra8::new(0, 0), &IDENTITY, 3, 2, Border::Reflect101);
    assert!(d.data.iter().all(|&v| v == 0));
}

#[test]
fn rotation_matrix_matches_opencv() {
    // cv::getRotationMatrix2D(Point2f(11, 8), 33.0, 0.9) printed by OpenCV 4.13.0.
    let expected = [
        [
            0.754_803_511_150_881_6,
            0.490_175_131_513_524_4,
            -1.224_239_674_767_893,
        ],
        [
            -0.490_175_131_513_524_4,
            0.754_803_511_150_881_6,
            7.353_498_357_441_715,
        ],
    ];
    let m = rotation_matrix((11.0, 8.0), 33.0, 0.9);
    for (row, erow) in m.iter().zip(expected) {
        for (v, e) in row.iter().zip(erow) {
            assert!((v - e).abs() < 1e-12, "{v} vs {e}");
        }
    }
}

/// FNV-1a hashes of `cv::warpAffine(INTER_LINEAR)` output on `pattern(23, 17)`,
/// produced by a scratch C++ program linked against Fedora 44's OpenCV 4.13.0.
#[test]
fn matches_opencv_golden_hashes() {
    let src = pattern(23, 17);
    assert_eq!(fnv(&src), 0x7ef6_3276_d40e_18b1, "pattern");
    // Arbitrary matrix shared with the golden program, not an approximation of 1/sqrt(2).
    #[allow(clippy::approx_constant)]
    let m1 = [[0.7071, -0.6, 3.25], [0.55, 0.81, -2.4]];
    let cases = [
        (
            "W1",
            m1,
            29,
            21,
            Border::Constant([0; 4]),
            0xb7db_fef7_42a0_9181,
        ),
        (
            "W2",
            rotation_matrix((11.0, 8.0), 0.0, 1.37),
            23,
            17,
            Border::Reflect101,
            0x06d7_3eb9_bda1_7fb5,
        ),
        (
            "W3",
            rotation_matrix((11.0, 8.0), 0.0, 0.43),
            23,
            17,
            Border::Reflect101,
            0x1cbb_bd5e_111a_226c,
        ),
        (
            "W4",
            rotation_matrix((11.0, 8.0), 33.0, 0.9),
            23,
            17,
            Border::Replicate,
            0x7c82_5cbb_c5f9_a929,
        ),
        (
            "W5",
            m1,
            29,
            21,
            Border::Constant([10, 20, 30, 40]),
            0x7aef_a23b_b989_49b6,
        ),
    ];
    for (name, m, w, h, b, hash) in cases {
        assert_eq!(fnv(&warp_affine(&src, &m, w, h, b)), hash, "{name}");
    }
}
