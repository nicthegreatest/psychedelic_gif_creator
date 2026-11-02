//! Frame renderer (port of v2 `src/core/frame_renderer.cpp`).
//!
//! `FrameRenderer::new` precomputes the fitted image, the pyramid, the opaque
//! rect and the vignette. [`FrameRenderer::render_frame`] is a pure function of
//! the frame index: no interior mutability, no shared state, no RNG.
//!
//! Pixel work stays in BGRA/BGR so the port can be compared with v2 line for
//! line. Kaleidoscope, stars, wave, pixelation, hue/saturation, invert and blur
//! live in [`crate::effects`] and run in v2's order. Pixel parameters are
//! scaled by the preview scale, exactly as v2 does.

use crate::effects::kaleidoscope::AnimatedRadial;
use crate::effects::stars::Star;
use crate::effects::{colour, kaleidoscope, stars, texture, wave};
use crate::fit::{self, Interpolation, Rect};
use crate::image::{bgra_to_bgr, Bgr8, Bgra8};
use crate::kernels::pyr::pyr_down;
use crate::kernels::resize::{resize_area, resize_cubic};
use crate::kernels::warp::{rotation_matrix, warp_affine, Border};
use crate::settings::{GifSettings, RotationDirection, ZoomMode};

const PI: f64 = std::f64::consts::PI;
const TWO_PI: f64 = 2.0 * PI;

/// One composited tunnel layer. `e` is its depth (`scale = decay^e`).
struct Layer {
    e: f64,
    alpha: f64,
}

/// Precomputed renderer. One instance paints every frame of a loop.
pub struct FrameRenderer {
    settings: GifSettings,
    /// Preview scale actually used (`<= 0` was replaced with 1, as in v2).
    scale: f64,
    width: u32,
    height: u32,
    /// Pyramid of the fitted premultiplied image. Level 0 is full output size.
    /// Empty when [`Self::animated_kaleido`] rebuilds it inside `render_frame`.
    pyramid: Vec<Bgra8>,
    /// Radial kaleidoscope that turns. `None` for off, quadrant mirror and a
    /// static radial fold (those are already baked into [`Self::pyramid`]).
    animated_kaleido: Option<AnimatedRadial>,
    /// Stars generated once from `star_seed`. Empty when the pattern is off.
    stars: Vec<Star>,
    /// Content rect when every pixel in it is opaque. Empty means "no skip".
    opaque: Option<Rect>,
    /// 8-bit vignette multiplier, one value per pixel, or absent when off.
    vignette: Option<Vec<u8>>,
    /// Signed turns per loop (counter-clockwise is negative).
    rotation_turns: f64,
}

impl FrameRenderer {
    /// Renderer at scale 1 (export size).
    pub fn new(settings: &GifSettings, source: &Bgra8) -> Self {
        Self::with_scale(settings, source, 1.0)
    }

    /// Renderer at `scale`. v2 uses 1 for export and `< 1` for the preview.
    /// A non-positive scale is treated as 1, matching the v2 constructor.
    pub fn with_scale(settings: &GifSettings, source: &Bgra8, scale: f64) -> Self {
        assert!(
            source.width > 0 && source.height > 0,
            "FrameRenderer source is empty"
        );
        assert_eq!(
            source.data.len(),
            source.width as usize * source.height as usize * 4,
            "BGRA buffer length does not match its size"
        );

        let scale = if scale > 0.0 { scale } else { 1.0 };
        let mut settings = settings.clone();
        settings.num_frames = settings.num_frames.max(1);
        let (width, height) = output_size(&settings, scale);

        // Fit the premultiplied source into the output frame.
        let mut fitted = source.clone();
        premultiply(&mut fitted);
        let plan = fit::fit_plan(&fitted, width, height, settings.fit_mode);
        let base = apply_fit(&fitted, &plan);

        // Opaque rect is the pre-kaleidoscope content, matching v2.
        let opaque = if content_fully_opaque(&base, plan.content) {
            Some(plan.content)
        } else {
            None
        };
        let (pyramid, animated_kaleido) = match kaleidoscope::prepare(base, &settings) {
            kaleidoscope::Prepared::Static(mirrored) => (build_pyramid(mirrored), None),
            kaleidoscope::Prepared::Animated(state) => (Vec::new(), Some(state)),
        };
        let stars = stars::layout(
            settings.num_stars,
            settings.star_seed,
            settings.star_pattern,
        );

        let vignette = if settings.vignette_strength > 0.0 {
            Some(build_vignette(width, height, settings.vignette_strength))
        } else {
            None
        };

        let mut rotation_turns = effective_rotation_turns(&settings);
        if settings.rotation_direction == RotationDirection::CounterClockwise {
            rotation_turns = -rotation_turns;
        }

        FrameRenderer {
            settings,
            scale,
            width,
            height,
            pyramid,
            animated_kaleido,
            stars,
            opaque,
            vignette,
            rotation_turns,
        }
    }

