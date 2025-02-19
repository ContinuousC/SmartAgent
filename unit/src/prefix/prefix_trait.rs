/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

pub trait Prefix: Sized + 'static {
    const BASE: u64;
    const SCALE: &'static [Self];

    fn from_power(n: i64) -> (i64, Self);
    fn power(&self) -> i64;
    fn prefix(&self) -> &'static str;

    fn multiplier(&self) -> f64 {
        (Self::BASE as f64).powf(self.power() as f64)
    }

    fn powi(self, n: i32) -> (f64, Self) {
        let (m, p) = Self::from_power(self.power() * n as i64);
        ((Self::BASE as f64).powi(m as i32), p)
    }

    fn mul_pow(self, p: i64, rhs: Self, q: i64) -> (f64, Self) {
        let e = match p + q == 0 {
            true => 0.0,
            false => {
                (p * self.power() + q * rhs.power()) as f64 / (p + q) as f64
            }
        };

        let (m, f) = Self::from_power(e.trunc() as i64);
        (e.fract() * (Self::BASE as f64).powi(m as i32), f)
    }
}

/* Operations on prefixes. */
/*
macro_rules! impl_operations {
    ($type:ty) => {

    impl Mul<$type> for $type {
        type Output = Option<Self>;
        fn mul(self, rhs: Self) -> (f64,Self) {
        match Self::from_power(self.power() + rhs.power()) {
            (m,p) => ((self::BASE as f64).powi(m as i32), p)
        }
        }
    }

    impl Div<$type> for $type {
        type Output = (f64,Self);
        fn div(self, rhs: Self) -> (f64,Self) {
        match Self::from_power(self.power() - rhs.power()) {
            (m,p) => ((self::BASE as f64).powi(m as i32), p)
        }
        }
    }

    }
}

impl_operations!(SiPrefix);
impl_operations!(FracPrefix);
impl_operations!(DecPrefix);
impl_operations!(BinPrefix);
*/
