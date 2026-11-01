//! Fit modes: how the source image is placed in the output frame (port of the fit block in
//! v2 `FrameRenderer::FrameRenderer`, frame_renderer.cpp).
//!
//! v2 premultiplies the source, resizes it, then copies a region onto a transparent W×H
//! canvas. This module computes that geometry as a [`FitPlan`]; the pixel work (OpenCV-style
//! `INTER_AREA` / `INTER_CUBIC` resize) arrives with the resize kernels in M2 part 3.

use crate::image::Bgra8;
use crate::settings::FitMode;

/// Resize filter v2 picks for the fit resize.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interpolation {
    /// OpenCV `INTER_AREA`: used when shrinking on both axes.
    Area,
    /// OpenCV `INTER_CUBIC` (a = -0.75): used otherwise (any axis grows or stays).
    Cubic,
}

/// Axis-aligned pixel rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Rect {
            x,
            y,
            width,
            height,
        }
    }
}

/// Geometry of fitting a source into the output frame.
///
/// Steps (as v2): resize the premultiplied source to `scaled_width × scaled_height` with
/// `interpolation`; copy `src_rect` of the scaled image to `dst_rect` of a transparent
/// (all-zero) `base_width × base_height` canvas; if the canvas is not the output size, resize
/// it to the output size with bilinear filtering (`safety_resize`, OpenCV `INTER_LINEAR`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FitPlan {
    pub mode: FitMode,
    pub out_width: u32,
    pub out_height: u32,
    pub scaled_width: u32,
    pub scaled_height: u32,
    pub interpolation: Interpolation,
    /// Region of the scaled image that is kept (Fill: centre crop).
    pub src_rect: Rect,
    /// Where that region lands on the canvas (Fit: centred letterbox).
    pub dst_rect: Rect,
    /// Canvas size before the safety resize (Stretch: the scaled size; else the output size).
    pub base_width: u32,
    pub base_height: u32,
    /// True when the canvas must be resized (bilinear) to the output size afterwards.
    pub safety_resize: bool,
    /// Area of the output holding image content; v2 uses it to detect a fully opaque layer.
    /// Letterbox bars outside it are transparent black (0,0,0,0).
    pub content: Rect,
}

/// Fit plan for `source` in an `out_width × out_height` frame.
pub fn fit_plan(source: &Bgra8, out_width: u32, out_height: u32, mode: FitMode) -> FitPlan {
    fit_plan_for_size(source.width, source.height, out_width, out_height, mode)
}

/// Fit plan from sizes alone. All sizes must be at least 1.
pub fn fit_plan_for_size(
    src_width: u32,
    src_height: u32,
    out_width: u32,
    out_height: u32,
    mode: FitMode,
) -> FitPlan {
    let (sw, sh) = (src_width.max(1), src_height.max(1));
    let (w, h) = (out_width.max(1), out_height.max(1));
    let sx = f64::from(w) / f64::from(sw);
    let sy = f64::from(h) / f64::from(sh);
    // v2: Size(max(1, lround(cols * fx)), max(1, lround(rows * fy))).
    let scaled = |fx: f64, fy: f64| {
        (
            ((f64::from(sw) * fx).round() as u32).max(1),
            ((f64::from(sh) * fy).round() as u32).max(1),
        )
    };
    let interp = |fx: f64, fy: f64| {
        if fx < 1.0 && fy < 1.0 {
            Interpolation::Area
        } else {
            Interpolation::Cubic
        }
    };
    let full = Rect::new(0, 0, w, h);

    let (fx, fy) = match mode {
        FitMode::Stretch => (sx, sy),
        FitMode::Fill => (sx.max(sy), sx.max(sy)),
        FitMode::Fit => (sx.min(sy), sx.min(sy)),
    };
    let (scaled_width, scaled_height) = scaled(fx, fy);
    let interpolation = interp(fx, fy);

    let (src_rect, dst_rect, base_width, base_height, content) = match mode {
        FitMode::Stretch => {
            let r = Rect::new(0, 0, scaled_width, scaled_height);
            (r, r, scaled_width, scaled_height, full)
        }
        FitMode::Fill => {
            // Centre crop; integer division truncates like C++.
            let ox = scaled_width.saturating_sub(w) / 2;
            let oy = scaled_height.saturating_sub(h) / 2;
            let (cw, ch) = (w.min(scaled_width), h.min(scaled_height));
            (
                Rect::new(ox, oy, cw, ch),
                Rect::new(0, 0, cw, ch),
                w,
                h,
                full,
            )
        }
        FitMode::Fit => {
            // C++ `(W - small.cols) / 2` truncates toward zero, so a negative gap gives 0.
            let x = w.saturating_sub(scaled_width) / 2;
            let y = h.saturating_sub(scaled_height) / 2;
            let (cw, ch) = (w.min(scaled_width), h.min(scaled_height));
            let content = Rect::new(x, y, cw, ch);
            (Rect::new(0, 0, cw, ch), content, w, h, content)
        }
    };

    FitPlan {
        mode,
        out_width: w,
        out_height: h,
        scaled_width,
        scaled_height,
        interpolation,
        src_rect,
        dst_rect,
        base_width,
        base_height,
        safety_resize: (base_width, base_height) != (w, h),
        content,
    }
}
