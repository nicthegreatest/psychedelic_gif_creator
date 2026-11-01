//! Loader tests (port of v2 render_tests.cpp "[loader]" plus EXIF, alpha and error cases).
//! Fixtures are written at runtime under Cargo's integration-test temp dir.

use std::fs;
use std::path::{Path, PathBuf};

use gifcore::loader::{load_image, normalise_to_bgra8, LoadError};
use image::codecs::gif::GifEncoder;
use image::codecs::png::PngEncoder;
use image::{
    DynamicImage, Frame, GrayAlphaImage, GrayImage, ImageBuffer, ImageEncoder, Luma, LumaA, Rgb,
    Rgb32FImage, Rgba, Rgba32FImage, RgbaImage,
};

/// Fresh, empty directory for one test (tests run in parallel).
fn temp_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("loader")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn rgb16_png_divides_by_257() {
    let dir = temp_dir("rgb16");
    let path = dir.join("rgb16.png");
    // Exact multiples of 257, plus the rounding edges 128/257 -> 0 and 129/257 -> 1.
    let img = ImageBuffer::<Rgb<u16>, Vec<u16>>::from_raw(
        2,
        1,
        vec![10 * 257, 20 * 257, 65535, 128, 129, 200 * 257],
    )
    .unwrap();
    img.save(&path).unwrap();

    let out = load_image(&path).unwrap();
    assert_eq!((out.width, out.height), (2, 1));
    assert_eq!(out.pixel(0, 0), &[255, 20, 10, 255]); // BGRA
    assert_eq!(out.pixel(1, 0), &[200, 1, 0, 255]);
}

#[test]
fn grey_png_expands_to_bgra() {
    let dir = temp_dir("grey");
    let path = dir.join("grey.png");
    GrayImage::from_raw(2, 1, vec![0, 77])
        .unwrap()
        .save(&path)
        .unwrap();

    let out = load_image(&path).unwrap();
    assert_eq!(out.pixel(0, 0), &[0, 0, 0, 255]);
    assert_eq!(out.pixel(1, 0), &[77, 77, 77, 255]);
}

#[test]
fn grey_alpha_png_keeps_alpha() {
    let dir = temp_dir("grey_alpha");
    let path = dir.join("ga.png");
    GrayAlphaImage::from_raw(2, 1, vec![50, 0, 200, 128])
        .unwrap()
        .save(&path)
        .unwrap();

    let out = load_image(&path).unwrap();
    assert_eq!(out.pixel(0, 0), &[50, 50, 50, 0]);
    assert_eq!(out.pixel(1, 0), &[200, 200, 200, 128]);
}

#[test]
fn rgba_png_alpha_is_straight() {
    let dir = temp_dir("rgba");
    let path = dir.join("rgba.png");
    RgbaImage::from_raw(1, 1, vec![10, 20, 30, 40])
        .unwrap()
        .save(&path)
        .unwrap();

    // Not premultiplied: colour stays as stored, channels swapped to BGRA.
    assert_eq!(load_image(&path).unwrap().pixel(0, 0), &[30, 20, 10, 40]);
}

#[test]
fn float_tiff_in_unit_range_scales_by_255() {
    let dir = temp_dir("float_tiff");
    let path = dir.join("f32.tiff");
    Rgb32FImage::from_raw(1, 1, vec![0.0, 0.5, 1.0])
        .unwrap()
        .save(&path)
        .unwrap();

    // 0.5 * 255 = 127.5 rounds half to even (OpenCV cvRound) -> 128.
    assert_eq!(load_image(&path).unwrap().pixel(0, 0), &[255, 128, 0, 255]);
}

#[test]
fn float_out_of_range_is_min_max_stretched() {
    // Range -1..3 over all channels, alpha included (v2 uses minMaxLoc on reshape(1)).
    let img = DynamicImage::ImageRgba32F(
        Rgba32FImage::from_raw(2, 1, vec![-1.0, 1.0, 3.0, 1.0, 3.0, -1.0, 0.0, 3.0]).unwrap(),
    );
    let out = normalise_to_bgra8(&img).unwrap();
    assert_eq!(out.pixel(0, 0), &[255, 128, 0, 128]);
    assert_eq!(out.pixel(1, 0), &[64, 0, 255, 255]);
}

#[test]
fn constant_float_image_does_not_divide_by_zero() {
    let img = DynamicImage::ImageRgb32F(Rgb32FImage::from_raw(1, 1, vec![5.0; 3]).unwrap());
    // Range 0 -> treated as 1: (5 - 5) * 255 = 0.
    assert_eq!(
        normalise_to_bgra8(&img).unwrap().pixel(0, 0),
        &[0, 0, 0, 255]
    );
}

