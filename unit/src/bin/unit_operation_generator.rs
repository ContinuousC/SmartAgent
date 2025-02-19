/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::BTreeMap;
use std::ops::{Div, Mul};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
enum Dimension {
    /* SI base dimensions. */
    Length,
    Mass,
    Time,
    Current,
    Temperature,
    //Substance,
    //Luminance,
    /* Additional base dimensions */
    Information,
    Operations,
    Rotations,
}

fn main() {
    /* Dimension and unit definitions. */

    let uc: BTreeMap<&str, char> = vec![
        ('u', "DimensionlessUnit"),
        ('i', "InformationUnit"),
        ('n', "OperationUnit"),
        ('l', "LengthUnit"),
        ('m', "MassUnit"),
        ('t', "TimeUnit"),
        ('t', "TemperatureUnit"),
        ('f', "FrequencyUnit"),
        ('f', "FanSpeedUnit"),
        ('a', "CurrentUnit"),
        ('v', "PotentialUnit"),
        ('w', "PowerUnit"),
        ('c', "ConductivityUnit"),
        ('r', "ResistanceUnit"),
    ]
    .into_iter()
    .map(|(c, u)| (u, c))
    .collect();

    let units: BTreeMap<&str, Unit> = vec![
        (
            "Dimensionless",
            Unit::from(vec![], vec![("DimensionlessUnit", 1)]),
        ),
        (
            "Information",
            Unit::from(
                vec![(Dimension::Information, 1)],
                vec![("InformationUnit", 1)],
            ),
        ),
        (
            "Operations",
            Unit::from(
                vec![(Dimension::Operations, 1)],
                vec![("OperationUnit", 1)],
            ),
        ),
        //("Rotations",    Unit::from(vec![(Dimension::Rotations,1)])),
        (
            "Length",
            Unit::from(vec![(Dimension::Length, 1)], vec![("LengthUnit", 1)]),
        ),
        (
            "Area",
            Unit::from(vec![(Dimension::Length, 2)], vec![("LengthUnit", 2)]),
        ),
        (
            "Volume",
            Unit::from(vec![(Dimension::Length, 3)], vec![("LengthUnit", 3)]),
        ),
        (
            "Mass",
            Unit::from(vec![(Dimension::Mass, 1)], vec![("MassUnit", 1)]),
        ),
        (
            "Time",
            Unit::from(vec![(Dimension::Time, 1)], vec![("TimeUnit", 1)]),
        ),
        (
            "TimeSquare",
            Unit::from(vec![(Dimension::Time, 2)], vec![("TimeUnit", 2)]),
        ),
        (
            "Temperature",
            Unit::from(
                vec![(Dimension::Temperature, 1)],
                vec![("TemperatureUnit", 1)],
            ),
        ),
        (
            "Current",
            Unit::from(vec![(Dimension::Current, 1)], vec![("CurrentUnit", 1)]),
        ),
        (
            "Potential",
            Unit::from(
                vec![
                    (Dimension::Mass, 1),
                    (Dimension::Length, 2),
                    (Dimension::Time, -3),
                    (Dimension::Current, -1),
                ],
                vec![("PotentialUnit", 1)],
            ),
        ),
        (
            "Power",
            Unit::from(
                vec![
                    (Dimension::Mass, 1),
                    (Dimension::Length, 2),
                    (Dimension::Time, -3),
                ],
                vec![("PowerUnit", 1)],
            ),
        ),
        (
            "Resistance",
            Unit::from(
                vec![
                    (Dimension::Mass, 1),
                    (Dimension::Length, 2),
                    (Dimension::Time, -3),
                    (Dimension::Current, -2),
                ],
                vec![("ResistanceUnit", 1)],
            ),
        ),
        (
            "Conductivity",
            Unit::from(
                vec![
                    (Dimension::Mass, -1),
                    (Dimension::Length, -2),
                    (Dimension::Time, 3),
                    (Dimension::Current, 2),
                ],
                vec![("ConductivityUnit", 1)],
            ),
        ),
        (
            "Speed",
            Unit::from(
                vec![(Dimension::Length, 1), (Dimension::Time, -1)],
                vec![("LengthUnit", 1), ("TimeUnit", -1)],
            ),
        ),
        (
            "Acceleration",
            Unit::from(
                vec![(Dimension::Length, 1), (Dimension::Time, -2)],
                vec![("LengthUnit", 1), ("TimeUnit", -2)],
            ),
        ),
        (
            "Bandwidth",
            Unit::from(
                vec![(Dimension::Information, 1), (Dimension::Time, -1)],
                vec![("InformationUnit", 1), ("TimeUnit", -1)],
            ),
        ),
        (
            "IOLatency",
            Unit::from(
                vec![(Dimension::Operations, -1), (Dimension::Time, 1)],
                vec![("TimeUnit", 1), ("OperationUnit", -1)],
            ),
        ),
        (
            "IOPerformance",
            Unit::from(
                vec![(Dimension::Operations, 1), (Dimension::Time, -1)],
                vec![("OperationUnit", 1), ("TimeUnit", -1)],
            ),
        ),
        (
            "AvgOpSize",
            Unit::from(
                vec![(Dimension::Information, 1), (Dimension::Operations, -1)],
                vec![("InformationUnit", 1), ("OperationUnit", -1)],
            ),
        ),
        (
            "Frequency",
            Unit::from(vec![(Dimension::Time, -1)], vec![("FrequencyUnit", 1)]),
        ),
        (
            "FanSpeed",
            Unit::from(
                vec![(Dimension::Rotations, 1), (Dimension::Time, -1)],
                vec![("FanSpeedUnit", 1)],
            ),
        ),
        (
            "AbsoluteHumidity",
            Unit::from(
                vec![(Dimension::Mass, 1), (Dimension::Length, -3)],
                vec![("MassUnit", 1), ("LengthUnit", -3)],
            ),
        ),
    ]
    .into_iter()
    .collect();

    let dims: BTreeMap<Dims, &str> = units
        .clone()
        .into_iter()
        .map(|(n, ds)| (ds.dims, n))
        .collect();

    /* Dimension integer power. */
    println!("/* Dimension operations. */");
    println!("impl Dimension {{");
    println!("    fn powi(self, n: i32) -> Result<Dimension,UnitError> {{");
    println!("        match (self,n) {{");
    println!("            (d,1) => Ok(d),");
    for (u, _) in units.iter().filter(|(_, d)| d.dims.is_empty()) {
        println!("            (_,0) => Ok(Dimension::{}),", u);
        println!("            (Dimension::{},_) => Ok(Dimension::{}),", u, u);
    }
    for (u, d) in units.iter().filter(|(_, d)| !d.dims.is_empty()) {
        for n in (-3..=3).filter(|n| *n != 0 && *n != 1) {
            if let Some(o) = dims.get(&(d.dims.clone().powi(n))) {
                println!(
                    "            (Dimension::{},{}) => Ok(Dimension::{}),",
                    u, n, o
                );
            }
        }
    }
    println!("            _ => Err(UnitError::Pow(self,n))");
    println!("        }}");
    println!("    }}");
    println!("}}");

    /* Dimension multiplication. */
    println!();
    println!("impl Mul<Dimension> for Dimension {{");
    println!("    type Output = Result<Dimension,UnitError>;");
    println!(
        "    fn mul(self, rhs: Dimension) -> Result<Dimension,UnitError> {{"
    );
    println!("        match (self,rhs) {{");
    for (u, _) in units.iter().filter(|(_, d)| d.dims.is_empty()) {
        println!("            (Dimension::{},d) => Ok(d),", u);
        println!("            (d,Dimension::{}) => Ok(d),", u);
    }
    for (l, ld) in units.iter().filter(|(_, d)| !d.dims.is_empty()) {
        for (r, rd) in units.iter().filter(|(_, d)| !d.dims.is_empty()) {
            if let Some(o) = dims.get(&(ld.dims.clone() * rd.dims.clone())) {
                println!(
                    "            (Dimension::{},Dimension::{}) => Ok(Dimension::{}),",
                    l, r, o
                );
            }
        }
    }
    println!("            _ => Err(UnitError::Mul(self,rhs))");
    println!("        }}");
    println!("    }}");
    println!("}}");

    /* Dimension division. */
    println!();
    println!("impl Div<Dimension> for Dimension {{");
    println!("    type Output = Result<Dimension,UnitError>;");
    println!(
        "    fn div(self, rhs: Dimension) -> Result<Dimension,UnitError> {{"
    );
    println!("        match (self,rhs) {{");
    for (u, _) in units.iter().filter(|(_, d)| d.dims.is_empty()) {
        println!("            (d,Dimension::{}) => Ok(d),", u);
    }
    for (l, ld) in units.iter() {
        for (r, rd) in units.iter().filter(|(_, d)| !d.dims.is_empty()) {
            if let Some(o) = dims.get(&(ld.dims.clone() / rd.dims.clone())) {
                println!(
                    "            (Dimension::{},Dimension::{}) => Ok(Dimension::{}),",
                    l, r, o
                );
            }
        }
    }
    println!("            _ => Err(UnitError::Div(self,rhs))");
    println!("        }}");
    println!("    }}");
    println!("}}");

    /* Unit integer power. */
    println!();
    println!("/* Unit operations. */");
    println!("impl Unit {{");
    println!("    fn powi(self, n: i32) -> Result<Quantity,UnitError> {{");
    println!("        match (self,n) {{");
    println!("            (u,1) => Ok(Quantity(1.0,u)),");
    for (u, d) in units.iter().filter(|(_, d)| d.dims.is_empty()) {
        println!(
            "            (_,0) => Ok(Quantity(1.0,Unit::{}({}))),",
            u,
            d.base.refs()
        );
        println!(
            "            (Unit::{}(u),n) => Ok(Quantity(u.multiplier().powi(n-1),Unit::{}(u))),",
            u, u
        );
    }
    for (u, d) in units.iter().filter(|(_, d)| !d.dims.is_empty()) {
        for n in (-3..=3).filter(|n| *n != 0 && *n != 1) {
            if let Some(o) = dims.get(&(d.dims.clone().powi(n))) {
                let (p, ms) = d.base.pat("", &uc);
                let (m, t) = ms.powi(n).assemble(&units[o].base);
                println!(
                    "            (Unit::{}({}),{}) => Ok(Quantity({},Unit::{}({}))),",
                    u, p, n, m, o, t
                );
            }
        }
    }
    println!("            _ => Err(UnitError::Pow(self.dimension(),n))");
    println!("        }}");
    println!("    }}");
    println!("}}");

    /* Unit multiplication. */
    println!();
    println!("impl Mul<Unit> for Unit {{");
    println!("    type Output = Result<Quantity,UnitError>;");
    println!("    fn mul(self, rhs: Unit) -> Result<Quantity,UnitError> {{");
    println!("        match (self,rhs) {{");
    for (u, _) in units.iter().filter(|(_, d)| d.dims.is_empty()) {
        println!(
            "            (Unit::{}(d),u) => Ok(Quantity(d.multiplier(), u)),",
            u
        );
        println!(
            "            (u,Unit::{}(d)) => Ok(Quantity(d.multiplier(), u)),",
            u
        );
    }
    for (l, ld) in units.iter().filter(|(_, d)| !d.dims.is_empty()) {
        for (r, rd) in units.iter().filter(|(_, d)| !d.dims.is_empty()) {
            if let Some(o) = dims.get(&(ld.dims.clone() * rd.dims.clone())) {
                let (lp, lms) = ld.base.pat("l", &uc);
                let (rp, rms) = rd.base.pat("r", &uc);
                let (m, t) = (lms * rms).assemble(&units[o].base);
                println!(
                    "            (Unit::{}({}),Unit::{}({})) => Ok(Quantity({},Unit::{}({}))),",
                    l, lp, r, rp, m, o, t
                );
            }
        }
    }
    println!("            _ => Err(UnitError::Mul(self.dimension(),rhs.dimension()))");
    println!("        }}");
    println!("    }}");
    println!("}}");

    /* Unit division. */
    println!();
    println!("impl Div<Unit> for Unit {{");
    println!("    type Output = Result<Quantity,UnitError>;");
    println!("    fn div(self, rhs: Unit) -> Result<Quantity,UnitError> {{");
    println!("        match (self,rhs) {{");
    for (u, _) in units.iter().filter(|(_, d)| d.dims.is_empty()) {
        println!(
            "            (u,Unit::{}(d)) => Ok(Quantity(1.0 / d.multiplier(), u)),",
            u
        );
    }
    for (l, ld) in units.iter().filter(|(_, d)| !d.dims.is_empty()) {
        for (r, rd) in units.iter().filter(|(_, d)| !d.dims.is_empty()) {
            if let Some(o) = dims.get(&(ld.dims.clone() / rd.dims.clone())) {
                let (lp, lms) = ld.base.pat("l", &uc);
                let (rp, rms) = rd.base.pat("r", &uc);
                let (m, t) = (lms / rms).assemble(&units[o].base);
                println!(
                    "            (Unit::{}({}),Unit::{}({})) => Ok(Quantity({},Unit::{}({}))),",
                    l, lp, r, rp, m, o, t
                );
            }
        }
    }
    println!("            _ => Err(UnitError::Div(self.dimension(),rhs.dimension()))");
    println!("        }}");
    println!("    }}");
    println!("}}");

    /* Unwrapped unit integer power. */
    println!();
    println!("/* Whole unit operations for composite unit construction. */");
    println!("impl Unit {{");
    println!(
        "    pub fn powi_unwrapped(self, n: i32) -> Result<Unit,UnitError> {{"
    );
    println!("        match (self,n) {{");
    println!("            (u,1) => Ok(u),");
    println!("            (_,0) => Ok(NEUTRAL_UNIT),");
    println!("            (NEUTRAL_UNIT,_) => Ok(NEUTRAL_UNIT),");
    for (u, d) in units.iter().filter(|(_, d)| !d.dims.is_empty()) {
        for n in (-3..=3).filter(|n| *n != 0 && *n != 1) {
            if let Some(o) = dims.get(&(d.dims.clone().powi(n))) {
                let (p, ms) = d.base.pat("", &uc);
                match ms.powi(n).assemble_unwrapped(&units[o].base) {
                    Some((Some(c), t)) => {
                        println!(
                            "            (Unit::{}({}),{}) => match {} {{",
                            u, p, n, c
                        );
                        println!(
                            "                true => Ok(Unit::{}({})),",
                            o, t
                        );
                        println!("                false => Err(UnitError::CPow(self,n))");
                        println!("            }},");
                    }
                    Some((None, t)) => {
                        println!(
                            "            (Unit::{}({}),{}) => Ok(Unit::{}({})),",
                            u, p, n, o, t
                        );
                    }
                    _ => {}
                }
            }
        }
    }
    println!("            _ => Err(UnitError::CPow(self,n))");
    println!("        }}");
    println!("    }}");

    /* Unit multiplication. */
    println!();
    println!("    pub fn mul_unwrapped(self, rhs: Unit) -> Result<Unit,UnitError> {{");
    println!("        match (self,rhs) {{");
    println!("            (NEUTRAL_UNIT,u) => Ok(u),");
    println!("            (u,NEUTRAL_UNIT) => Ok(u),");
    for (l, ld) in units.iter().filter(|(_, d)| !d.dims.is_empty()) {
        for (r, rd) in units.iter().filter(|(_, d)| !d.dims.is_empty()) {
            if let Some(o) = dims.get(&(ld.dims.clone() * rd.dims.clone())) {
                let (lp, lms) = ld.base.pat("l", &uc);
                let (rp, rms) = rd.base.pat("r", &uc);
                match (lms * rms).assemble_unwrapped(&units[o].base) {
                    Some((Some(c), t)) => {
                        println!(
                            "            (Unit::{}({}),Unit::{}({})) => match {} {{",
                            l, lp, r, rp, c
                        );
                        println!(
                            "                true => Ok(Unit::{}({})),",
                            o, t
                        );
                        println!("                false => Err(UnitError::CMul(self,rhs))");
                        println!("            }},");
                    }
                    Some((None, t)) => {
                        println!(
                            "            (Unit::{}({}),Unit::{}({})) => Ok(Unit::{}({})),",
                            l, lp, r, rp, o, t
                        );
                    }
                    _ => {}
                }
            }
        }
    }
    println!("            _ => Err(UnitError::CMul(self,rhs))");
    println!("        }}");
    println!("    }}");

    /* Unit division. */
    println!();
    println!("    pub fn div_unwrapped(self, rhs: Unit) -> Result<Unit,UnitError> {{");
    println!("        match (self,rhs) {{");
    println!("            (u,NEUTRAL_UNIT) => Ok(u),");
    for (l, ld) in units.iter().filter(|(_, d)| !d.dims.is_empty()) {
        for (r, rd) in units.iter().filter(|(_, d)| !d.dims.is_empty()) {
            if let Some(o) = dims.get(&(ld.dims.clone() / rd.dims.clone())) {
                let (lp, lms) = ld.base.pat("l", &uc);
                let (rp, rms) = rd.base.pat("r", &uc);
                match (lms / rms).assemble_unwrapped(&units[o].base) {
                    Some((Some(c), t)) => {
                        println!(
                            "            (Unit::{}({}),Unit::{}({})) => match {} {{",
                            l, lp, r, rp, c
                        );
                        println!(
                            "                true => Ok(Unit::{}({})),",
                            o, t
                        );
                        println!("                false => Err(UnitError::CDiv(self,rhs))");
                        println!("            }},");
                    }
                    Some((None, t)) => {
                        println!(
                            "            (Unit::{}({}),Unit::{}({})) => Ok(Unit::{}({})),",
                            l, lp, r, rp, o, t
                        );
                    }
                    _ => {}
                }
            }
        }
    }
    println!("            _ => Err(UnitError::CDiv(self,rhs))");
    println!("        }}");
    println!("    }}");
    println!("}}");
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
struct Unit {
    dims: Dims,
    base: Base,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
struct Dims(BTreeMap<Dimension, i64>);

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
struct Base(Vec<(&'static str, i64)>);

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
struct Match(BTreeMap<&'static str, Vec<(String, i64)>>);

impl Unit {
    fn from<T: IntoIterator<Item = (Dimension, i64)>>(
        dims: T,
        base: Vec<(&'static str, i64)>,
    ) -> Self {
        Unit {
            dims: Dims(dims.into_iter().collect()),
            base: Base(base),
        }
    }
}

impl Base {
    /*fn powi(self, p: i64) -> Self {
    Self(self.0.into_iter().map(|(u,n)| (u,n*p)).collect())
    }

    fn mult(&self, suff: &'static str, uc: &BTreeMap<&'static str,char>) -> String {
    self.0.iter().map(|(u,n)| format!("{}{}.multiplier().powi({})", uc[u], suff, n))
        .collect::<Vec<String>>().join("*")
    }*/

    fn pat(
        &self,
        suff: &'static str,
        uc: &BTreeMap<&'static str, char>,
    ) -> (String, Match) {
        (
            self.0
                .iter()
                .map(|(u, _)| format!("{}{}", uc[u], suff))
                .collect::<Vec<String>>()
                .join(","),
            Match(self.0.iter().fold(BTreeMap::new(), |mut m, (u, n)| {
                m.entry(u)
                    .or_default()
                    .push((format!("{}{}", uc[u], suff), *n));
                m
            })),
        )
    }

    fn refs(&self) -> String {
        self.0
            .iter()
            .map(|(u, _)| format!("{}::REFERENCE", u))
            .collect::<Vec<String>>()
            .join(",")
    }
}

impl Match {
    fn assemble(mut self, t: &Base) -> (String, String) {
        let mut mults = Vec::new();
        let base: Vec<String> =
            t.0.iter()
                .map(|(u, n)| match self.0.remove(u) {
                    Some(mut ms) => {
                        ms.sort_by_key(|(_, n)| n.abs());
                        let (c, m) = ms.pop().unwrap();
                        mults.extend(ms);
                        mults.push((c.clone(), m - n));
                        c
                    }
                    None => {
                        format!("{}::REFERENCE", u)
                    }
                })
                .collect();

        mults.extend(self.0.into_iter().flat_map(|(_, ms)| ms));

        let (nums, denoms): (Vec<_>, Vec<_>) = mults
            .into_iter()
            .filter(|(_, n)| *n != 0)
            .partition(|(_, n)| *n > 0);

        let num = match nums.is_empty() {
            true => "1.0".to_string(),
            false => prod(nums),
        };

        let mult = match denoms.len() {
            0 => num,
            1 => format!(
                "{} / {}",
                num,
                prod(denoms.into_iter().map(|(c, n)| (c, -n)))
            ),
            _ => format!(
                "{} / ({})",
                num,
                prod(denoms.into_iter().map(|(c, n)| (c, -n)))
            ),
        };

        (mult, base.join(","))
    }

    fn assemble_unwrapped(
        mut self,
        t: &Base,
    ) -> Option<(Option<String>, String)> {
        let mut conds = Vec::new();
        let mut base = Vec::new();

        for (u, n) in &t.0 {
            match self.0.remove(u) {
                Some(mut ms) => {
                    match *n == ms.iter().fold(0, |s, (_, n)| s + n) {
                        true => {
                            let (c, _) = ms.pop().unwrap();
                            conds.extend(
                                ms.into_iter()
                                    .map(|(d, _)| format!("{} == {}", c, d)),
                            );
                            base.push(c);
                        }
                        false => return None,
                    }
                }
                None => return None,
            }
        }

        for (_, mut ms) in self.0.into_iter() {
            match 0 == ms.iter().fold(0, |s, (_, n)| s + n) {
                true => {
                    let (c, _) = ms.pop().unwrap();
                    conds.extend(
                        ms.into_iter().map(|(d, _)| format!("{} == {}", c, d)),
                    );
                }
                false => return None,
            }
        }

        Some((
            match conds.is_empty() {
                true => None,
                false => Some(conds.join(" && ")),
            },
            base.join(","),
        ))
    }

    fn powi(self, p: i64) -> Self {
        Match(
            self.0
                .into_iter()
                .map(|(u, ms)| {
                    (u, ms.into_iter().map(|(c, n)| (c, n * p)).collect())
                })
                .collect(),
        )
    }
}

fn prod<T: IntoIterator<Item = (String, i64)>>(us: T) -> String {
    us.into_iter()
        .map(|(c, n)| match n {
            1 => format!("{}.multiplier()", c),
            _ => format!("{}.multiplier().powi({})", c, n),
        })
        .collect::<Vec<_>>()
        .join(" * ")
}

impl Mul<Match> for Match {
    type Output = Self;
    fn mul(mut self, rhs: Self) -> Self {
        rhs.0.into_iter().for_each(|(u, ms)| {
            self.0.entry(u).or_default().extend(ms);
        });
        self
    }
}

impl Div<Match> for Match {
    type Output = Self;
    fn div(mut self, rhs: Self) -> Self {
        rhs.0.into_iter().for_each(|(u, ms)| {
            self.0
                .entry(u)
                .or_default()
                .extend(ms.into_iter().map(|(c, n)| (c, -n)));
        });
        self
    }
}

impl Dims {
    fn powi(self, p: i64) -> Self {
        Self(self.0.into_iter().map(|(d, n)| (d, n * p)).collect())
    }
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Mul<Dims> for Dims {
    type Output = Self;
    fn mul(mut self, rhs: Self) -> Self {
        rhs.0.into_iter().for_each(|(d, n)| {
            *self.0.entry(d).or_insert(0) += n;
        });
        Self(self.0.into_iter().filter(|(_, n)| *n != 0).collect())
    }
}

impl Div<Dims> for Dims {
    type Output = Self;
    fn div(mut self, rhs: Self) -> Self {
        rhs.0.into_iter().for_each(|(d, n)| {
            *self.0.entry(d).or_insert(0) -= n;
        });
        Self(self.0.into_iter().filter(|(_, n)| *n != 0).collect())
    }
}
