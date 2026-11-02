//! `std::mt19937` and libstdc++ `uniform_real_distribution<float>`.
//!
//! Star layouts are generated once from `star_seed` with this pair (v2
//! `frame_renderer.cpp`). GCC 16's `generate_canonical<float>` for a
//! power-of-two range takes one `uint32_t`, converts it to `float` and
//! divides by `2^32`. A sample that rounds to exactly 1 is discarded.

/// 32-bit Mersenne Twister, the C++ `std::mt19937` parameter set.
pub struct Mt19937 {
    state: [u32; 624],
    index: usize,
}

impl Mt19937 {
    /// Seed the generator the way `std::mt19937(seed)` does.
    pub fn new(seed: u32) -> Self {
        let mut state = [0u32; 624];
        state[0] = seed;
        for i in 1..624 {
            let prev = state[i - 1];
            state[i] = 1_812_433_253u32
                .wrapping_mul(prev ^ (prev >> 30))
                .wrapping_add(i as u32);
        }
        Mt19937 { state, index: 624 }
    }

    /// Next tempered value.
    pub fn next_u32(&mut self) -> u32 {
        if self.index >= 624 {
            self.twist();
        }
        let mut y = self.state[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y
    }

    fn twist(&mut self) {
        const M: usize = 397;
        const A: u32 = 0x9908_b0df;
        for i in 0..624 {
            let x = (self.state[i] & 0x8000_0000) | (self.state[(i + 1) % 624] & 0x7fff_ffff);
            let mut mixed = x >> 1;
            if x & 1 != 0 {
                mixed ^= A;
            }
            self.state[i] = self.state[(i + M) % 624] ^ mixed;
        }
        self.index = 0;
    }
}

/// Half-open uniform float, libstdc++ `uniform_real_distribution<float>`.
pub struct UniformReal {
    low: f32,
    span: f32,
}

impl UniformReal {
    /// Distribution of `[low, high)`.
    pub fn new(low: f32, high: f32) -> Self {
        UniformReal {
            low,
            span: high - low,
        }
    }

    /// Draw one sample. Retries if the canonical value rounded to 1.
    pub fn sample(&self, rng: &mut Mt19937) -> f32 {
        let unit = loop {
            let value = (rng.next_u32() as f32) / 4_294_967_296.0;
            if value < 1.0 {
                break value;
            }
        };
        unit * self.span + self.low
    }
}