#[test]
fn sixteen_bit_grey_alpha_normalises() {
    let img = DynamicImage::ImageLumaA16(
        ImageBuffer::<LumaA<u16>, Vec<u16>>::from_raw(1, 1, vec![257 * 9, 65535]).unwrap(),
    );
    assert_eq!(
        normalise_to_bgra8(&img).unwrap().pixel(0, 0),
        &[9, 9, 9, 255]
    );
    let grey16 = DynamicImage::ImageLuma16(
        ImageBuffer::<Luma<u16>, Vec<u16>>::from_raw(1, 1, vec![257 * 3]).unwrap(),
    );
    assert_eq!(
        normalise_to_bgra8(&grey16).unwrap().pixel(0, 0),
        &[3, 3, 3, 255]
    );
}

/// Big-endian EXIF (TIFF) block holding only an Orientation tag.
fn exif_orientation(value: u16) -> Vec<u8> {
    let mut e = vec![b'M', b'M', 0, 42, 0, 0, 0, 8]; // header, IFD at offset 8
    e.extend_from_slice(&1u16.to_be_bytes()); // one entry
    e.extend_from_slice(&0x0112u16.to_be_bytes()); // Orientation
    e.extend_from_slice(&3u16.to_be_bytes()); // SHORT
    e.extend_from_slice(&1u32.to_be_bytes()); // count
    e.extend_from_slice(&value.to_be_bytes());
    e.extend_from_slice(&[0, 0]); // value padding
    e.extend_from_slice(&0u32.to_be_bytes()); // no next IFD
    e
}

#[test]
fn exif_orientation_is_applied() {
    let dir = temp_dir("exif");
    let path = dir.join("rotated.png");
    // 2×1: red, blue. Orientation 6 = rotate 90° clockwise -> 1×2: red on top, blue below.
    let mut bytes = Vec::new();
    let mut enc = PngEncoder::new(&mut bytes);
    enc.set_exif_metadata(exif_orientation(6)).unwrap();
    enc.write_image(
        &[255, 0, 0, 0, 0, 255],
        2,
        1,
        image::ExtendedColorType::Rgb8,
    )
    .unwrap();
    fs::write(&path, bytes).unwrap();

    let out = load_image(&path).unwrap();
    assert_eq!((out.width, out.height), (1, 2));
    assert_eq!(out.pixel(0, 0), &[0, 0, 255, 255]); // red in BGRA
    assert_eq!(out.pixel(0, 1), &[255, 0, 0, 255]); // blue in BGRA
}

#[test]
fn gif_loads_first_frame() {
    let dir = temp_dir("gif");
    let path = dir.join("two.gif");
    let red = RgbaImage::from_pixel(2, 2, Rgba([255, 0, 0, 255]));
    let green = RgbaImage::from_pixel(2, 2, Rgba([0, 255, 0, 255]));
    {
        let file = fs::File::create(&path).unwrap();
        let mut enc = GifEncoder::new(file);
        enc.encode_frames(vec![Frame::new(red), Frame::new(green)])
            .unwrap();
    }
    let out = load_image(&path).unwrap();
    assert_eq!((out.width, out.height), (2, 2));
    assert_eq!(out.pixel(1, 1), &[0, 0, 255, 255]);
}

#[test]
fn missing_file_fails_cleanly() {
    let dir = temp_dir("missing");
    let path = dir.join("missing.png");
    let err = load_image(&path).unwrap_err();
    assert_eq!(err, LoadError::NotReadable(path.display().to_string()));
    assert_eq!(
        err.to_string(),
        format!(
            "Cannot open \"{}\": the file does not exist or is not readable.",
            path.display()
        )
    );
}

#[test]
fn directory_is_not_a_file() {
    let dir = temp_dir("dir");
    assert!(matches!(load_image(&dir), Err(LoadError::NotReadable(_))));
}

#[test]
fn empty_path_fails() {
    let err = load_image(Path::new("")).unwrap_err();
    assert_eq!(err.to_string(), "No input image selected.");
}

#[test]
fn garbage_file_is_a_decode_error() {
    let dir = temp_dir("garbage");
    let path = dir.join("not_an_image.png");
    fs::write(&path, b"this is plain text, not pixels").unwrap();
    let err = load_image(&path).unwrap_err();
    assert_eq!(
        err.to_string(),
        format!(
            "Could not read \"{}\" (missing file or unsupported format).",
            path.display()
        )
    );
}
