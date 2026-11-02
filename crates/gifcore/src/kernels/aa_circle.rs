//! Filled anti-aliased circle matching OpenCV `cv::circle(..., FILLED, LINE_AA)`.
//!
//! `circle` shifts the centre and radius by 16 bits and draws an ellipse.
//! The outline is an `ellipse2Poly` (the sine table in `drawing.cpp`, not
//! `libm`) stroked with `LineAA`, then `FillConvexPoly` overwrites the
//! interior. `LineAA` blends each covered pixel twice. The colour is already
//! an 8-bit BGRA tuple (`scalarToRawData`).

use crate::image::Bgra8;

const XY_SHIFT: i32 = 16;
const XY_ONE: i64 = 1 << XY_SHIFT;

const SLOPE_CORR: [u8; 32] = [
    181, 181, 181, 182, 182, 183, 184, 185, 187, 188, 190, 192, 194, 196, 198, 201, 203, 206, 209,
    211, 214, 218, 221, 224, 227, 231, 235, 238, 242, 246, 250, 254,
];

const FILTER: [i32; 64] = [
    168, 177, 185, 194, 202, 210, 218, 224, 231, 236, 241, 246, 249, 252, 254, 254, 254, 254, 252,
    249, 246, 241, 236, 231, 224, 218, 210, 202, 194, 185, 177, 168, 158, 149, 140, 131, 122, 114,
    105, 97, 89, 82, 75, 68, 62, 56, 50, 45, 40, 36, 32, 28, 25, 22, 19, 16, 14, 12, 11, 9, 8, 7,
    5, 5,
];