    /// Output size in pixels for `settings` at `scale` (minimum 2×2).
    ///
    /// Unlike [`Self::with_scale`], this does not rewrite a non-positive scale:
    /// v2's `outputSize` multiplies by the scale it is given.
    pub fn output_size(settings: &GifSettings, scale: f64) -> (u32, u32) {
        output_size(settings, scale)
    }

    /// Whole turns actually used outside Tunnel mode.
    ///
    /// Fractional speeds are rounded so the loop is seamless, and any non-zero
    /// speed is at least one turn. Tunnel mode keeps the value exact. The sign
    /// (counter-clockwise) is applied by the constructor, not here.
    pub fn effective_rotation_turns(settings: &GifSettings) -> f64 {
        effective_rotation_turns(settings)
    }

    /// Output width and height.
    pub fn frame_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Frames per loop (`num_frames`, at least 1).
    pub fn frame_count(&self) -> i32 {
        self.settings.num_frames
    }

    /// Render frame `index` as 8-bit BGR.
    ///
    /// `t = index / frame_count`. `index == frame_count` is `t = 1`, which
    /// equals frame 0 when the loop is seamless.
    pub fn render_frame(&self, index: i32) -> Bgr8 {
        let t = f64::from(index) / f64::from(self.settings.num_frames);

        let rebuilt;
        let pyramid = if let Some(state) = &self.animated_kaleido {
            rebuilt = build_pyramid(kaleidoscope::frame_at(state, t));
            rebuilt.as_slice()
        } else {
            self.pyramid.as_slice()
        };

        let mut frame = Bgra8::new(self.width, self.height);
        self.composite_layers(&mut frame, pyramid, t);
        stars::draw(&mut frame, &self.stars, self.settings.star_pattern, t);

        // Premultiplied BGRA with the alpha dropped is "composited over black".
        let mut bgr = bgra_to_bgr(&frame);
        if let Some(mask) = &self.vignette {
            apply_vignette(&mut bgr, mask);
        }
        if self.settings.zoom_mode == ZoomMode::Oscillating {
            bgr = oscillating_zoom(bgr, &self.settings, t);
        }

        texture::pixelate(&mut bgr, self.settings.pixelation_level, self.scale);
        wave::apply(&mut bgr, &self.settings, self.scale, t);
        colour::apply(&mut bgr, &self.settings, t, index);
        texture::blur(&mut bgr, self.settings.blur_radius, self.scale);
        bgr
    }

