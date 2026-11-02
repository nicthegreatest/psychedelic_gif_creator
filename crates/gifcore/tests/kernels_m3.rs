//! Hashes of OpenCV 4.13.0 outputs from `/tmp/m3_probe` (not in the repo):
//! GaussianBlur, cvtColor BGR↔HSV and filled LINE_AA circles on the same
//! pattern M2 used. Star bits are `tests/golden/stars_seed1.txt`, dumped from
//! libstdc++ `mt19937(1)` + `uniform_real_distribution<float>`.

use gifcore::image::{Bgr8, Bgra8};
use gifcore::kernels::aa_circle::circle_filled_aa;
use gifcore::kernels::gaussian::gaussian_blur;
use gifcore::kernels::hsv::{bgr_to_hsv, hsv_to_bgr};
use gifcore::rng::{Mt19937, UniformReal};
use gifcore::settings::StarPattern;

fn fnv(bytes: &[u8]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for b in bytes {
        h = (h ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3);
    }
    h
}

fn pattern_bgr(w: u32, h: u32) -> Bgr8 {
    let mut img = Bgr8::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let px = img.pixel_mut(x, y);
            for (c, slot) in px.iter_mut().enumerate() {
                *slot = ((x * 37 + y * 91 + c as u32 * 53 + x * y * 13) % 256) as u8;
            }
        }
    }
    img
}

#[test]
fn gaussian_matches_opencv_413() {
    let src = pattern_bgr(23, 17);
    let cases = [
        (0.06, 0x51b6_791d_6c51_09e9),
        (0.1, 0x51b6_791d_6c51_09e9),
        (0.5, 0xfeaa_7737_316f_f8d3),
        (1.0, 0x273c_6a2e_5fa8_1bf2),
        (1.7, 0xfbbd_4f7d_afb7_be9d),
        (3.0, 0x8ddb_8cd1_3185_6e82),
        (8.0, 0x0dbe_07bf_5e64_506f),
    ];
    for (sigma, hash) in cases {
        assert_eq!(
            fnv(&gaussian_blur(&src, sigma).data),
            hash,
            "23x17 sigma {sigma}"
        );
    }
    assert_eq!(
        fnv(&gaussian_blur(&pattern_bgr(5, 4), 1.0).data),
        0x062b_6245_0c40_c360,
        "5x4"
    );
    assert_eq!(
        fnv(&gaussian_blur(&pattern_bgr(1, 9), 0.8).data),
        0x29d2_8eb8_7cbc_d2a6,
        "1x9"
    );
    let mut solid = Bgr8::new(6, 6);
    for px in solid.data.as_chunks_mut::<3>().0 {
        *px = [40, 80, 200];
    }
    assert_eq!(
        fnv(&gaussian_blur(&solid, 1.3).data),
        0xd332_e284_991b_dc15,
        "solid"
    );
}

#[test]
fn hsv_matches_opencv_413() {
    let src = pattern_bgr(23, 17);
    let hsv = bgr_to_hsv(&src);
    let back = hsv_to_bgr(&hsv);
    assert_eq!(fnv(&hsv.data), 0x170f_6c3a_0f78_8ef8, "bgr2hsv");
    assert_eq!(fnv(&back.data), 0x481e_9a3e_f6f9_5772, "hsv2bgr");

    // 1×1 pixels. Values are OpenCV's.
    let primaries = [
        ([0, 0, 0], [0, 0, 0], [0, 0, 0]),
        ([255, 0, 0], [120, 255, 255], [255, 0, 0]),
        ([0, 255, 0], [60, 255, 255], [0, 255, 0]),
        ([0, 0, 255], [0, 255, 255], [0, 0, 255]),
        ([255, 255, 255], [0, 0, 255], [255, 255, 255]),
        ([10, 20, 30], [15, 170, 30], [10, 20, 30]),
        ([128, 0, 255], [165, 255, 255], [127, 0, 255]),
        ([1, 2, 3], [15, 170, 3], [1, 2, 3]),
    ];
    for (bgr, hsv_px, back_px) in primaries {
        let mut one = Bgr8::new(1, 1);
        one.pixel_mut(0, 0).copy_from_slice(&bgr);
        let h = bgr_to_hsv(&one);
        let b = hsv_to_bgr(&h);
        assert_eq!(h.pixel(0, 0), &hsv_px, "hsv {bgr:?}");
        assert_eq!(b.pixel(0, 0), &back_px, "back {bgr:?}");
    }
}

#[test]
fn aa_circle_matches_opencv_413() {
    // `cv::Mat(31, 29)` is 29 columns by 31 rows.
    let mut canvas = Bgra8::new(29, 31);
    circle_filled_aa(&mut canvas, (10, 12), 1, [200, 10, 30, 255]);
    circle_filled_aa(&mut canvas, (20, 8), 4, [0, 180, 90, 255]);
    circle_filled_aa(&mut canvas, (4, 24), 7, [255, 255, 255, 255]);
    circle_filled_aa(&mut canvas, (-2, 2), 5, [40, 50, 60, 128]);
    assert_eq!(fnv(&canvas.data), 0x4e5d_b5e2_91d3_7db8);
}

#[test]
fn star_layout_matches_libstdcpp_seed1() {
    let stars = gifcore::effects::stars::layout(300, 1, StarPattern::Random);
    let expected = include_str!("golden/stars_seed1.txt");
    let mut lines = expected.lines();
    let mut rng = Mt19937::new(1);
    let unit = UniformReal::new(0.0, 1.0);
    let bright = UniformReal::new(0.5, 1.0);
    for (i, star) in stars.iter().enumerate() {
        let x = unit.sample(&mut rng);
        let y = unit.sample(&mut rng);
        let phase = unit.sample(&mut rng);
        let br = bright.sample(&mut rng);
        let line = lines.next().unwrap_or_else(|| panic!("short dump at {i}"));
        let text = format!(
            "{:08x} {:08x} {:08x} {:08x}",
            x.to_bits(),
            y.to_bits(),
            phase.to_bits(),
            br.to_bits()
        );
        assert_eq!(line, text, "rng sample {i}");
        assert_eq!(star.x.to_bits(), x.to_bits());
        assert_eq!(star.y.to_bits(), y.to_bits());
        assert_eq!(star.phase.to_bits(), phase.to_bits());
        assert_eq!(star.brightness.to_bits(), br.to_bits());
    }
    assert!(lines.next().is_none());
}
