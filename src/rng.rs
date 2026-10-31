//! Small seedable random source. A fixed seed makes the game tests reproducible; the running bot
//! seeds from the clock.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn seeded(seed: u64) -> Self {
        // The golden ratio constant keeps state(0) from staying zero.
        Self {
            state: seed ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF03_1622_3134_2E85);
        z = (z ^ (z >> 27)).wrapping_mul(0x942F_4893_DF72_2F2D);
        z ^ (z >> 31)
    }

    pub fn index(&mut self, len: usize) -> usize {
        if len == 0 {
            return 0;
        }
        (self.next_u64() % len as u64) as usize
    }

    pub fn shuffle(&mut self, values: &mut [usize]) {
        for i in (1..values.len()).rev() {
            let j = self.index(i + 1);
            values.swap(i, j);
        }
    }
}