    /// Nested copies of the image. Tunnel mode flies through them (depth twist);
    /// Oscillating and None stack `max_layers` copies and spin them together.
    fn composite_layers(&self, frame: &mut Bgra8, pyr: &[Bgra8], t: f64) {
        let layers_n = self.settings.max_layers.max(1);
        let decay = self.settings.scale_decay.clamp(0.05, 0.995);
        let centre = (
            (f64::from(self.width) - 1.0) / 2.0,
            (f64::from(self.height) - 1.0) / 2.0,
        );
        let tunnel = self.settings.zoom_mode == ZoomMode::Tunnel;
        let steps = self.settings.tunnel_steps.max(1);
        let layers = if tunnel {
            tunnel_layers(layers_n, decay, t, steps)
        } else {
            (0..layers_n)
                .map(|k| Layer {
                    e: f64::from(k),
                    alpha: 1.0,
                })
                .collect()
        };

        let angle_of = |e: f64| -> f64 {
            if tunnel {
                // Twist by depth: layer k at t = 1 is layer k - steps at t = 0,
                // so a fractional turn still loops.
                -360.0 * self.rotation_turns * e / f64::from(steps)
            } else {
                360.0 * self.rotation_turns * t
            }
        };

        // Nothing behind the smallest fully-opaque layer that covers the frame
        // can be seen, so drawing starts there.
        let first = self.first_visible_layer(&layers, pyr, centre, decay, &angle_of);

        let frame_rect_w = self.width;
        let frame_rect_h = self.height;
        for layer in layers.iter().skip(first) {
            if layer.alpha <= 0.0 {
                continue;
            }
            let scale = decay.powf(layer.e);
            let level = pyramid_level(pyr.len(), scale);
            let img = &pyr[level];
            let mut matrix = layer_matrix(img, pyr, scale, centre, angle_of(layer.e));
            let roi = transformed_roi(img.width, img.height, &matrix, frame_rect_w, frame_rect_h);
            if roi.width < 1 || roi.height < 1 {
                continue;
            }
            // Warp the clipped ROI only; the matrix origin moves with it.
            matrix[0][2] -= f64::from(roi.x);
            matrix[1][2] -= f64::from(roi.y);
            let warped = warp_affine(
                img,
                &matrix,
                roi.width,
                roi.height,
                Border::Constant([0, 0, 0, 0]),
            );
            let alpha8 = (layer.alpha * 255.0).round() as i32;
            blend_over(frame, roi, &warped, alpha8);
        }
    }

    /// Index of the first layer that is not completely hidden.
    fn first_visible_layer(
        &self,
        layers: &[Layer],
        pyr: &[Bgra8],
        centre: (f64, f64),
        decay: f64,
        angle_of: &impl Fn(f64) -> f64,
    ) -> usize {
        let Some(opaque) = self.opaque else {
            return 0;
        };
        let mut first = 0usize;
        for (i, layer) in layers.iter().enumerate() {
            if layer.alpha < 1.0 {
                continue;
            }
            // The cover test uses the full-resolution image, as v2 does.
            let scale = decay.powf(layer.e);
            let matrix = layer_matrix(&pyr[0], pyr, scale, centre, angle_of(layer.e));
            if layer_covers_frame(&matrix, self.width, self.height, opaque) {
                first = i;
            }
        }
        first
    }
}

fn output_size(settings: &GifSettings, scale: f64) -> (u32, u32) {
    let dim = |n: i32| {
        let rounded = (f64::from(n) * scale).round();
        if !rounded.is_finite() || rounded < 2.0 {
            2
        } else if rounded >= f64::from(u32::MAX) {
            u32::MAX
        } else {
            rounded as u32
        }
    };
    (dim(settings.output_width), dim(settings.output_height))
}

fn effective_rotation_turns(settings: &GifSettings) -> f64 {
    if settings.rotation_direction == RotationDirection::None || settings.rotation_turns <= 0.0 {
        return 0.0;
    }
    if settings.zoom_mode == ZoomMode::Tunnel {
        return settings.rotation_turns;
    }
    // `f64::round` is half away from zero, like C++ `std::round`.
    settings.rotation_turns.round().max(1.0)
}

fn premultiply(bgra: &mut Bgra8) {
    for px in bgra.data.as_chunks_mut::<4>().0 {
        let a = i32::from(px[3]);
        if a == 255 {
            continue;
        }
        for channel in &mut px[..3] {
            *channel = ((i32::from(*channel) * a + 127) / 255) as u8;
        }
    }
}