/// 451 entries, degrees 0..=450, copied from OpenCV `SinTable`.
/// The spellings are OpenCV's; shortening them would not change the bits, and
/// `0.7071068` must stay this literal rather than `FRAC_1_SQRT_2`.
#[allow(clippy::excessive_precision, clippy::approx_constant)]
const SIN_TABLE: [f32; 451] = [
    0.0000000, 0.0174524, 0.0348995, 0.0523360, 0.0697565, 0.0871557, 0.1045285, 0.1218693,
    0.1391731, 0.1564345, 0.1736482, 0.1908090, 0.2079117, 0.2249511, 0.2419219, 0.2588190,
    0.2756374, 0.2923717, 0.3090170, 0.3255682, 0.3420201, 0.3583679, 0.3746066, 0.3907311,
    0.4067366, 0.4226183, 0.4383711, 0.4539905, 0.4694716, 0.4848096, 0.5000000, 0.5150381,
    0.5299193, 0.5446390, 0.5591929, 0.5735764, 0.5877853, 0.6018150, 0.6156615, 0.6293204,
    0.6427876, 0.6560590, 0.6691306, 0.6819984, 0.6946584, 0.7071068, 0.7193398, 0.7313537,
    0.7431448, 0.7547096, 0.7660444, 0.7771460, 0.7880108, 0.7986355, 0.8090170, 0.8191520,
    0.8290376, 0.8386706, 0.8480481, 0.8571673, 0.8660254, 0.8746197, 0.8829476, 0.8910065,
    0.8987940, 0.9063078, 0.9135455, 0.9205049, 0.9271839, 0.9335804, 0.9396926, 0.9455186,
    0.9510565, 0.9563048, 0.9612617, 0.9659258, 0.9702957, 0.9743701, 0.9781476, 0.9816272,
    0.9848078, 0.9876883, 0.9902681, 0.9925462, 0.9945219, 0.9961947, 0.9975641, 0.9986295,
    0.9993908, 0.9998477, 1.0000000, 0.9998477, 0.9993908, 0.9986295, 0.9975641, 0.9961947,
    0.9945219, 0.9925462, 0.9902681, 0.9876883, 0.9848078, 0.9816272, 0.9781476, 0.9743701,
    0.9702957, 0.9659258, 0.9612617, 0.9563048, 0.9510565, 0.9455186, 0.9396926, 0.9335804,
    0.9271839, 0.9205049, 0.9135455, 0.9063078, 0.8987940, 0.8910065, 0.8829476, 0.8746197,
    0.8660254, 0.8571673, 0.8480481, 0.8386706, 0.8290376, 0.8191520, 0.8090170, 0.7986355,
    0.7880108, 0.7771460, 0.7660444, 0.7547096, 0.7431448, 0.7313537, 0.7193398, 0.7071068,
    0.6946584, 0.6819984, 0.6691306, 0.6560590, 0.6427876, 0.6293204, 0.6156615, 0.6018150,
    0.5877853, 0.5735764, 0.5591929, 0.5446390, 0.5299193, 0.5150381, 0.5000000, 0.4848096,
    0.4694716, 0.4539905, 0.4383711, 0.4226183, 0.4067366, 0.3907311, 0.3746066, 0.3583679,
    0.3420201, 0.3255682, 0.3090170, 0.2923717, 0.2756374, 0.2588190, 0.2419219, 0.2249511,
    0.2079117, 0.1908090, 0.1736482, 0.1564345, 0.1391731, 0.1218693, 0.1045285, 0.0871557,
    0.0697565, 0.0523360, 0.0348995, 0.0174524, 0.0000000, -0.0174524, -0.0348995, -0.0523360,
    -0.0697565, -0.0871557, -0.1045285, -0.1218693, -0.1391731, -0.1564345, -0.1736482, -0.1908090,
    -0.2079117, -0.2249511, -0.2419219, -0.2588190, -0.2756374, -0.2923717, -0.3090170, -0.3255682,
    -0.3420201, -0.3583679, -0.3746066, -0.3907311, -0.4067366, -0.4226183, -0.4383711, -0.4539905,
    -0.4694716, -0.4848096, -0.5000000, -0.5150381, -0.5299193, -0.5446390, -0.5591929, -0.5735764,
    -0.5877853, -0.6018150, -0.6156615, -0.6293204, -0.6427876, -0.6560590, -0.6691306, -0.6819984,
    -0.6946584, -0.7071068, -0.7193398, -0.7313537, -0.7431448, -0.7547096, -0.7660444, -0.7771460,
    -0.7880108, -0.7986355, -0.8090170, -0.8191520, -0.8290376, -0.8386706, -0.8480481, -0.8571673,
    -0.8660254, -0.8746197, -0.8829476, -0.8910065, -0.8987940, -0.9063078, -0.9135455, -0.9205049,
    -0.9271839, -0.9335804, -0.9396926, -0.9455186, -0.9510565, -0.9563048, -0.9612617, -0.9659258,
    -0.9702957, -0.9743701, -0.9781476, -0.9816272, -0.9848078, -0.9876883, -0.9902681, -0.9925462,
    -0.9945219, -0.9961947, -0.9975641, -0.9986295, -0.9993908, -0.9998477, -1.0000000, -0.9998477,
    -0.9993908, -0.9986295, -0.9975641, -0.9961947, -0.9945219, -0.9925462, -0.9902681, -0.9876883,
    -0.9848078, -0.9816272, -0.9781476, -0.9743701, -0.9702957, -0.9659258, -0.9612617, -0.9563048,
    -0.9510565, -0.9455186, -0.9396926, -0.9335804, -0.9271839, -0.9205049, -0.9135455, -0.9063078,
    -0.8987940, -0.8910065, -0.8829476, -0.8746197, -0.8660254, -0.8571673, -0.8480481, -0.8386706,
    -0.8290376, -0.8191520, -0.8090170, -0.7986355, -0.7880108, -0.7771460, -0.7660444, -0.7547096,
    -0.7431448, -0.7313537, -0.7193398, -0.7071068, -0.6946584, -0.6819984, -0.6691306, -0.6560590,
    -0.6427876, -0.6293204, -0.6156615, -0.6018150, -0.5877853, -0.5735764, -0.5591929, -0.5446390,
    -0.5299193, -0.5150381, -0.5000000, -0.4848096, -0.4694716, -0.4539905, -0.4383711, -0.4226183,
    -0.4067366, -0.3907311, -0.3746066, -0.3583679, -0.3420201, -0.3255682, -0.3090170, -0.2923717,
    -0.2756374, -0.2588190, -0.2419219, -0.2249511, -0.2079117, -0.1908090, -0.1736482, -0.1564345,
    -0.1391731, -0.1218693, -0.1045285, -0.0871557, -0.0697565, -0.0523360, -0.0348995, -0.0174524,
    -0.0000000, 0.0174524, 0.0348995, 0.0523360, 0.0697565, 0.0871557, 0.1045285, 0.1218693,
    0.1391731, 0.1564345, 0.1736482, 0.1908090, 0.2079117, 0.2249511, 0.2419219, 0.2588190,
    0.2756374, 0.2923717, 0.3090170, 0.3255682, 0.3420201, 0.3583679, 0.3746066, 0.3907311,
    0.4067366, 0.4226183, 0.4383711, 0.4539905, 0.4694716, 0.4848096, 0.5000000, 0.5150381,
    0.5299193, 0.5446390, 0.5591929, 0.5735764, 0.5877853, 0.6018150, 0.6156615, 0.6293204,
    0.6427876, 0.6560590, 0.6691306, 0.6819984, 0.6946584, 0.7071068, 0.7193398, 0.7313537,
    0.7431448, 0.7547096, 0.7660444, 0.7771460, 0.7880108, 0.7986355, 0.8090170, 0.8191520,
    0.8290376, 0.8386706, 0.8480481, 0.8571673, 0.8660254, 0.8746197, 0.8829476, 0.8910065,
    0.8987940, 0.9063078, 0.9135455, 0.9205049, 0.9271839, 0.9335804, 0.9396926, 0.9455186,
    0.9510565, 0.9563048, 0.9612617, 0.9659258, 0.9702957, 0.9743701, 0.9781476, 0.9816272,
    0.9848078, 0.9876883, 0.9902681, 0.9925462, 0.9945219, 0.9961947, 0.9975641, 0.9986295,
    0.9993908, 0.9998477, 1.0000000,
];

