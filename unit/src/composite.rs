/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::convert::From;
use std::collections::BTreeMap;
use std::iter::{once,IntoIterator};
use std::fmt::{self,Display,Formatter};
use std::ops::Mul;


/// Composite unit / quantity.
#[derive(PartialEq,Eq,Hash,Clone,Debug)]
pub struct Composite<T:Ord>(BTreeMap<T,i8>);


impl<T:Ord> Composite<T> {

    pub fn simple(val: T) -> Self {
	Composite::from(once((val,1)))
    }

    pub fn from_map(map: BTreeMap<T,i8>) -> Self {
	Composite(map)
    }

    pub fn as_map(&self) -> &BTreeMap<T,i8> {
	&self.0
    }

    pub fn iter(&self) -> impl Iterator<Item = (&T,&i8)> {
	self.as_map().iter()
    }

    pub fn into_iter(self) -> impl Iterator<Item = (T,i8)> {
	self.0.into_iter()
    }

}

impl<S,T:Ord> From<S> for Composite<T>
where S: IntoIterator<Item = (T,i8)> {
    fn from(vals: S) -> Self {
	Composite(vals.into_iter().collect())
    }
}


impl<T:Ord> Mul<Composite<T>> for Composite<T> {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
	Composite(other.into_iter().fold(self.0, |mut m,(u,n)| {
	    *m.entry(u).or_insert(0) += n; m }))
    }
}


impl<T:Display+Ord> Display for Composite<T> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
	write!(f, "{}", self.0.iter().filter(|(_,&p)| p > 0).map(
	    |(u,p)| format!("{}{}", u, superscript(*p)))
	       .collect::<Vec<String>>().join("\u{22c5}"))?;
	if self.0.iter().any(|(_,&p)| p < 0) {
	    write!(f, "/{}", self.0.iter().filter(|(_,&p)| p < 0).map(
		|(u,p)| format!("{}{}", u, superscript(-*p)))
		   .collect::<Vec<String>>().join("\u{22c5}"))?;
	}
	Ok(())
    }
}


static SS : [char;10] = ['\u{2070}','\u{00b9}','\u{00b2}','\u{00b3}','\u{2074}',
			 '\u{2075}','\u{2076}','\u{2077}','\u{2078}','\u{2079}'];

fn superscript(val: i8) -> String {
    if val != 1 {
	val.to_string().chars().map(|c| match c.to_digit(10) {
	    Some(n) => SS[n as usize], None => c }).collect()
    } else {
	"".to_string()
    }
}