fn apply_fit(src: &Bgra8, plan: &fit::FitPlan) -> Bgra8 {
    let scaled = match plan.interpolation {
        Interpolation::Area => resize_area(src, plan.scaled_width, plan.scaled_height),
        Interpolation::Cubic => resize_cubic(src, plan.scaled_width, plan.scaled_height),
    };
    let mut canvas = Bgra8::new(plan.base_width, plan.base_height);
    blit(&scaled, plan.src_rect, &mut canvas, plan.dst_rect);
    if plan.safety_resize {
        // v2: `cv::resize` default, `INTER_LINEAR`. Rounding makes the scaled
        // canvas differ from the output size; see `resize_linear`.
        resize_linear(&canvas, plan.out_width, plan.out_height)
    } else {
        canvas
    }
}

fn blit(src: &Bgra8, src_rect: Rect, dst: &mut Bgra8, dst_rect: Rect) {
    debug_assert_eq!(
        (src_rect.width, src_rect.height),
        (dst_rect.width, dst_rect.height)
    );
    if src_rect.width == 0 || src_rect.height == 0 {
        return;
    }
    let row_bytes = src_rect.width as usize * 4;
    for row in 0..src_rect.height {
        let s0 = ((src_rect.y + row) as usize * src.width as usize + src_rect.x as usize) * 4;
        let d0 = ((dst_rect.y + row) as usize * dst.width as usize + dst_rect.x as usize) * 4;
        dst.data[d0..d0 + row_bytes].copy_from_slice(&src.data[s0..s0 + row_bytes]);
    }
}

fn content_fully_opaque(img: &Bgra8, content: Rect) -> bool {
    if content.width == 0
        || content.height == 0
        || content.x.saturating_add(content.width) > img.width
        || content.y.saturating_add(content.height) > img.height
    {
        return false;
    }
    (content.y..content.y + content.height)
        .all(|y| (content.x..content.x + content.width).all(|x| img.pixel(x, y)[3] == 255))
}

fn build_pyramid(base: Bgra8) -> Vec<Bgra8> {
    let mut pyr = vec![base];
    while pyr.len() < 8 {
        let last = pyr.last().expect("pyramid always has a base");
        if last.width < 32 || last.height < 32 {
            break;
        }
        let next = pyr_down(last);
        pyr.push(next);
    }
    pyr
}

/// v2: `mask = clamp(1 - strength * d² / maxd², 0, 1) * 255`, stored as 8-bit
/// with `cvRound` (half to even), then multiplied per channel with scale 1/255.
fn build_vignette(width: u32, height: u32, strength: f64) -> Vec<u8> {
    let cx = f64::from(width) / 2.0;
    let cy = f64::from(height) / 2.0;
    let max_d2 = cx * cx + cy * cy;
    let mut mask = vec![0u8; width as usize * height as usize];
    for r in 0..height {
        for c in 0..width {
            let dx = f64::from(c) - cx;
            let dy = f64::from(r) - cy;
            let d2 = (dx * dx + dy * dy) / max_d2;
            let v = ((1.0 - strength * d2).clamp(0.0, 1.0) * 255.0) as f32;
            mask[(r * width + c) as usize] = v.round_ties_even().clamp(0.0, 255.0) as u8;
        }
    }
    mask
}

fn apply_vignette(bgr: &mut Bgr8, mask: &[u8]) {
    for (px, &m) in bgr.data.as_chunks_mut::<3>().0.iter_mut().zip(mask) {
        let m = i32::from(m);
        for channel in px {
            // Matches `cv::multiply(..., 1/255)` for every 8-bit pair.
            *channel = ((i32::from(*channel) * m + 127) / 255) as u8;
        }
    }
}

