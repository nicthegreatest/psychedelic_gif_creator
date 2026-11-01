use gifcore::image::{bgr_to_rgba, bgra_to_bgr, rgba_to_bgra, Bgr8, Bgra8, Rgba8};

#[test]
fn from_raw_rejects_wrong_length() {
    let result = Rgba8::from_raw(2, 2, vec![0u8; 10]); // 2x2 RGBA needs 16 bytes
    assert!(result.is_none());
}

#[test]
fn pixel_pixel_mut_roundtrip() {
    let mut img = Rgba8::new(4, 4);
    // Write a pixel
    img.pixel_mut(1, 2)
        .copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
    // Read it back
    let read = img.pixel(1, 2);
    assert_eq!(read, &[0xAA, 0xBB, 0xCC, 0xDD]);
}

#[test]
fn rgba_to_bgra_swap_channels() {
    // 2x1 image: pixel(0,0) = RGBA(10,20,30,40), pixel(1,0) = RGBA(50,60,70,80)
    let src = Rgba8::from_raw(
        2,
        1,
        vec![
            10, 20, 30, 40, // pixel (0,0): R=10, G=20, B=30, A=40
            50, 60, 70, 80, // pixel (1,0): R=50, G=60, B=70, A=80
        ],
    )
    .unwrap();

    let dst = rgba_to_bgra(&src);

    // BGRA should have R and B swapped
    assert_eq!(dst.pixel(0, 0), &[30, 20, 10, 40]);
    assert_eq!(dst.pixel(1, 0), &[70, 60, 50, 80]);
}

#[test]
fn bgra_to_bgr_drops_alpha() {
    let src = Bgra8::from_raw(
        2,
        1,
        vec![
            10, 20, 30, 40, // pixel (0,0): B=10, G=20, R=30, A=40
            50, 60, 70, 80, // pixel (1,0): B=50, G=60, R=70, A=80
        ],
    )
    .unwrap();

    let dst = bgra_to_bgr(&src);

    // BGR should keep B, G, R in order
    assert_eq!(dst.pixel(0, 0), &[10, 20, 30]);
    assert_eq!(dst.pixel(1, 0), &[50, 60, 70]);
}

#[test]
fn bgr_to_rgba_adds_alpha() {
    let src = Bgr8::from_raw(
        2,
        1,
        vec![
            10, 20, 30, // pixel (0,0): B=10, G=20, R=30
            50, 60, 70, // pixel (1,0): B=50, G=60, R=70
        ],
    )
    .unwrap();

    let dst = bgr_to_rgba(&src);

    // BGR in, RGBA out: R and B swap, alpha = 255
    assert_eq!(dst.pixel(0, 0), &[30, 20, 10, 255]);
    assert_eq!(dst.pixel(1, 0), &[70, 60, 50, 255]);
}
