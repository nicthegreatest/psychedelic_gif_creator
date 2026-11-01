use gifcore::image::Bgra8;
use gifcore::kernels::resize::resize_nearest;

#[test]
fn identity_size_returns_identical_pixels() {
    let src = Bgra8::from_raw(
        3,
        2,
        vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        ],
    )
    .unwrap();

    let dst = resize_nearest(&src, src.width, src.height);

    assert_eq!(dst, src);
}

#[test]
fn upscale_repeats_each_pixel_as_a_block() {
    // 2x2 source; each pixel should become a 2x2 block in the 4x4 result.
    let src = Bgra8::from_raw(
        2,
        2,
        vec![
            0x00, 0x01, 0x02, 0x03, // (0,0)
            0x04, 0x05, 0x06, 0x07, // (1,0)
            0x08, 0x09, 0x0A, 0x0B, // (0,1)
            0x0C, 0x0D, 0x0E, 0x0F, // (1,1)
        ],
    )
    .unwrap();

    let dst = resize_nearest(&src, 4, 4);
    assert_eq!(dst.width, 4);
    assert_eq!(dst.height, 4);

    // Each source pixel maps to a 2x2 block at (sx*2, sy*2).
    // (0,0) -> block at (0,0)
    let p00 = src.pixel(0, 0);
    assert_eq!(dst.pixel(0, 0), p00);
    assert_eq!(dst.pixel(1, 0), p00);
    assert_eq!(dst.pixel(0, 1), p00);
    assert_eq!(dst.pixel(1, 1), p00);
    // (1,0) -> block at (2,0)
    let p10 = src.pixel(1, 0);
    assert_eq!(dst.pixel(2, 0), p10);
    assert_eq!(dst.pixel(3, 0), p10);
    assert_eq!(dst.pixel(2, 1), p10);
    assert_eq!(dst.pixel(3, 1), p10);
    // (0,1) -> block at (0,2)
    let p01 = src.pixel(0, 1);
    assert_eq!(dst.pixel(0, 2), p01);
    assert_eq!(dst.pixel(1, 2), p01);
    assert_eq!(dst.pixel(0, 3), p01);
    assert_eq!(dst.pixel(1, 3), p01);
    // (1,1) -> block at (2,2)
    let p11 = src.pixel(1, 1);
    assert_eq!(dst.pixel(2, 2), p11);
    assert_eq!(dst.pixel(3, 2), p11);
    assert_eq!(dst.pixel(2, 3), p11);
    assert_eq!(dst.pixel(3, 3), p11);
}

#[test]
fn downscales_picks_top_left_pixels() {
    // 4x4 source with a distinct value per pixel so we can spot the mapping.
    // Pixel i holds (B=i, G=i+16, R=i+32, A=255) so all four channels differ.
    let src = Bgra8::from_raw(
        4,
        4,
        (0..16)
            .flat_map(|i| [i as u8, (i + 16) as u8, (i + 32) as u8, 255u8])
            .collect(),
    )
    .unwrap();

    let dst = resize_nearest(&src, 2, 2);
    assert_eq!(dst.width, 2);
    assert_eq!(dst.height, 2);

    // dst(x,y) reads src(2x, 2y).
    assert_eq!(dst.pixel(0, 0), src.pixel(0, 0));
    assert_eq!(dst.pixel(1, 0), src.pixel(2, 0));
    assert_eq!(dst.pixel(0, 1), src.pixel(0, 2));
    assert_eq!(dst.pixel(1, 1), src.pixel(2, 2));
}

#[test]
fn zero_width_returns_empty_image() {
    let src = Bgra8::from_raw(2, 2, vec![1u8; 16]).unwrap();

    let dst = resize_nearest(&src, 0, 5);

    assert_eq!(dst.width, 0);
    assert_eq!(dst.height, 5);
    assert!(dst.data.is_empty());
}

#[test]
fn zero_height_returns_empty_image() {
    let src = Bgra8::from_raw(2, 2, vec![1u8; 16]).unwrap();

    let dst = resize_nearest(&src, 5, 0);

    assert_eq!(dst.width, 5);
    assert_eq!(dst.height, 0);
    assert!(dst.data.is_empty());
}