/// Draw a filled anti-aliased circle. `radius < 0` does nothing.
pub fn circle_filled_aa(img: &mut Bgra8, center: (i32, i32), radius: i32, color: [u8; 4]) {
    if radius < 0 || img.width == 0 || img.height == 0 {
        return;
    }
    let center = (
        i64::from(center.0) << XY_SHIFT,
        i64::from(center.1) << XY_SHIFT,
    );
    let radius = i64::from(radius) << XY_SHIFT;
    let vertices = ellipse_vertices(center, (radius, radius));
    fill_convex_poly(img, &vertices, &color);
}

fn cv_round(v: f64) -> i64 {
    v.round_ties_even() as i64
}

fn sincos(angle: i32) -> (f32, f32) {
    let angle = if angle < 0 { angle + 360 } else { angle };
    (SIN_TABLE[(450 - angle) as usize], SIN_TABLE[angle as usize])
}

fn ellipse2_poly(center: (f64, f64), axes: (f64, f64), delta: i32) -> Vec<(f64, f64)> {
    let (alpha, beta) = sincos(0);
    let mut pts = Vec::new();
    let mut i = 0;
    while i < 360 + delta {
        let angle = if i > 360 { 360 } else { i };
        let x = axes.0 * f64::from(SIN_TABLE[(450 - angle) as usize]);
        let y = axes.1 * f64::from(SIN_TABLE[angle as usize]);
        pts.push((
            center.0 + x * f64::from(alpha) - y * f64::from(beta),
            center.1 + x * f64::from(beta) + y * f64::from(alpha),
        ));
        i += delta;
    }
    if pts.is_empty() {
        pts.push(center);
        pts.push(center);
    } else if pts.len() == 1 {
        pts.push(center);
    }
    pts
}

fn ellipse_vertices(center: (i64, i64), axes: (i64, i64)) -> Vec<(i64, i64)> {
    let axes = (axes.0.abs(), axes.1.abs());
    let rough = ((axes.0.max(axes.1) + (XY_ONE >> 1)) >> XY_SHIFT) as i32;
    let delta = if rough < 3 {
        90
    } else if rough < 10 {
        30
    } else if rough < 15 {
        18
    } else {
        5
    };
    let raw = ellipse2_poly(
        (center.0 as f64, center.1 as f64),
        (axes.0 as f64, axes.1 as f64),
        delta,
    );
    let mut vertices = Vec::new();
    // OpenCV's sentinel is `0xFFFFFFFFFFFFFFFF`, which is -1 as int64.
    let mut prev = (-1i64, -1i64);
    for p in raw {
        let mut x = cv_round(p.0 / (XY_ONE as f64)) << XY_SHIFT;
        let mut y = cv_round(p.1 / (XY_ONE as f64)) << XY_SHIFT;
        x += cv_round(p.0 - (x as f64));
        y += cv_round(p.1 - (y as f64));
        if (x, y) != prev {
            vertices.push((x, y));
            prev = (x, y);
        }
    }
    if vertices.len() <= 1 {
        vec![center, center]
    } else {
        vertices
    }
}