fn tunnel_layers(layers_n: i32, decay: f64, t: f64, steps: i32) -> Vec<Layer> {
    let shift = f64::from(steps) * t;
    let max_scale: f64 = 3.0;
    // `decay` is in (0, 1), so this depth is negative.
    let e_min = max_scale.ln() / decay.ln();
    let mut layers = Vec::new();
    let mut k = (e_min + shift).floor() as i32 - 1;
    let limit = f64::from(layers_n);
    loop {
        let e = f64::from(k) - shift;
        if e >= limit {
            break;
        }
        k += 1;
        if e < e_min {
            continue;
        }
        let mut alpha = 1.0;
        if e > limit - 1.0 {
            alpha = limit - e; // fade in at the centre
        }
        if e < e_min + 1.0 {
            alpha = alpha.min(e - e_min); // fade out when huge
        }
        layers.push(Layer {
            e,
            alpha: alpha.clamp(0.0, 1.0),
        });
    }
    layers
}

/// Deepest pyramid level whose texels are still at least as fine as the layer.
fn pyramid_level(levels: usize, scale: f64) -> usize {
    let mut level = 0usize;
    while level + 1 < levels && scale * f64::from(1u32 << (level + 1)) <= 1.0 {
        level += 1;
    }
    level
}

/// Affine map of pyramid level `img` onto the frame: per-axis scale (pyrDown
/// rounds sizes), rotation by `angle_deg` clockwise, centred on the frame.
///
/// v2 builds this by hand rather than with `getRotationMatrix2D`, because the
/// two axes can differ. Positive `angle_deg` is clockwise here; the matrix
/// itself uses OpenCV's counter-clockwise convention (`th = -angle`).
fn layer_matrix(
    img: &Bgra8,
    pyr: &[Bgra8],
    scale: f64,
    centre: (f64, f64),
    angle_deg: f64,
) -> [[f64; 3]; 2] {
    let sx = scale * f64::from(pyr[0].width) / f64::from(img.width);
    let sy = scale * f64::from(pyr[0].height) / f64::from(img.height);
    let th = -angle_deg * PI / 180.0;
    let (co, si) = (th.cos(), th.sin());
    let cx = (f64::from(img.width) - 1.0) / 2.0;
    let cy = (f64::from(img.height) - 1.0) / 2.0;
    let m00 = co * sx;
    let m01 = si * sy;
    let m10 = -si * sx;
    let m11 = co * sy;
    [
        [m00, m01, centre.0 - (m00 * cx + m01 * cy)],
        [m10, m11, centre.1 - (m10 * cx + m11 * cy)],
    ]
}

/// Inverse of a 2×3 affine matrix, OpenCV `invertAffineTransform` (double).
/// A singular matrix inverts to zeros.
fn invert_affine(m: &[[f64; 3]; 2]) -> [f64; 6] {
    let [[m0, m1, m2], [m3, m4, m5]] = *m;
    let det = m0 * m4 - m1 * m3;
    let det = if det != 0.0 { 1.0 / det } else { 0.0 };
    let n0 = m4 * det;
    let n1 = -m1 * det;
    let n3 = -m3 * det;
    let n4 = m0 * det;
    let b1 = -n0 * m2 - n1 * m5;
    let b2 = -n3 * m2 - n4 * m5;
    [n0, n1, b1, n3, n4, b2]
}

fn layer_covers_frame(matrix: &[[f64; 3]; 2], width: u32, height: u32, opaque: Rect) -> bool {
    let inv = invert_affine(matrix);
    let x0 = f64::from(opaque.x);
    let y0 = f64::from(opaque.y);
    // `br() - 1`: the last pixel still inside the rect.
    let x1 = f64::from(opaque.x + opaque.width) - 1.0;
    let y1 = f64::from(opaque.y + opaque.height) - 1.0;
    let corners = [
        (0.0, 0.0),
        (f64::from(width), 0.0),
        (0.0, f64::from(height)),
        (f64::from(width), f64::from(height)),
    ];
    corners.iter().all(|(px, py)| {
        let x = inv[0] * px + inv[1] * py + inv[2];
        let y = inv[3] * px + inv[4] * py + inv[5];
        x >= x0 && y >= y0 && x <= x1 && y <= y1
    })
}

