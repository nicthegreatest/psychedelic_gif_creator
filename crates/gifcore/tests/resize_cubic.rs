use gifcore::image::Bgra8;
use gifcore::kernels::resize::resize_cubic;

fn solid(w: u32, h: u32, px: [u8; 4]) -> Bgra8 {
    Bgra8::from_raw(w, h, px.repeat((w * h) as usize)).unwrap()
}

#[test]
fn same_size_is_identity() {
    let src = Bgra8::from_raw(3, 2, (0..24).map(|v| v * 10).collect()).unwrap();
    assert_eq!(resize_cubic(&src, 3, 2), src);
}

#[test]
fn uniform_colour_stays_uniform() {
    let px = [12, 34, 56, 78];
    for (w, h) in [(3, 2), (13, 11)] {
        let dst = resize_cubic(&solid(7, 5, px), w, h);
        assert!(dst.data.chunks(4).all(|p| p == px), "{w}x{h}");
    }
}

#[test]
fn output_has_requested_size() {
    let dst = resize_cubic(&solid(4, 4, [1, 2, 3, 4]), 9, 2);
    assert_eq!((dst.width, dst.height, dst.data.len()), (9, 2, 9 * 2 * 4));
}

#[test]
fn zero_size_target_is_empty() {
    let dst = resize_cubic(&solid(4, 4, [1, 2, 3, 4]), 0, 3);
    assert_eq!((dst.width, dst.height), (0, 3));
    assert!(dst.data.is_empty());
}

#[test]
fn ramp_stays_monotonic() {
    let data: Vec<u8> = (0..8u8).flat_map(|i| [i * 32, 0, 0, 255]).collect();
    let src = Bgra8::from_raw(8, 1, data).unwrap();
    let dst = resize_cubic(&src, 16, 1);
    let b: Vec<u8> = dst.data.chunks(4).map(|p| p[0]).collect();
    assert!(b.windows(2).all(|w| w[0] <= w[1]), "{b:?}");
}