fn clip_line(width: i64, height: i64, p1: &mut (i64, i64), p2: &mut (i64, i64)) -> bool {
    if width <= 0 || height <= 0 {
        return false;
    }
    let right = width - 1;
    let bottom = height - 1;
    let (mut x1, mut y1) = *p1;
    let (mut x2, mut y2) = *p2;
    let code = |x: i64, y: i64| {
        i32::from(x < 0)
            + i32::from(x > right) * 2
            + i32::from(y < 0) * 4
            + i32::from(y > bottom) * 8
    };
    let mut c1 = code(x1, y1);
    let mut c2 = code(x2, y2);
    if (c1 & c2) == 0 && (c1 | c2) != 0 {
        if c1 & 12 != 0 {
            let a = if c1 < 8 { 0 } else { bottom };
            x1 += ((a - y1) as f64 * (x2 - x1) as f64 / (y2 - y1) as f64) as i64;
            y1 = a;
            c1 = i32::from(x1 < 0) + i32::from(x1 > right) * 2;
        }
        if c2 & 12 != 0 {
            let a = if c2 < 8 { 0 } else { bottom };
            x2 += ((a - y2) as f64 * (x2 - x1) as f64 / (y2 - y1) as f64) as i64;
            y2 = a;
            c2 = i32::from(x2 < 0) + i32::from(x2 > right) * 2;
        }
        if (c1 & c2) == 0 && (c1 | c2) != 0 {
            if c1 != 0 {
                let a = if c1 == 1 { 0 } else { right };
                y1 += ((a - x1) as f64 * (y2 - y1) as f64 / (x2 - x1) as f64) as i64;
                x1 = a;
                c1 = 0;
            }
            if c2 != 0 {
                let a = if c2 == 1 { 0 } else { right };
                y2 += ((a - x2) as f64 * (y2 - y1) as f64 / (x2 - x1) as f64) as i64;
                x2 = a;
                c2 = 0;
            }
        }
    }
    *p1 = (x1, y1);
    *p2 = (x2, y2);
    c1 | c2 == 0
}

fn blend_channel(current: u8, color: u8, coverage: i32) -> u8 {
    let mut value = i32::from(current);
    value += ((i32::from(color) - value) * coverage + 127) >> 8;
    value += ((i32::from(color) - value) * coverage + 127) >> 8;
    value as u8
}

fn put_aa(img: &mut Bgra8, x: i32, y: i32, color: &[u8; 4], coverage: i32) {
    if (x as u32) >= img.width || (y as u32) >= img.height {
        return;
    }
    let px = img.pixel_mut(x as u32, y as u32);
    for c in 0..4 {
        px[c] = blend_channel(px[c], color[c], coverage);
    }
}

