//! Deterministic randomness: the same seed always gives the same poster,
//! at any resolution.

/// SplitMix64 – small, fast and good enough for drawing
#[derive(Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in 0..1
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }

    /// Uniform in 0..n (n > 0)
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n.max(1) as u64) as usize
    }

    pub fn chance(&mut self, p: f32) -> bool {
        self.f32() < p
    }

    /// A random element of a non-empty slice
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

/// 2D Perlin noise with a seeded permutation table
pub struct Noise {
    perm: [u8; 512],
}

impl Noise {
    pub fn new(seed: u64) -> Self {
        let mut table: [u8; 256] = std::array::from_fn(|i| i as u8);
        let mut rng = Rng::new(seed);
        for i in (1..256).rev() {
            table.swap(i, rng.below(i + 1));
        }
        Self {
            perm: std::array::from_fn(|i| table[i & 255]),
        }
    }

    fn gradient(&self, ix: i32, iy: i32, dx: f32, dy: f32) -> f32 {
        let h = self.perm[self.perm[(ix & 255) as usize] as usize + (iy & 255) as usize];
        // Eight directions
        match h & 7 {
            0 => dx + dy,
            1 => dx - dy,
            2 => -dx + dy,
            3 => -dx - dy,
            4 => dx,
            5 => -dx,
            6 => dy,
            _ => -dy,
        }
    }

    /// Roughly -1…1, smooth, 0 on integer grid points
    pub fn get(&self, x: f32, y: f32) -> f32 {
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let (ix, iy) = (x0 as i32, y0 as i32);
        let fade = |t: f32| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
        let (u, v) = (fade(fx), fade(fy));
        let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
        let n00 = self.gradient(ix, iy, fx, fy);
        let n10 = self.gradient(ix + 1, iy, fx - 1.0, fy);
        let n01 = self.gradient(ix, iy + 1, fx, fy - 1.0);
        let n11 = self.gradient(ix + 1, iy + 1, fx - 1.0, fy - 1.0);
        lerp(lerp(n00, n10, u), lerp(n01, n11, u), v) * 1.4
    }

    /// Fractal noise: `octaves` layers, each twice as fine and `gain` as strong
    pub fn fbm(&self, x: f32, y: f32, octaves: u32, gain: f32) -> f32 {
        let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
        for octave in 0..octaves {
            // Shift each octave so the grid points don't line up
            let shift = octave as f32 * 17.3;
            sum += amp * self.get(x * freq + shift, y * freq - shift);
            norm += amp;
            amp *= gain;
            freq *= 2.0;
        }
        sum / norm.max(1e-6)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_numbers() {
        let (mut a, mut b) = (Rng::new(7), Rng::new(7));
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        assert_ne!(Rng::new(1).next_u64(), Rng::new(2).next_u64());
    }

    #[test]
    fn noise_stays_in_range() {
        let noise = Noise::new(3);
        for i in 0..2000 {
            let v = noise.fbm(i as f32 * 0.137, i as f32 * 0.071, 3, 0.5);
            assert!((-1.2..=1.2).contains(&v), "{v}");
        }
    }
}
