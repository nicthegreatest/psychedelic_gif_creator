//! Fit-mode geometry (v2 FrameRenderer constructor), 16:9 vs 4:3 in both directions.

use gifcore::fit::{fit_plan, fit_plan_for_size, Interpolation, Rect};
use gifcore::image::Bgra8;
use gifcore::settings::FitMode;

#[test]
fn fill_4x3_source_into_16x9_crops_top_and_bottom() {
    let p = fit_plan_for_size(640, 480, 320, 180, FitMode::Fill);
    assert_eq!((p.scaled_width, p.scaled_height), (320, 240));
    assert_eq!(p.interpolation, Interpolation::Area);
    assert_eq!(p.src_rect, Rect::new(0, 30, 320, 180));
    assert_eq!(p.dst_rect, Rect::new(0, 0, 320, 180));
    assert_eq!(p.content, Rect::new(0, 0, 320, 180));
    assert!(!p.safety_resize);
}

#[test]
fn fill_16x9_source_into_4x3_crops_sides() {
    let p = fit_plan_for_size(1600, 900, 400, 300, FitMode::Fill);
    // 1600 / 3 = 533.33 -> 533; (533 - 400) / 2 = 66 (truncated).
    assert_eq!((p.scaled_width, p.scaled_height), (533, 300));
    assert_eq!(p.src_rect, Rect::new(66, 0, 400, 300));
    assert_eq!(p.dst_rect, Rect::new(0, 0, 400, 300));
}

#[test]
fn fit_4x3_source_into_16x9_letterboxes_sides() {
    let p = fit_plan_for_size(640, 480, 320, 180, FitMode::Fit);
    assert_eq!((p.scaled_width, p.scaled_height), (240, 180));
    assert_eq!(p.interpolation, Interpolation::Area);
    assert_eq!(p.src_rect, Rect::new(0, 0, 240, 180));
    assert_eq!(p.dst_rect, Rect::new(40, 0, 240, 180));
    assert_eq!(p.content, p.dst_rect);
    assert_eq!((p.base_width, p.base_height), (320, 180));
    assert!(!p.safety_resize);
}

#[test]
fn fit_16x9_source_into_4x3_letterboxes_top_and_bottom() {
    let p = fit_plan_for_size(1600, 900, 400, 300, FitMode::Fit);
    assert_eq!((p.scaled_width, p.scaled_height), (400, 225));
    // (300 - 225) / 2 = 37 (truncated), so the bottom bar is one row taller.
    assert_eq!(p.dst_rect, Rect::new(0, 37, 400, 225));
}

#[test]
fn stretch_ignores_aspect() {
    let p = fit_plan_for_size(640, 480, 320, 180, FitMode::Stretch);
    assert_eq!((p.scaled_width, p.scaled_height), (320, 180));
    assert_eq!(p.interpolation, Interpolation::Area);
    assert_eq!(p.dst_rect, Rect::new(0, 0, 320, 180));
    assert_eq!(p.content, Rect::new(0, 0, 320, 180));
    assert!(!p.safety_resize);

    let q = fit_plan_for_size(1600, 900, 400, 300, FitMode::Stretch);
    assert_eq!((q.scaled_width, q.scaled_height), (400, 300));
}

#[test]
fn upscaling_uses_cubic() {
    let p = fit_plan_for_size(160, 90, 400, 300, FitMode::Fill);
    assert_eq!(p.interpolation, Interpolation::Cubic);
    assert_eq!((p.scaled_width, p.scaled_height), (533, 300));
    assert_eq!(p.src_rect, Rect::new(66, 0, 400, 300));
    let f = fit_plan_for_size(160, 90, 400, 300, FitMode::Fit);
    assert_eq!(f.interpolation, Interpolation::Cubic);
    assert_eq!(f.dst_rect, Rect::new(0, 37, 400, 225));
}

#[test]
fn stretch_shrinking_one_axis_only_uses_cubic() {
    // INTER_AREA only when both scale factors are below 1.
    let p = fit_plan_for_size(640, 480, 1280, 240, FitMode::Stretch);
    assert_eq!(p.interpolation, Interpolation::Cubic);
}

#[test]
fn same_size_is_identity() {
    for mode in [FitMode::Fill, FitMode::Fit, FitMode::Stretch] {
        let p = fit_plan(&Bgra8::new(320, 180), 320, 180, mode);
        assert_eq!((p.scaled_width, p.scaled_height), (320, 180));
        assert_eq!(p.interpolation, Interpolation::Cubic);
        assert_eq!(p.dst_rect, Rect::new(0, 0, 320, 180));
        assert_eq!(p.content, Rect::new(0, 0, 320, 180));
    }
}