/// Bounding box of the image's outer corners after `matrix`, clipped to the
/// frame. v2 transforms `Point2f` with the matrix converted to float, then
/// `boundingRect` (floor of each extent, inclusive).
fn transformed_roi(
    src_w: u32,
    src_h: u32,
    matrix: &[[f64; 3]; 2],
    frame_w: u32,
    frame_h: u32,
) -> Rect {
    let m = [
        [
            matrix[0][0] as f32,
            matrix[0][1] as f32,
            matrix[0][2] as f32,
        ],
        [
            matrix[1][0] as f32,
            matrix[1][1] as f32,
            matrix[1][2] as f32,
        ],
    ];
    let pts = [
        (0.0f32, 0.0f32),
        (src_w as f32, 0.0),
        (0.0, src_h as f32),
        (src_w as f32, src_h as f32),
    ];
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for (x, y) in pts {
        let xo = m[0][0] * x + m[0][1] * y + m[0][2];
        let yo = m[1][0] * x + m[1][1] * y + m[1][2];
        min_x = min_x.min(xo);
        max_x = max_x.max(xo);
        min_y = min_y.min(yo);
        max_y = max_y.max(yo);
    }
    let x0 = min_x.floor() as i32;
    let y0 = min_y.floor() as i32;
    let x1 = max_x.floor() as i32;
    let y1 = max_y.floor() as i32;
    // Inclusive bounding rect (x0, y0, x1 - x0 + 1, y1 - y0 + 1) intersected
    // with the frame, matching `boundingRect & Rect(0, 0, W, H)`.
    let rx = x0.max(0);
    let ry = y0.max(0);
    let rw = (x1 + 1).min(frame_w as i32) - rx;
    let rh = (y1 + 1).min(frame_h as i32) - ry;
    if rw <= 0 || rh <= 0 {
        Rect::new(0, 0, 0, 0)
    } else {
        Rect::new(rx as u32, ry as u32, rw as u32, rh as u32)
    }
}

/// `dst = src * alpha OVER dst`, both premultiplied BGRA. `alpha8 == 255` is
/// the common path. Rounding is `(v + 127) / 255`, then a truncating cast,
/// matching v2's `static_cast<uchar>`.
fn blend_over(dst: &mut Bgra8, roi: Rect, src: &Bgra8, alpha8: i32) {
    for row in 0..roi.height {
        let d_row = ((roi.y + row) as usize * dst.width as usize + roi.x as usize) * 4;
        let s_row = row as usize * src.width as usize * 4;
        for col in 0..roi.width as usize {
            let d = d_row + col * 4;
            let s = s_row + col * 4;
            let sa = i32::from(src.data[s + 3]);
            if sa == 0 {
                continue;
            }
            if alpha8 == 255 {
                if sa == 255 {
                    dst.data[d..d + 4].copy_from_slice(&src.data[s..s + 4]);
                    continue;
                }
                let inv = 255 - sa;
                for k in 0..4 {
                    let sv = i32::from(src.data[s + k]);
                    let dv = i32::from(dst.data[d + k]);
                    dst.data[d + k] = (sv + (dv * inv + 127) / 255) as u8;
                }
            } else {
                let mut sv = [0i32; 4];
                for (k, slot) in sv.iter_mut().enumerate() {
                    *slot = (i32::from(src.data[s + k]) * alpha8 + 127) / 255;
                }
                let inv = 255 - sv[3];
                for (k, &sv_k) in sv.iter().enumerate() {
                    let dv = i32::from(dst.data[d + k]);
                    dst.data[d + k] = (sv_k + (dv * inv + 127) / 255) as u8;
                }
            }
        }
    }
}

