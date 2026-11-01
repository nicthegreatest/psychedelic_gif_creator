use gifcore::image::Bgra8;
use gifcore::kernels::resize::resize_area;

fn solid(w: u32, h: u32, px: [u8; 4]) -> Bgra8 {
    Bgra8::from_raw(w, h, px.repeat((w * h) as usize)).unwrap()
}

#[test]
fn halves_into_block_averages() {
    // Pixel (x, y) has B = 8*y + 2*x, so each 2x2 block averages to a whole number.
    let data: Vec<u8> = (0..4u8)
        .flat_map(|y| (0..4u8).flat_map(move |x| [8 * y + 2 * x, 100, 200, 255]))
        .collect();
    let src = Bgra8::from_raw(4, 4, data).unwrap();
    let dst = resize_area(&src, 2, 2);
    // Block averages: (0+2+8+10)/4 = 5, then +4 per block in x and +16 per block in y.
    assert_eq!(dst.pixel(0, 0), &[5, 100, 200, 255]);
    assert_eq!(dst.pixel(1, 0), &[9, 100, 200, 255]);
    assert_eq!(dst.pixel(0, 1), &[21, 100, 200, 255]);
    assert_eq!(dst.pixel(1, 1), &[25, 100, 200, 255]);
}

#[test]
fn same_size_is_identity() {
    let src = Bgra8::from_raw(2, 2, (0..16).collect()).unwrap();
    assert_eq!(resize_area(&src, 2, 2), src);
}

#[test]
fn fractional_weights_three_to_two() {
    let src = Bgra8::from_raw(3, 1, vec![0, 0, 0, 255, 30, 0, 0, 255, 60, 0, 0, 255]).unwrap();
    let dst = resize_area(&src, 2, 1);
    assert_eq!(dst.pixel(0, 0)[0], 10);
    assert_eq!(dst.pixel(1, 0)[0], 50);
}

#[test]
fn solid_colour_stays_solid() {
    let px = [12, 34, 56, 78];
    for (w, h) in [(5, 3), (2, 2), (1, 1), (3, 2)] {
        let dst = resize_area(&solid(7, 5, px), w, h);
        assert!(dst.data.chunks(4).all(|p| p == px), "{w}x{h}");
    }
}

#[test]
fn zero_size_target_is_empty() {
    let dst = resize_area(&solid(4, 4, [1, 2, 3, 4]), 0, 3);
    assert_eq!((dst.width, dst.height), (0, 3));
    assert!(dst.data.is_empty());
}
