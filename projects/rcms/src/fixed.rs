//! Fixed-point types.

use std::convert::TryFrom;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReprError<T>(pub T);

impl<T> fmt::Display for ReprError<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "could not represent {} in the desired data type", self.0)
    }
}

impl<T> Error for ReprError<T> where T: fmt::Debug + fmt::Display {}

/// Signed 15.16 fixed-point number.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct s15f16(i32);

impl s15f16 {
    pub const ZERO: s15f16 = s15f16(0);

    pub fn from_bytes(this: i32) -> Self {
        s15f16(this)
    }
    pub fn to_bytes(self) -> i32 {
        self.0
    }
}

impl fmt::Display for s15f16 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let sign = self.0.signum();
        let value = self.0.abs();
        let whole = (value >> 16) & 0x7FFF;
        let fract = value & 0xFFFF;

        write!(f, "{}.", sign * whole)?;

        let mut fract_float = fract as f64 / 65536.;
        fract_float *= 10.;
        while fract_float > 0. {
            write!(f, "{}", fract_float.floor() as u8)?;
            fract_float = 10. * fract_float.fract();
        }
        Ok(())
    }
}

impl fmt::Debug for s15f16 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl From<s15f16> for f64 {
    fn from(this: s15f16) -> f64 {
        let sign = this.0.signum() as f64;
        let value = this.0.abs();

        let whole = (value >> 16) & 0x7FFF;
        let fract = value & 0xFFFF;

        sign * (whole as f64) + (fract as f64 / 65536.)
    }
}

impl TryFrom<f64> for s15f16 {
    type Error = ReprError<f64>;

    fn try_from(this: f64) -> Result<s15f16, ReprError<f64>> {
        let sign = this.signum() as i32;
        let value = this.abs();

        let whole = {
            let value = value.floor();
            if value > 0x7FFF as f64 {
                return Err(ReprError(this));
            }
            value as i32
        };
        let frac_part = value.fract();

        Ok(s15f16(
            sign * (((whole & 0x7FFF) << 16) | (((frac_part * 65536.) as i32) & 0xFFFF)),
        ))
    }
}

/// Unsigned 16.16 fixed-point number.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct u16f16(u32);

impl u16f16 {
    pub const ZERO: u16f16 = u16f16(0);

    pub fn from_bytes(this: u32) -> Self {
        u16f16(this)
    }
    pub fn to_bytes(self) -> u32 {
        self.0
    }
}

impl fmt::Display for u16f16 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let value = self.0;
        let whole = (value >> 16) & 0xFFFF;
        let fract = value & 0xFFFF;

        write!(f, "{}.", whole)?;

        let mut fract_float = fract as f64 / 65536.;
        fract_float *= 10.;
        while fract_float > 0. {
            write!(f, "{}", fract_float.floor() as u8)?;
            fract_float = 10. * fract_float.fract();
        }
        Ok(())
    }
}

impl fmt::Debug for u16f16 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl From<u16f16> for f64 {
    fn from(this: u16f16) -> f64 {
        this.0 as f64 / 65536.
    }
}

impl TryFrom<f64> for u16f16 {
    type Error = ReprError<f64>;
    fn try_from(this: f64) -> Result<u16f16, ReprError<f64>> {
        if this < 0. || this > 0xFFFF as f64 {
            return Err(ReprError(this));
        }
        Ok(u16f16((this * 65536.) as u32))
    }
}
