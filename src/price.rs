use std::cmp;
use std::fmt;
use std::ops;
use std::fmt::Display;
use std::num::ParseFloatError;

const WHOLE: i64 = 100000000;
const CENT: i64 = WHOLE / 100; // always displayed up to this precision
const FRACTIONAL_DIGITS: usize = 6; // these are only displayed when used
const FROM_WHOLE: f64 = 1.0 / WHOLE as f64;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Price {
    val: i64,
}

impl PartialOrd for Price {
    fn partial_cmp(&self, rhs: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(rhs))
    }
}

impl Ord for Price {
    fn cmp(&self, rhs: &Self) -> cmp::Ordering {
        self.val.cmp(&rhs.val)
    }
}

impl Price {
    pub fn zero() -> Price {
        Price { val: 0 }
    }

    pub fn parse(s: &str) -> Result<Price, ParseFloatError> {
        /*
        let mut val: i64;
        let mut iter = s.split('.');
        match iter.next() {
            Some(whole) => val = WHOLE * parse_price_int(whole)?,
            None => return Err(ParseError::FailedToParse),
        }
        match iter.next() {
            Some(whole) => val = WHOLE * parse_price_int(whole)?,
            None => return Err(ParseError::FailedToParse),
        }
        Ok(Price{ val })
        */
        Ok(Self::from(s.parse::<f64>()?))
    }
}

impl From<f64> for Price {
    fn from(px: f64) -> Self {
        Price {
            val: (px * WHOLE as f64) as i64,
        }
    }
}

impl From<Price> for f64 {
    fn from(px: Price) -> Self {
        px.val as f64 * FROM_WHOLE
    }
}

impl ops::Add for Price {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Price {
            val: self.val + rhs.val,
        }
    }
}

impl ops::AddAssign for Price {
    fn add_assign(&mut self, rhs: Self) {
        self.val += rhs.val;
    }
}

impl ops::Sub for Price {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Price {
            val: self.val - rhs.val,
        }
    }
}

impl ops::SubAssign for Price {
    fn sub_assign(&mut self, rhs: Self) {
        self.val -= rhs.val;
    }
}

// Work, but maybe too permissive: impl<T> ops::Mul<T> for Price where T: ops::Mul<i64, Output=i64> {
impl ops::Mul<i64> for Price {
    type Output = Self;
    fn mul(self, rhs: i64) -> Self {
        Price {
            val: rhs * self.val,
        }
    }
}

impl ops::MulAssign<i64> for Price {
    fn mul_assign(&mut self, rhs: i64) {
        self.val *= rhs;
    }
}

// Doesn't work: impl<T> ops::Mul<Price> for T where T: ops::Mul<i64, Output=i64> {
impl ops::Mul<Price> for i64 {
    type Output = Price;
    fn mul(self, rhs: Price) -> Price {
        Price {
            val: self * rhs.val,
        }
    }
}

// Works, but maybe too permissive: impl<T> ops::Div<T> for Price where T: ops::Div<i64, Output=i64> {
impl ops::Div<i64> for Price {
    type Output = Self;
    fn div(self, rhs: i64) -> Self {
        Price {
            val: self.val / rhs,
        }
    }
}

impl ops::DivAssign<i64> for Price {
    fn div_assign(&mut self, rhs: i64) {
        self.val *= rhs;
    }
}

impl Display for Price {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let abs = self.val.abs();
        let whole = abs / WHOLE;
        let part = abs % WHOLE;
        let cents = part / CENT;

        if self.val < 0 {
            write!(f, "-")?;
        }
        write!(f, "{}.{:02}", whole, cents)?;

        let mut fractional = part % CENT;
        let mut min_width = FRACTIONAL_DIGITS;
        while fractional != 0 && fractional % 10 == 0 {
            fractional /= 10;
            min_width -= 1;
        }
        if fractional != 0 {
            write!(f, "{:01$}", fractional, min_width)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn price_display() {
        assert_eq!("1024.65", format!("{}", Price::from(1024.65)));
        assert_eq!("1024.00", format!("{}", Price::from(1024.)));
        assert_eq!("15024.015", format!("{}", Price::from(15024.015)));
        assert_eq!("1024.0151", format!("{}", Price::from(1024.0151)));
        assert_eq!("1024.01512", format!("{}", Price::from(1024.01512)));
        assert_eq!("1024.010001", format!("{}", Price::from(1024.010001)));
        assert_eq!("0.10", format!("{}", Price::from(0.10)));
        assert_eq!("0.00", format!("{}", Price::from(0.0)));

        assert_eq!("-1024.65", format!("{}", Price::from(-1024.65)));
        assert_eq!("-1024.00", format!("{}", Price::from(-1024.)));
        assert_eq!("-15024.015", format!("{}", Price::from(-15024.015)));
        assert_eq!("-1024.0151", format!("{}", Price::from(-1024.0151)));
        assert_eq!("-1024.01512", format!("{}", Price::from(-1024.01512)));
        assert_eq!("-1024.010001", format!("{}", Price::from(-1024.010001)));
        assert_eq!("-0.10", format!("{}", Price::from(-0.10)));
    }

    #[test]
    fn math_ops() {
        assert_eq!(Price::from(110.), Price::from(11.) * 10);
        assert_eq!(Price::from(110.), 10 * Price::from(11.));
        assert_eq!(Price::from(11.), Price::from(110.) / 10);
        assert_eq!(Price::from(25.), Price::from(10.) + Price::from(15.));
    }
}
