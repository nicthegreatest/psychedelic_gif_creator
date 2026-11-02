//! 8-bit Gaussian blur matching OpenCV 4.13 `GaussianBlur` (`Size(0, 0)`, sigma).
//!
//! Kernel size is `cvRound(sigma * 6 + 1) | 1`. The coefficients are the
//! fixed-point (Q8) kernel: a softdouble `exp` polynomial, normalised, then
//! error-diffused so the taps sum to 256. The filter is separable.
//! Horizontal products are saturating Q8; the vertical pass sums Q8×Q8 and
//! rounds with `(sum + 32768) >> 16`. The border is `BORDER_REFLECT_101`.
//! A side of length 1 forces that kernel size to 1, and a 1×1 kernel is a copy.

use super::warp::{border_index, Border};
use crate::image::Bgr8;

/// `exp` from OpenCV `softfloat.cpp` (`f64_exp`), evaluated with IEEE f64
/// operations and no fused multiply-add. That is what `softdouble::exp` does.
fn soft_exp(x: f64) -> f64 {
    const A0_SCALE: f64 = f64::from_bits(0x3f83_ce0f_3e46_f431);
    const PRESCALE: f64 = f64::from_bits(0x3ff7_1547_652b_82fe) * 64.0;
    const POSTSCALE: f64 = 1.0 / 64.0;
    const EXP_MAX: f64 = 3000.0 * 64.0;
    const TAB: [u64; 64] = [
        0x3ff0_0000_0000_0000,
        0x3ff0_2c9a_3e77_8061,
        0x3ff0_59b0_d315_8574,
        0x3ff0_8745_1875_9bc8,
        0x3ff0_b558_6cf9_890f,
        0x3ff0_e3ec_32d3_d1a2,
        0x3ff1_1301_d012_5b51,
        0x3ff1_429a_aea9_2de0,
        0x3ff1_72b8_3c7d_517b,
        0x3ff1_a35b_eb6f_cb75,
        0x3ff1_d487_3168_b9aa,
        0x3ff2_063b_8862_8cd6,
        0x3ff2_387a_6e75_6238,
        0x3ff2_6b45_65e2_7cdd,
        0x3ff2_9e9d_f51f_dee1,
        0x3ff2_d285_a6e4_030b,
        0x3ff3_06fe_0a31_b715,
        0x3ff3_3c08_b264_16ff,
        0x3ff3_71a7_373a_a9cb,
        0x3ff3_a7db_34e5_9ff7,
        0x3ff3_dea6_4c12_3422,
        0x3ff4_160a_21f7_2e2a,
        0x3ff4_4e08_6061_892d,
        0x3ff4_86a2_b5c1_3cd0,
        0x3ff4_bfda_d536_2a27,
        0x3ff4_f9b2_769d_2ca7,
        0x3ff5_342b_569d_4f82,
        0x3ff5_6f47_36b5_27da,
        0x3ff5_ab07_dd48_5429,
        0x3ff5_e76f_15ad_2148,
        0x3ff6_247e_b03a_5585,
        0x3ff6_6238_8255_2225,
        0x3ff6_a09e_667f_3bcd,
        0x3ff6_dfb2_3c65_1a2f,
        0x3ff7_1f75_e8ec_5f74,
        0x3ff7_5feb_5642_67c9,
        0x3ff7_a114_73eb_0187,
        0x3ff7_e2f3_36cf_4e62,
        0x3ff8_2589_994c_ce13,
        0x3ff8_68d9_9b44_92ed,
        0x3ff8_ace5_422a_a0db,
        0x3ff8_f1ae_9915_7736,
        0x3ff9_3737_b0cd_c5e5,
        0x3ff9_7d82_9fde_4e50,
        0x3ff9_c491_82a3_f090,
        0x3ffa_0c66_7b5d_e565,
        0x3ffa_5503_b23e_255d,
        0x3ffa_9e6b_5579_fdbf,
        0x3ffa_e89f_995a_d3ad,
        0x3ffb_33a2_b84f_15fb,
        0x3ffb_7f76_f2fb_5e47,
        0x3ffb_cc1e_904b_c1d2,
        0x3ffc_199b_dd85_529c,
        0x3ffc_67f1_2e57_d14b,
        0x3ffc_b720_dcef_9069,
        0x3ffd_072d_4a07_897c,
        0x3ffd_5818_dcfb_a487,
        0x3ffd_a9e6_03db_3285,
        0x3ffd_fc97_337b_9b5f,
        0x3ffe_502e_e78b_3ff6,
        0x3ffe_a4af_a2a4_90da,
        0x3ffe_fa1b_ee61_5a27,
        0x3fff_5076_5b6e_4540,
        0x3fff_a7c1_819e_90d8,
    ];

    if x.is_nan() {
        return f64::NAN;
    }
    if x.is_infinite() {
        return if x.is_sign_positive() { x } else { 0.0 };
    }

    let a5 = 1.0 / A0_SCALE;
    let a4 = f64::from_bits(0x3fe6_2e42_fefa_39f1) / A0_SCALE;
    let a3 = f64::from_bits(0x3fce_bfbd_ff82_a45a) / A0_SCALE;
    let a2 = f64::from_bits(0x3fac_6b08_d81f_ec75) / A0_SCALE;
    let a1 = f64::from_bits(0x3f83_b2a7_2b4f_3cd3) / A0_SCALE;
    let a0 = f64::from_bits(0x3f55_e7aa_1566_c2a4) / A0_SCALE;

    // Biased exponent > 1023 + 10 means |x| >= 1024.
    let exp_bits = (x.to_bits() >> 52) & 0x7ff;
    let mut x0 = if exp_bits > 1023 + 10 {
        if x.is_sign_negative() {
            -EXP_MAX
        } else {
            EXP_MAX
        }
    } else {
        x * PRESCALE
    };
    let val0 = cv_round(x0) as i32;
    let t = ((val0 >> 6) + 1023).clamp(0, 2047) as u64;
    let buf = f64::from_bits(t << 52);
    x0 = (x0 - cv_round_f64(x0)) * POSTSCALE;
    let mut poly = a0 * x0 + a1;
    poly = poly * x0 + a2;
    poly = poly * x0 + a3;
    poly = poly * x0 + a4;
    poly = poly * x0 + a5;
    buf * A0_SCALE * f64::from_bits(TAB[(val0 & 63) as usize]) * poly
}

