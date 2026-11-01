//! File -> 8-bit BGRA with straight alpha (port of v2 `image_loader.cpp`).
//!
//! Decoding uses the `image` crate (PNG, JPEG, WebP, TIFF, BMP, GIF first frame, PNM). EXIF
//! orientation is applied, then [`normalise_to_bgra8`] follows v2's `normaliseToBGRA8` rules.

use std::fmt;
use std::fs::File;
use std::path::Path;

use ::image::metadata::Orientation;
use ::image::{DynamicImage, ImageDecoder, ImageError, ImageReader};

use crate::image::Bgra8;

/// Why an image could not be loaded. `Display` gives v2's user-facing messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// The path was empty.
    NoPath,
    /// The path is not a regular file or cannot be opened for reading.
    NotReadable(String),
    /// The file exists but could not be decoded (unknown or corrupt format).
    Decode(String),
    /// The decoded image has a channel count we cannot map to BGRA.
    UnsupportedChannels(usize),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::NoPath => write!(f, "No input image selected."),
            LoadError::NotReadable(path) => write!(
                f,
                "Cannot open \"{path}\": the file does not exist or is not readable."
            ),
            LoadError::Decode(path) => write!(
                f,
                "Could not read \"{path}\" (missing file or unsupported format)."
            ),
            LoadError::UnsupportedChannels(n) => {
                write!(f, "Unsupported number of image channels: {n}")
            }
        }
    }
}

impl std::error::Error for LoadError {}

/// Load any supported image file as 8-bit BGRA with straight (non-premultiplied) alpha.
pub fn load_image(path: &Path) -> Result<Bgra8, LoadError> {
    if path.as_os_str().is_empty() {
        return Err(LoadError::NoPath);
    }
    let shown = path.display().to_string();
    if !path.is_file() || File::open(path).is_err() {
        return Err(LoadError::NotReadable(shown));
    }
    let decoded = decode_oriented(path).map_err(|_| LoadError::Decode(shown))?;
    normalise_to_bgra8(&decoded)
}

/// Decode by content (not extension) and apply the EXIF orientation, if any.
fn decode_oriented(path: &Path) -> Result<DynamicImage, ImageError> {
    let mut decoder = ImageReader::open(path)?
        .with_guessed_format()?
        .into_decoder()?;
    // A broken EXIF block should not stop the image from loading.
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut image = DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);
    Ok(image)
}

/// Normalise a decoded image to 8-bit BGRA, as v2's `normaliseToBGRA8`:
///
/// - depth: 8-bit kept; 16-bit divided by 257 (rounded); float in 0..1 times 255, otherwise
///   min–max stretched to 0..255 (min/max taken over all channels, alpha included);
/// - channels: grey -> B=G=R, alpha 255; grey+alpha -> B=G=R plus alpha; RGB -> BGR, alpha 255;
///   RGBA -> BGRA, alpha kept straight.
pub fn normalise_to_bgra8(input: &DynamicImage) -> Result<Bgra8, LoadError> {
    let (width, height) = (input.width(), input.height());
    let (samples, channels): (Vec<u8>, usize) = match input {
        DynamicImage::ImageLuma8(b) => (b.as_raw().clone(), 1),
        DynamicImage::ImageLumaA8(b) => (b.as_raw().clone(), 2),
        DynamicImage::ImageRgb8(b) => (b.as_raw().clone(), 3),
        DynamicImage::ImageRgba8(b) => (b.as_raw().clone(), 4),
        DynamicImage::ImageLuma16(b) => (u16_to_u8(b.as_raw()), 1),
        DynamicImage::ImageLumaA16(b) => (u16_to_u8(b.as_raw()), 2),
        DynamicImage::ImageRgb16(b) => (u16_to_u8(b.as_raw()), 3),
        DynamicImage::ImageRgba16(b) => (u16_to_u8(b.as_raw()), 4),
        DynamicImage::ImageRgb32F(b) => (f32_to_u8(b.as_raw()), 3),
        DynamicImage::ImageRgba32F(b) => (f32_to_u8(b.as_raw()), 4),
        // `DynamicImage` is non-exhaustive; let the crate convert anything newer.
        other => (other.to_rgba8().into_raw(), 4),
    };

    let mut data = Vec::with_capacity(width as usize * height as usize * 4);
    match channels {
        1 => samples
            .iter()
            .for_each(|&l| data.extend_from_slice(&[l, l, l, 255])),
        2 => samples.as_chunks::<2>().0.iter().for_each(|&[l, a]| {
            data.extend_from_slice(&[l, l, l, a]);
        }),
        3 => samples.as_chunks::<3>().0.iter().for_each(|&[r, g, b]| {
            data.extend_from_slice(&[b, g, r, 255]);
        }),
        4 => samples.as_chunks::<4>().0.iter().for_each(|&[r, g, b, a]| {
            data.extend_from_slice(&[b, g, r, a]);
        }),
        n => return Err(LoadError::UnsupportedChannels(n)),
    }
    Bgra8::from_raw(width, height, data).ok_or(LoadError::UnsupportedChannels(channels))
}

/// 16-bit -> 8-bit like OpenCV `convertTo(CV_8U, 1/257)`: round to nearest
/// (never a tie, because 257 is odd).
fn u16_to_u8(samples: &[u16]) -> Vec<u8> {
    samples
        .iter()
        .map(|&v| ((u32::from(v) + 128) / 257) as u8)
        .collect()
}

/// Float -> 8-bit: values already in 0..1 are scaled by 255, anything else is min–max
/// stretched. Mirrors OpenCV `convertTo` (float maths, round half to even, saturate).
fn f32_to_u8(samples: &[f32]) -> Vec<u8> {
    let (lo, hi) = samples
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
            (lo.min(f64::from(v)), hi.max(f64::from(v)))
        });
    let (lo, hi) = if samples.is_empty() {
        (0.0, 0.0)
    } else {
        (lo, hi)
    };
    let (alpha, beta) = if lo >= 0.0 && hi <= 1.0 {
        (255.0_f32, 0.0_f32)
    } else {
        let range = if hi - lo > 0.0 { hi - lo } else { 1.0 };
        ((255.0 / range) as f32, (-lo * 255.0 / range) as f32)
    };
    samples
        .iter()
        .map(|&v| saturate_u8(v * alpha + beta))
        .collect()
}

/// OpenCV `saturate_cast<uchar>(float)`: round half to even, clamp to 0..255 (NaN -> 0).
fn saturate_u8(v: f32) -> u8 {
    v.round_ties_even().clamp(0.0, 255.0) as u8
}
