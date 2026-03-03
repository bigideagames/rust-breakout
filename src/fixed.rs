// ============================================================================
// Fixed-Point Arithmetic (Q48.16)
// ============================================================================

const FRAC_BITS: i32 = 16;
pub const FRAC_SCALE: i64 = 1 << FRAC_BITS; // 65536

#[derive(Clone, Copy, Debug)]
pub struct Fixed {
    pub raw: i64,
}

impl Fixed {
    pub const ZERO: Fixed = Fixed { raw: 0 };
    pub const ONE: Fixed = Fixed { raw: FRAC_SCALE };
    pub const TWO: Fixed = Fixed { raw: 2 * FRAC_SCALE };
    pub const HALF: Fixed = Fixed { raw: FRAC_SCALE / 2 };

    pub const fn from_raw(raw: i64) -> Self {
        Self { raw }
    }

    pub const fn from_i32(v: i32) -> Self {
        Self {
            raw: (v as i64) << FRAC_BITS,
        }
    }

    pub fn from_f64(v: f64) -> Self {
        Self {
            raw: (v * FRAC_SCALE as f64) as i64,
        }
    }

    /// Convert to f64 for rendering boundary only
    pub fn to_f64(self) -> f64 {
        self.raw as f64 / FRAC_SCALE as f64
    }

    pub fn abs(self) -> Self {
        Self {
            raw: self.raw.abs(),
        }
    }

    /// Fixed-point square root using Newton's method on i128
    pub fn sqrt(self) -> Self {
        if self.raw <= 0 {
            return Self::ZERO;
        }
        let val = (self.raw as i128) * (FRAC_SCALE as i128);
        let mut guess = val;
        let mut prev;
        loop {
            if guess == 0 {
                return Self::ZERO;
            }
            prev = guess;
            guess = (guess + val / guess) / 2;
            if guess >= prev {
                break;
            }
        }
        Self { raw: prev as i64 }
    }

    pub fn min(self, other: Self) -> Self {
        if self.raw < other.raw {
            self
        } else {
            other
        }
    }

    pub fn max(self, other: Self) -> Self {
        if self.raw > other.raw {
            self
        } else {
            other
        }
    }

    pub fn clamp(self, lo: Self, hi: Self) -> Self {
        self.max(lo).min(hi)
    }
}

// --- Operator Overloads ---

impl std::ops::Add for Fixed {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self { raw: self.raw + rhs.raw }
    }
}

impl std::ops::AddAssign for Fixed {
    fn add_assign(&mut self, rhs: Self) {
        self.raw += rhs.raw;
    }
}

impl std::ops::Sub for Fixed {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self { raw: self.raw - rhs.raw }
    }
}

impl std::ops::SubAssign for Fixed {
    fn sub_assign(&mut self, rhs: Self) {
        self.raw -= rhs.raw;
    }
}

impl std::ops::Mul for Fixed {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self {
            raw: ((self.raw as i128 * rhs.raw as i128) >> FRAC_BITS) as i64,
        }
    }
}

impl std::ops::MulAssign for Fixed {
    fn mul_assign(&mut self, rhs: Self) {
        self.raw = ((self.raw as i128 * rhs.raw as i128) >> FRAC_BITS) as i64;
    }
}

impl std::ops::Div for Fixed {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        Self {
            raw: (((self.raw as i128) << FRAC_BITS) / rhs.raw as i128) as i64,
        }
    }
}

impl std::ops::Neg for Fixed {
    type Output = Self;
    fn neg(self) -> Self {
        Self { raw: -self.raw }
    }
}

impl PartialEq for Fixed {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl Eq for Fixed {}

impl PartialOrd for Fixed {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Fixed {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.raw.cmp(&other.raw)
    }
}

// ============================================================================
// Sin/Cos Lookup Table (256 entries covering one full period)
// ============================================================================

const TRIG_TABLE_SIZE: usize = 256;

static mut SIN_LUT: [i64; TRIG_TABLE_SIZE] = [0i64; TRIG_TABLE_SIZE];
static mut SIN_LUT_INIT: bool = false;

pub fn ensure_sin_lut() {
    unsafe {
        if SIN_LUT_INIT {
            return;
        }
        for i in 0..TRIG_TABLE_SIZE {
            let angle =
                2.0 * std::f64::consts::PI * (i as f64) / (TRIG_TABLE_SIZE as f64);
            SIN_LUT[i] = (angle.sin() * FRAC_SCALE as f64) as i64;
        }
        SIN_LUT_INIT = true;
    }
}

/// Fixed-point sin using lookup table. Input is fixed-point radians.
pub fn fixed_sin(angle: Fixed) -> Fixed {
    ensure_sin_lut();
    let two_pi_raw = Fixed::from_f64(std::f64::consts::TAU).raw;
    let mut a = angle.raw % two_pi_raw;
    if a < 0 {
        a += two_pi_raw;
    }
    let index =
        ((a as i128 * TRIG_TABLE_SIZE as i128) / two_pi_raw as i128) as usize % TRIG_TABLE_SIZE;
    unsafe { Fixed::from_raw(SIN_LUT[index]) }
}

/// Fixed-point cos via sin(angle + π/2)
pub fn fixed_cos(angle: Fixed) -> Fixed {
    fixed_sin(angle + Fixed::from_f64(std::f64::consts::FRAC_PI_2))
}