/// `cvRound(double)`: half to even.
fn cv_round(v: f64) -> i64 {
    v.round_ties_even() as i64
}

fn cv_round_f64(v: f64) -> f64 {
    v.round_ties_even()
}

/// Q8 Gaussian taps that sum to 256. `n` is odd and at least 1.
fn kernel_q8(n: usize, sigma: f64) -> Vec<u16> {
    let n2 = n / 2;
    let sigma_x = if sigma > 0.0 {
        sigma
    } else {
        (n as f64) * 0.15 + 0.35
    };
    let scale2 = -0.125 / (sigma_x * sigma_x);
    let mut values = Vec::with_capacity(n2);
    let mut sum = 0.0;
    let mut x = 1 - n as i32;
    for _ in 0..n2 {
        let t = soft_exp(f64::from(x * x) * scale2);
        values.push(t);
        sum += t;
        x += 2;
    }
    sum *= 2.0;
    sum += 1.0;
    let mul = 1.0 / sum;
    let mut exact = vec![0.0; n];
    for (i, t) in values.iter().enumerate() {
        let v = t * mul;
        exact[i] = v;
        exact[n - 1 - i] = v;
    }
    exact[n2] = mul;

    let mut out = vec![0u16; n];
    let mut err = 0.0;
    let mut side = 0i64;
    for i in 0..n2 {
        let adj = exact[i] * 256.0 + err;
        let v0 = cv_round(adj);
        err = adj - (v0 as f64);
        let bits = v0 as u16;
        out[i] = bits;
        out[n - 1 - i] = bits;
        side += v0;
    }
    let centre = 256 - side * 2;
    out[n2] = centre as u16;
    out
}

fn ksize(sigma: f64) -> i32 {
    (cv_round(sigma * 6.0 + 1.0) as i32) | 1
}

fn reflect(p: i32, len: i32) -> usize {
    border_index(p, len, Border::Reflect101).expect("reflect-101 always maps") as usize
}

/// Blur `src` with OpenCV's 8-bit Gaussian. `sigma <= 0` returns a copy.
pub fn gaussian_blur(src: &Bgr8, sigma: f64) -> Bgr8 {
    if src.width == 0 || src.height == 0 || sigma <= 0.0 {
        return src.clone();
    }
    let mut kx = if src.width == 1 { 1 } else { ksize(sigma) };
    let mut ky = if src.height == 1 { 1 } else { ksize(sigma) };
    if kx < 1 {
        kx = 1;
    }
    if ky < 1 {
        ky = 1;
    }
    if kx == 1 && ky == 1 {
        return src.clone();
    }
    let kernel_x = kernel_q8(kx as usize, sigma);
    let kernel_y = if ky == kx {
        kernel_x.clone()
    } else {
        kernel_q8(ky as usize, sigma)
    };

    let (w, h) = (src.width as usize, src.height as usize);
    let cn = Bgr8::CHANNELS;
    let mut horizontal = vec![0u16; w * h * cn];
    let rx = (kernel_x.len() / 2) as i32;
    for y in 0..h {
        let row = &src.data[y * w * cn..(y + 1) * w * cn];
        let dst = &mut horizontal[y * w * cn..(y + 1) * w * cn];
        for x in 0..w as i32 {
            for c in 0..cn {
                let mut sum = 0u32;
                for (k, &coeff) in kernel_x.iter().enumerate() {
                    let sx = reflect(x + k as i32 - rx, w as i32);
                    let pixel = u32::from(row[sx * cn + c]);
                    let prod = (u32::from(coeff) * pixel).min(65535);
                    sum = sum.saturating_add(prod);
                }
                dst[x as usize * cn + c] = sum.min(65535) as u16;
            }
        }
    }

    let mut out = Bgr8::new(src.width, src.height);
    let ry = (kernel_y.len() / 2) as i32;
    for y in 0..h as i32 {
        for x in 0..w * cn {
            let mut sum = 0u32;
            for (k, &coeff) in kernel_y.iter().enumerate() {
                let sy = reflect(y + k as i32 - ry, h as i32);
                let value = u32::from(horizontal[sy * w * cn + x]);
                sum = sum.saturating_add(u32::from(coeff).saturating_mul(value));
            }
            let rounded = sum.wrapping_add(32768) >> 16;
            out.data[y as usize * w * cn + x] = rounded.min(255) as u8;
        }
    }
    out
}
