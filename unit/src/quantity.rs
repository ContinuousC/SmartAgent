/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::ops::{Add, Div, Mul, Sub};

use serde::{Deserialize, Serialize};

use crate::parser::parse_quantity;

use super::error::UnitError;
use super::{Dimension, Unit, NEUTRAL_UNIT};

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Quantity(pub f64, pub Unit);

impl Quantity {
    pub fn new(val: f64, unit: Unit) -> Self {
        Quantity(val, unit)
    }

    pub fn parse(input: &str) -> Result<Self, UnitError> {
        parse_quantity(input)
    }

    pub fn from_unit(unit: Unit) -> Self {
        Quantity(1.0, unit)
    }

    pub fn from_value(value: f64) -> Self {
        Quantity(value, NEUTRAL_UNIT)
    }

    pub fn normalize(self) -> Result<Self, UnitError> {
        self.convert(&self.1.normalize())
    }

    pub fn autoscale(&self) -> Result<Self, UnitError> {
        if self.0 == 0.0 {
            return self.normalize();
        }
        let scale = self.1.scale();
        for unit in scale[1..].iter().rev() {
            let val = self.1.convert(unit, self.0)?;
            if val >= 1.0 {
                return Ok(Quantity(val, *unit));
            }
        }
        let unit = *scale.first().unwrap();
        let val = self.1.convert(&unit, self.0)?;
        Ok(Quantity(val, unit))
    }

    pub fn convert(self, unit: &Unit) -> Result<Self, UnitError> {
        Ok(Quantity(self.1.convert(unit, self.0)?, *unit))
    }

    pub fn powi(self, n: i32) -> Result<Self, UnitError> {
        let Quantity(m, u) = self.1.powi(n)?;
        Ok(Quantity(m * self.0.powi(n), u))
    }

    /* Note: we cannot implement the trait, because it does not allow
    for error conditions. */
    pub fn partial_cmp(
        &self,
        rhs: &Self,
    ) -> Result<Option<Ordering>, UnitError> {
        Ok(self.0.partial_cmp(&rhs.convert(&self.1)?.0))
    }

    pub fn dimension(&self) -> Dimension {
        self.1.dimension()
    }
}

impl Display for Quantity {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "{} {}", self.0, self.1)
    }
}

impl Add<Quantity> for Quantity {
    type Output = Result<Quantity, UnitError>;
    fn add(self, rhs: Quantity) -> Result<Quantity, UnitError> {
        Ok(Quantity(
            self.1.delinearize(
                self.1.linearize(self.0)
                    + self.1.linearize(rhs.1.convert(&self.1, rhs.0)?),
            ),
            self.1,
        ))
    }
}

impl Sub<Quantity> for Quantity {
    type Output = Result<Quantity, UnitError>;
    fn sub(self, rhs: Quantity) -> Result<Quantity, UnitError> {
        Ok(Quantity(
            self.1.delinearize(
                self.1.linearize(self.0)
                    - self.1.linearize(rhs.1.convert(&self.1, rhs.0)?),
            ),
            self.1,
        ))
    }
}

impl Mul<Quantity> for Quantity {
    type Output = Result<Quantity, UnitError>;
    fn mul(self, rhs: Quantity) -> Result<Quantity, UnitError> {
        let Quantity(m, u) = (self.1 * rhs.1)?;
        Ok(Quantity(
            u.delinearize(
                m * self.1.linearize(self.0) * rhs.1.linearize(rhs.0),
            ),
            u,
        ))
    }
}

impl Div<Quantity> for Quantity {
    type Output = Result<Quantity, UnitError>;
    fn div(self, rhs: Quantity) -> Result<Quantity, UnitError> {
        let Quantity(m, u) = (self.1 / rhs.1)?;
        Ok(Quantity(
            u.delinearize(
                m * self.1.linearize(self.0) / rhs.1.linearize(rhs.0),
            ),
            u,
        ))
    }
}

impl Mul<f64> for Quantity {
    type Output = Quantity;
    fn mul(self, rhs: f64) -> Quantity {
        Quantity(self.1.delinearize(self.1.linearize(self.0) * rhs), self.1)
    }
}

impl Div<f64> for Quantity {
    type Output = Quantity;
    fn div(self, rhs: f64) -> Quantity {
        Quantity(self.1.delinearize(self.1.linearize(self.0) / rhs), self.1)
    }
}