fn line_aa(img: &mut Bgra8, mut pt1: (i64, i64), mut pt2: (i64, i64), color: &[u8; 4]) {
    let size0 = (i64::from(img.width), i64::from(img.height));
    let size = (size0.0 << XY_SHIFT, size0.1 << XY_SHIFT);
    if !clip_line(size.0, size.1, &mut pt1, &mut pt2) {
        return;
    }
    let dx0 = pt2.0 - pt1.0;
    let dy0 = pt2.1 - pt1.1;
    let ax = dx0.abs();
    let ay = dy0.abs();
    let (mut dy, mut dx) = (dy0, dx0);
    if ax > ay {
        if dx0 < 0 {
            std::mem::swap(&mut pt1, &mut pt2);
            dy = -dy;
        }
    } else if dy0 < 0 {
        std::mem::swap(&mut pt1, &mut pt2);
        dx = -dx;
    }

    let (x_step, y_step, mut ecount, slope_src, end_i, end_j, horizontal) = if ax > ay {
        let y_step = ((dy as u64).wrapping_shl(XY_SHIFT as u32) as i64) / (ax | 1);
        pt2.0 += XY_ONE;
        let ecount = ((pt2.0 >> XY_SHIFT) - (pt1.0 >> XY_SHIFT)) as i32;
        let j = -(pt1.0 & (XY_ONE - 1));
        pt1.1 += ((y_step * j) >> XY_SHIFT) + (XY_ONE >> 1);
        let end_i = (pt1.0 >> (XY_SHIFT - 7)) & 0x78;
        let end_j = (pt2.0 >> (XY_SHIFT - 7)) & 0x78;
        (XY_ONE, y_step, ecount, y_step, end_i, end_j, true)
    } else {
        let x_step = ((dx as u64).wrapping_shl(XY_SHIFT as u32) as i64) / (ay | 1);
        pt2.1 += XY_ONE;
        let ecount = ((pt2.1 >> XY_SHIFT) - (pt1.1 >> XY_SHIFT)) as i32;
        let j = -(pt1.1 & (XY_ONE - 1));
        pt1.0 += ((x_step * j) >> XY_SHIFT) + (XY_ONE >> 1);
        let end_i = (pt1.1 >> (XY_SHIFT - 7)) & 0x78;
        let end_j = (pt2.1 >> (XY_SHIFT - 7)) & 0x78;
        (x_step, XY_ONE, ecount, x_step, end_i, end_j, false)
    };

    let mut slope = ((slope_src >> (XY_SHIFT - 5)) & 0x3f) as i32;
    if slope_src < 0 {
        slope ^= 0x3f;
    }
    let slope = if slope & 0x20 != 0 {
        0x100
    } else {
        i32::from(SLOPE_CORR[slope as usize])
    };

    let end_i = end_i as i32;
    let end_j = end_j as i32;
    let t0 = slope << 7;
    let t1 = ((0x78 - end_i) | 4) * slope;
    let t2 = (end_j | 4) * slope;
    let mut ep = [0i32; 9];
    ep[0] = 0;
    ep[8] = slope;
    ep[1] = (((((end_j - end_i) & 0x78) | 4) * slope) >> 8) & 0x1ff;
    ep[3] = ep[1];
    ep[2] = (t1 >> 8) & 0x1ff;
    ep[4] = (((((end_j - end_i) + 0x80) | 4) * slope) >> 8) & 0x1ff;
    ep[5] = ((t1 + t0) >> 8) & 0x1ff;
    ep[6] = (t2 >> 8) & 0x1ff;
    ep[7] = ((t2 + t0) >> 8) & 0x1ff;

    let mut scount = 0i32;
    if horizontal {
        let mut x = (pt1.0 >> XY_SHIFT) as i32;
        let mut y_acc = pt1.1;
        while ecount >= 0 {
            if (x as u32) < img.width {
                let y = (y_acc >> XY_SHIFT) as i32 - 1;
                let ep_corr = ep[((((scount >= 2) as i32 + 1) & (scount | 2)) * 3
                    + (((ecount >= 2) as i32 + 1) & (ecount | 2)))
                    as usize];
                let dist = ((y_acc >> (XY_SHIFT - 5)) & 31) as usize;
                let cover = |band: usize| ((ep_corr * FILTER[band]) >> 8) & 0xff;
                put_aa(img, x, y, color, cover(dist + 32));
                put_aa(img, x, y + 1, color, cover(dist));
                put_aa(img, x, y + 2, color, cover(63 - dist));
            }
            x += 1;
            y_acc += y_step;
            scount += 1;
            ecount -= 1;
        }
    } else {
        let mut y = (pt1.1 >> XY_SHIFT) as i32;
        let mut x_acc = pt1.0;
        while ecount >= 0 {
            if (y as u32) < img.height {
                let x = (x_acc >> XY_SHIFT) as i32 - 1;
                let ep_corr = ep[((((scount >= 2) as i32 + 1) & (scount | 2)) * 3
                    + (((ecount >= 2) as i32 + 1) & (ecount | 2)))
                    as usize];
                let dist = ((x_acc >> (XY_SHIFT - 5)) & 31) as usize;
                let cover = |band: usize| ((ep_corr * FILTER[band]) >> 8) & 0xff;
                put_aa(img, x, y, color, cover(dist + 32));
                put_aa(img, x + 1, y, color, cover(dist));
                put_aa(img, x + 2, y, color, cover(63 - dist));
            }
            y += 1;
            x_acc += x_step;
            scount += 1;
            ecount -= 1;
        }
    }
}

