/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

pub trait BaseUnit: Sized + 'static {
    const LIST: &[Self];
    const REFERENCE: Self;
    const POWER: i32 = 1;

    fn scale(&self) -> Vec<Self>;

    fn multiplier(&self) -> f64;

    fn offset(&self) -> f64 {
        0.0
    }

    fn linearize(&self, n: f64) -> f64 {
        n
    }

    fn delinearize(&self, n: f64) -> f64 {
        n
    }

    fn mul_pow(self, rhs: Self, p: i32, q: i32) -> (f64, Self) {
        (
            self.multiplier().powi(p) * rhs.multiplier().powi(q),
            Self::REFERENCE,
        )
    }

    fn normalize(&self) -> Self {
        Self::REFERENCE
    }
}