fn oscillating_zoom(bgr: Bgr8, settings: &GifSettings, t: f64) -> Bgr8 {
    let g = settings.zoom_midpoint
        + settings.zoom_amplitude * (TWO_PI * f64::from(settings.zoom_cycles) * t).sin();
    if (g - 1.0).abs() <= 1e-9 || g <= 0.0 {
        return bgr;
    }
    // v2 passes a `Point2f` centre, `(w - 1) / 2` in float.
    let centre = (
        f64::from((bgr.width - 1) as f32 / 2.0),
        f64::from((bgr.height - 1) as f32 / 2.0),
    );
    let matrix = rotation_matrix(centre, 0.0, g);
    // The warp kernel is BGRA. Reflect-101 sampling is per channel, and a
    // constant alpha of 255 stays 255, so the BGR result matches a 3-channel warp.
    let bgra = bgr_to_bgra(&bgr);
    let warped = warp_affine(&bgra, &matrix, bgr.width, bgr.height, Border::Reflect101);
    bgra_to_bgr(&warped)
}

fn bgr_to_bgra(src: &Bgr8) -> Bgra8 {
    let mut out = Bgra8::new(src.width, src.height);
    for (dst, src) in out
        .data
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(src.data.as_chunks::<3>().0)
    {
        dst[0] = src[0];
        dst[1] = src[1];
        dst[2] = src[2];
        dst[3] = 255;
    }
    out
}

/// Bilinear resize, OpenCV `INTER_LINEAR` coordinate convention
/// (`(x + 0.5) * src / dst - 0.5`) with edge pixels replicated.
///
/// v2 only calls this when `lround(src * (out / src))` is not the output size.
/// That does not happen for any side length in `1..=2000`, so the golden frames
/// never reach here. It is not the fixed-point OpenCV kernel; see TODO.
fn resize_linear(src: &Bgra8, dst_w: u32, dst_h: u32) -> Bgra8 {
    if dst_w == 0 || dst_h == 0 || src.width == 0 || src.height == 0 {
        return Bgra8::new(dst_w, dst_h);
    }
    if src.width == dst_w && src.height == dst_h {
        return src.clone();
    }
    let scale_x = f64::from(src.width) / f64::from(dst_w);
    let scale_y = f64::from(src.height) / f64::from(dst_h);
    let x_last = f64::from(src.width - 1);
    let y_last = f64::from(src.height - 1);
    let mut dst = Bgra8::new(dst_w, dst_h);
    for y in 0..dst_h {
        let (y0, y1, ty) = linear_tap((f64::from(y) + 0.5) * scale_y - 0.5, y_last);
        let row0 = y0 as usize * src.width as usize * 4;
        let row1 = y1 as usize * src.width as usize * 4;
        for x in 0..dst_w {
            let (x0, x1, tx) = linear_tap((f64::from(x) + 0.5) * scale_x - 0.5, x_last);
            let mut acc = [0.0f64; 4];
            for (row, wy) in [(row0, 1.0 - ty), (row1, ty)] {
                for (col, wx) in [(x0, 1.0 - tx), (x1, tx)] {
                    let s = row + col as usize * 4;
                    let w = wx * wy;
                    for (a, v) in acc.iter_mut().zip(&src.data[s..s + 4]) {
                        *a += w * f64::from(*v);
                    }
                }
            }
            let d = (y as usize * dst_w as usize + x as usize) * 4;
            for (out, a) in dst.data[d..d + 4].iter_mut().zip(acc) {
                *out = a.round_ties_even().clamp(0.0, 255.0) as u8;
            }
        }
    }
    dst
}

/// Source tap pair for one destination coordinate. Out-of-range positions
/// clamp to the edge sample with a zero fraction (OpenCV replicate).
fn linear_tap(pos: f64, last: f64) -> (u32, u32, f64) {
    if pos <= 0.0 {
        return (0, 0, 0.0);
    }
    if pos >= last {
        let i = last as u32;
        return (i, i, 0.0);
    }
    let i0 = pos.floor();
    let t = pos - i0;
    (i0 as u32, i0 as u32 + 1, t)
}