struct Edge {
    idx: i32,
    di: i32,
    x: i64,
    dx: i64,
    ye: i32,
}

fn fill_convex_poly(img: &mut Bgra8, v: &[(i64, i64)], color: &[u8; 4]) {
    let npts = v.len();
    if npts == 0 {
        return;
    }
    let shift = XY_SHIFT;
    let delta = (1_i32 << shift) >> 1;
    let delta1 = XY_ONE - 1;
    let delta2 = 0i64;

    let mut prev = v[npts - 1];
    let mut xmin = v[0].0;
    let mut xmax = v[0].0;
    let mut ymin = v[0].1;
    let mut ymax = v[0].1;
    let mut imin = 0i32;
    for (i, &p) in v.iter().enumerate() {
        if p.1 < ymin {
            ymin = p.1;
            imin = i as i32;
        }
        ymax = ymax.max(p.1);
        xmax = xmax.max(p.0);
        xmin = xmin.min(p.0);
        line_aa(img, prev, p, color);
        prev = p;
    }

    xmin = (xmin + i64::from(delta)) >> shift;
    xmax = (xmax + i64::from(delta)) >> shift;
    ymin = (ymin + i64::from(delta)) >> shift;
    ymax = (ymax + i64::from(delta)) >> shift;
    let width = i64::from(img.width);
    let height = i64::from(img.height);
    if npts < 3 || xmax < 0 || ymax < 0 || xmin >= width || ymin >= height {
        return;
    }
    ymax = ymax.min(height - 1);

    let mut edge = [
        Edge {
            idx: imin,
            di: 1,
            x: -XY_ONE,
            dx: 0,
            ye: ymin as i32,
        },
        Edge {
            idx: imin,
            di: npts as i32 - 1,
            x: -XY_ONE,
            dx: 0,
            ye: ymin as i32,
        },
    ];
    let mut y = ymin as i32;
    let mut edges_left = npts as i32;
    loop {
        if y < ymax as i32 || y == ymin as i32 {
            for side in &mut edge {
                if y < side.ye {
                    continue;
                }
                let mut idx0 = side.idx;
                let di = side.di;
                let mut idx = idx0 + di;
                if idx >= npts as i32 {
                    idx -= npts as i32;
                }
                loop {
                    let prev_edges = edges_left;
                    edges_left -= 1;
                    if prev_edges <= 0 {
                        break;
                    }
                    let ty = ((v[idx as usize].1 + i64::from(delta)) >> shift) as i32;
                    if ty > y {
                        let xs = v[idx0 as usize].0;
                        let xe = v[idx as usize].0;
                        let span = i64::from(ty - y);
                        side.ye = ty;
                        side.dx = ((xe - xs) * 2 + span) / (2 * span);
                        side.x = xs;
                        side.idx = idx;
                        break;
                    }
                    idx0 = idx;
                    idx += di;
                    if idx >= npts as i32 {
                        idx -= npts as i32;
                    }
                }
            }
        }
        if edges_left < 0 {
            break;
        }
        if y >= 0 {
            let (left, right) = if edge[0].x > edge[1].x {
                (1, 0)
            } else {
                (0, 1)
            };
            let mut xx1 = ((edge[left].x + delta1) >> XY_SHIFT) as i32;
            let mut xx2 = ((edge[right].x + delta2) >> XY_SHIFT) as i32;
            if xx2 >= 0 && xx1 < img.width as i32 {
                if xx1 < 0 {
                    xx1 = 0;
                }
                if xx2 >= img.width as i32 {
                    xx2 = img.width as i32 - 1;
                }
                for x in xx1..=xx2 {
                    img.pixel_mut(x as u32, y as u32).copy_from_slice(color);
                }
            }
        }
        edge[0].x += edge[0].dx;
        edge[1].x += edge[1].dx;
        y += 1;
        if y > ymax as i32 {
            break;
        }
    }
}
