use gifcore::image::Bgra8;
use gifcore::kernels::pyr::pyr_down;

fn solid(w: u32, h: u32, px: [u8; 4]) -> Bgra8 {
    Bgra8::from_raw(w, h, px.repeat((w * h) as usize)).unwrap()
}

#[test]
fn output_size_rounds_up() {
    for (w, h, ew, eh) in [(8, 6, 4, 3), (5, 3, 3, 2), (1, 1, 1, 1), (2, 7, 1, 4)] {
        let d = pyr_down(&solid(w, h, [0, 0, 0, 0]));
        assert_eq!((d.width, d.height), (ew, eh), "{w}x{h}");
    }
}

#[test]
fn uniform_colour_stays_uniform() {
    let px = [12, 34, 56, 78];
    for (w, h) in [(8, 6), (5, 3), (1, 1), (2, 2)] {
        let d = pyr_down(&solid(w, h, px));
        assert!(d.data.chunks(4).all(|p| p == px), "{w}x{h}");
    }
}

#[test]
fn empty_source_gives_empty_output() {
    let d = pyr_down(&Bgra8::new(0, 0));
    assert_eq!((d.width, d.height), (0, 0));
}

#[test]
fn values_stay_within_source_range() {
    let data: Vec<u8> = (0..64u32)
        .flat_map(|i| [(i * 4) as u8, 50, 200, 255])
        .collect();
    let d = pyr_down(&Bgra8::from_raw(8, 8, data).unwrap());
    assert!(d
        .data
        .chunks(4)
        .all(|p| p[1] == 50 && p[2] == 200 && p[3] == 255));
    assert!(d.data.chunks(4).all(|p| p[0] <= 252));
}
