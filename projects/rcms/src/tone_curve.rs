//! Gamma tone curves.

use std::cmp::Ordering;
use std::f64;
use std::ops::Range;

const EPSILON: f64 = 0.0001;

/// An ICC parametric curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IccParametricCurve {
    /// `y = x ^ g` (type 0)
    Gamma(f64),
    GammaInv(f64),

    /// `(ax + b) ^ g` (type 1)
    ///
    /// Parameters are g, a, b.
    LinGamma(f64, f64, f64),
    LinGammaInv(f64, f64, f64),

    /// (type 2)
    ///
    /// ```text
    /// y = (ax + b) ^ g + c | x <= -b / a
    /// y = c                | else
    /// ```
    ///
    /// Parameters are g, a, b, c.
    LinBGamma(f64, f64, f64, f64),
    LinBGammaInv(f64, f64, f64, f64),

    /// (type 3)
    ///
    /// ```text
    /// y = (ax + b) ^ g | x >= d
    /// y = cx           | else
    /// ```
    ///
    ///
    /// Parameters are g, a, b, c, d.
    LinLinGamma(f64, f64, f64, f64, f64),
    LinLinGammaInv(f64, f64, f64, f64, f64),

    /// (type 4)
    ///
    /// ```text
    /// y = (ax + b) ^ g + e | x >= d
    /// y = cx + f           | else
    /// ```
    ///
    /// Parameters are g, a, b, c, d, e, f.
    LinLinOffGamma(f64, f64, f64, f64, f64, f64, f64),
    LinLinOffGammaInv(f64, f64, f64, f64, f64, f64, f64),

    /// `y = (ax + b) ^ g + c` (type 5)
    ///
    /// Parameters are g, a, b, c.
    LinOffGamma(f64, f64, f64, f64),
    LinOffGammaInv(f64, f64, f64, f64),

    /// `y = a log_10(b * x ^ g + c) + d` (type 6)
    ///
    /// Parameters are g, a, b, c, d.
    LogGamma(f64, f64, f64, f64, f64),
    LogGammaInv(f64, f64, f64, f64, f64),

    /// `y = a * b ^ (cx + d) + e` (type 7)
    ///
    /// Parameters are g, a, b, c, d, e.
    LinPow(f64, f64, f64, f64, f64, f64),
    LinPowInv(f64, f64, f64, f64, f64, f64),

    /// `y = (1 - (1 - x) ^ (1 / g)) ^ (1 / g)` (type 107)
    Sigmoid(f64),
    SigmoidInv(f64),
}

impl IccParametricCurve {
    /// Creates a new parametric curve from the ICC type id and whether or not it should be
    /// inverted. The list of parameters should be appropriately sized, or this function will
    /// panic.
    ///
    /// Will return None if the type is unknown.
    pub fn from_type(p_type: u16, invert: bool, params: &[f64]) -> Option<Self> {
        match (p_type, invert) {
            (0, false) => Some(Self::Gamma(params[0])),
            (0, true) => Some(Self::GammaInv(params[0])),
            (1, false) => Some(Self::LinGamma(params[0], params[1], params[2])),
            (1, true) => Some(Self::LinGammaInv(params[0], params[1], params[2])),
            (2, false) => Some(Self::LinBGamma(params[0], params[1], params[2], params[3])),
            (2, true) => Some(Self::LinBGammaInv(
                params[0], params[1], params[2], params[3],
            )),
            (3, false) => Some(Self::LinLinGamma(
                params[0], params[1], params[2], params[3], params[4],
            )),
            (3, true) => Some(Self::LinLinGammaInv(
                params[0], params[1], params[2], params[3], params[4],
            )),
            (4, false) => Some(Self::LinLinOffGamma(
                params[0], params[1], params[2], params[3], params[4], params[5], params[6],
            )),
            (4, true) => Some(Self::LinLinOffGammaInv(
                params[0], params[1], params[2], params[3], params[4], params[5], params[6],
            )),
            (5, false) => Some(Self::LinOffGamma(
                params[0], params[1], params[2], params[3],
            )),
            (5, true) => Some(Self::LinOffGammaInv(
                params[0], params[1], params[2], params[3],
            )),
            (6, false) => Some(Self::LogGamma(
                params[0], params[1], params[2], params[3], params[4],
            )),
            (6, true) => Some(Self::LogGammaInv(
                params[0], params[1], params[2], params[3], params[4],
            )),
            (7, false) => Some(Self::LinPow(
                params[0], params[1], params[2], params[3], params[4], params[5],
            )),
            (7, true) => Some(Self::LinPowInv(
                params[0], params[1], params[2], params[3], params[4], params[5],
            )),
            (107, false) => Some(Self::Sigmoid(params[0])),
            (107, true) => Some(Self::SigmoidInv(params[0])),
            _ => None,
        }
    }

    /// Returns the icc type.
    pub fn icc_type(&self) -> u16 {
        use IccParametricCurve::*;
        match *self {
            Gamma(_) | GammaInv(_) => 0,
            LinGamma(..) | LinGammaInv(..) => 1,
            LinBGamma(..) | LinBGammaInv(..) => 2,
            LinLinGamma(..) | LinLinGammaInv(..) => 3,
            LinLinOffGamma(..) | LinLinOffGammaInv(..) => 4,
            LinOffGamma(..) | LinOffGammaInv(..) => 5,
            LogGamma(..) | LogGammaInv(..) => 6,
            LinPow(..) | LinPowInv(..) => 7,
            Sigmoid(..) | SigmoidInv(..) => 107,
        }
    }

    /// Returns whether this curve is the inverted version.
    pub fn is_inverted(&self) -> bool {
        use IccParametricCurve::*;
        match *self {
            GammaInv(_)
            | LinGammaInv(..)
            | LinBGammaInv(..)
            | LinLinGammaInv(..)
            | LinLinOffGammaInv(..)
            | LinOffGammaInv(..)
            | LogGammaInv(..)
            | LinPowInv(..)
            | SigmoidInv(..) => true,
            _ => false,
        }
    }

    /// Returns an iterator over all parameters, in order (g, a, b, c, ...).
    pub fn params(&self) -> impl Iterator<Item = f64> {
        enum Iter {
            S1(usize, [f64; 1]),
            S3(usize, [f64; 3]),
            S4(usize, [f64; 4]),
            S5(usize, [f64; 5]),
            S6(usize, [f64; 6]),
            S7(usize, [f64; 7]),
        }

        impl Iterator for Iter {
            type Item = f64;
            fn next(&mut self) -> Option<f64> {
                let (n, a): (&mut usize, &[f64]) = match self {
                    Iter::S1(n, a) => (n, a),
                    Iter::S3(n, a) => (n, a),
                    Iter::S4(n, a) => (n, a),
                    Iter::S5(n, a) => (n, a),
                    Iter::S6(n, a) => (n, a),
                    Iter::S7(n, a) => (n, a),
                };
                *n += 1;
                let index = *n - 1;
                a.get(index).map(|f| *f)
            }
        }

        use IccParametricCurve::*;
        match *self {
            Gamma(g) | GammaInv(g) => Iter::S1(0, [g]),
            LinGamma(g, a, b) | LinGammaInv(g, a, b) => Iter::S3(0, [g, a, b]),
            LinBGamma(g, a, b, c) | LinBGammaInv(g, a, b, c) => Iter::S4(0, [g, a, b, c]),
            LinLinGamma(g, a, b, c, d) | LinLinGammaInv(g, a, b, c, d) => {
                Iter::S5(0, [g, a, b, c, d])
            }
            LinLinOffGamma(g, a, b, c, d, e, f) | LinLinOffGammaInv(g, a, b, c, d, e, f) => {
                Iter::S7(0, [g, a, b, c, d, e, f])
            }
            LinOffGamma(g, a, b, c) | LinOffGammaInv(g, a, b, c) => Iter::S4(0, [g, a, b, c]),
            LogGamma(g, a, b, c, d) | LogGammaInv(g, a, b, c, d) => Iter::S5(0, [g, a, b, c, d]),
            LinPow(g, a, b, c, d, e) | LinPowInv(g, a, b, c, d, e) => {
                Iter::S6(0, [g, a, b, c, d, e])
            }
            Sigmoid(g) | SigmoidInv(g) => Iter::S1(0, [g]),
        }
    }

    /// Returns the inverted version of this curve.
    pub fn inverted(&self) -> Self {
        use IccParametricCurve::*;
        match *self {
            Gamma(g) => GammaInv(g),
            GammaInv(g) => Gamma(g),

            LinGamma(g, a, b) => LinGammaInv(g, a, b),
            LinGammaInv(g, a, b) => LinGamma(g, a, b),

            LinBGamma(g, a, b, c) => LinBGammaInv(g, a, b, c),
            LinBGammaInv(g, a, b, c) => LinBGamma(g, a, b, c),

            LinLinGamma(g, a, b, c, d) => LinLinGammaInv(g, a, b, c, d),
            LinLinGammaInv(g, a, b, c, d) => LinLinGamma(g, a, b, c, d),

            LinLinOffGamma(g, a, b, c, d, e, f) => LinLinOffGammaInv(g, a, b, c, d, e, f),
            LinLinOffGammaInv(g, a, b, c, d, e, f) => LinLinOffGamma(g, a, b, c, d, e, f),

            LinOffGamma(g, a, b, c) => LinOffGammaInv(g, a, b, c),
            LinOffGammaInv(g, a, b, c) => LinOffGamma(g, a, b, c),

            LogGamma(g, a, b, c, d) => LogGammaInv(g, a, b, c, d),
            LogGammaInv(g, a, b, c, d) => LogGamma(g, a, b, c, d),

            LinPow(g, a, b, c, d, e) => LinPowInv(g, a, b, c, d, e),
            LinPowInv(g, a, b, c, d, e) => LinPow(g, a, b, c, d, e),

            Sigmoid(g) => SigmoidInv(g),
            SigmoidInv(g) => Sigmoid(g),
        }
    }

    /// Evaluates the parametric curve at the given position.
    ///
    /// Will return None if the curve is undefined at that point.
    pub fn eval(&self, x: f64) -> Option<f64> {
        match self {
            // y = x ^ g
            IccParametricCurve::Gamma(g) => {
                if x < 0. {
                    if (g - 1.).abs() < EPSILON {
                        Some(x)
                    } else {
                        None
                    }
                } else {
                    Some(x.powf(*g))
                }
            }
            // x = y ^ (1 / g)
            IccParametricCurve::GammaInv(g) => {
                if x < 0. {
                    if (g - 1.).abs() < EPSILON {
                        Some(x)
                    } else {
                        None
                    }
                } else {
                    if g.abs() < EPSILON {
                        None
                    } else {
                        Some(x.powf(1. / g))
                    }
                }
            }
            // CIE 122-1966
            // y = (ax + b) ^ g | x >= -b / a
            // y = 0            | else
            IccParametricCurve::LinGamma(g, a, b) => {
                if a.abs() < EPSILON {
                    None
                } else if x < -b / a {
                    Some(0.)
                } else {
                    Some((a * x + b).powf(*g))
                }
            }
            // x = (y ^ (1 / g) - b) / a
            IccParametricCurve::LinGammaInv(g, a, b) => {
                if g.abs() < EPSILON || a.abs() < EPSILON {
                    None
                } else if x < 0. {
                    Some(0.)
                } else {
                    Some(((x.powf(1. / g) - b) / a).max(0.))
                }
            }
            // IEC 61966-3
            // y = (ax + b) ^ g + c | x <= -b / a
            // y = c                | else
            IccParametricCurve::LinBGamma(g, a, b, c) => {
                if a.abs() < EPSILON {
                    None
                } else if x < (-b / a).max(0.) {
                    Some(*c)
                } else {
                    let x = a * x + b;
                    if x < 0. {
                        Some(*c)
                    } else {
                        Some(x.powf(*g) + c)
                    }
                }
            }
            // x = ((y - c) ^ (1 / g) - b) / a | y >= c
            // x = -b / a                      | else
            IccParametricCurve::LinBGammaInv(g, a, b, c) => {
                if g.abs() < EPSILON || a.abs() < EPSILON {
                    None
                } else if x < *c {
                    Some(-b / a)
                } else {
                    Some(((x - c).powf(1. / g) - b) / a)
                }
            }
            // IEC 61966-2.1 (sRGB)
            // y = (ax + b) ^ g | x >= d
            // y = cx           | else
            IccParametricCurve::LinLinGamma(g, a, b, c, d) => {
                if x < *d {
                    Some(c * x)
                } else {
                    Some((a * x + b).powf(*g))
                }
            }
            // x = (y ^ (1 / g) - b) / a | y >= (ad + b) ^ g
            // x = y / c                 | else
            IccParametricCurve::LinLinGammaInv(g, a, b, c, d) => {
                if a.abs() < EPSILON || b.abs() < EPSILON || d.abs() < EPSILON {
                    None
                } else if x < (a * d + b).max(0.).powf(*g) {
                    Some(x / c)
                } else {
                    Some((x.powf(1. / g) - b) / a)
                }
            }
            // y = (ax + b) ^ g + e | x >= d
            // y = cx + f           | else
            IccParametricCurve::LinLinOffGamma(g, a, b, c, d, e, f) => {
                if x >= *d {
                    Some((a * x + b).max(0.).powf(*g) + e)
                } else {
                    Some(c * x + f)
                }
            },
            // x = ((y - e) ^ (1 / g) - b) / a | y >= (ad + b) ^ g + e, cd + f
            // x = (y - f) / c                 | else
            IccParametricCurve::LinLinOffGammaInv(g, a, b, c, d, e, f) => {
                if g.abs() < EPSILON || a.abs() < EPSILON || c.abs() < EPSILON {
                    None
                } else if x < c * d + e {
                    Some((x - f) / c)
                } else {
                    Some(((x - e).powf(1. / g) - b) / a)
                }
            },

            // y = (ax + b) ^ g + c
            IccParametricCurve::LinOffGamma(g, a, b, c) => {
                Some((a * x + b).max(0.).powf(*g) + c)
            }
            // x = ((y - c) ^ (1 / g) - b) / a
            IccParametricCurve::LinOffGammaInv(g, a, b, c) => {
                if g.abs() < EPSILON || a.abs() < EPSILON {
                    None
                } else {
                    Some(((x - c).powf(1. / g) - b) / a)
                }
            }

            // y = a log_10(b * x ^ g + c) + d
            IccParametricCurve::LogGamma(g, a, b, c, d) => {
                Some(a * ((b * x).powf(*g) + c).log10() + d)
            }
            //                                (y - d) / a = log_10(b * x ^ g + c)
            //                       pow(10, (y - d) / a) = b * x ^ g + c
            // pow((pow(10, (y - d) / a) - c) / b, 1 / g) = x
            IccParametricCurve::LogGammaInv(g, a, b, c, d) => {
                if g.abs() < EPSILON || a.abs() < EPSILON || b.abs() < EPSILON {
                    None
                } else {
                    Some(((10_f64.powf((x - d) / a) - c) / b).powf(1. / g))
                }
            }

            // FIXME: does this curve really not use the gamma parameter?
            // y = a * b ^ (cx + d) + e
            IccParametricCurve::LinPow(_, a, b, c, d, e) => {
                Some((a * b).powf(c * x + d) + e)
            }
            // x = (log_b((y - e) / a) - d) / c
            IccParametricCurve::LinPowInv(_, a, b, c, d, e) => {
                if a.abs() < EPSILON || (b - 1.).abs() < EPSILON || c.abs() < EPSILON {
                    None
                } else {
                    Some((((x - e) / a - d).log(*b) - d) / c)
                }
            }

            // y = (1 - (1 - x) ^ (1 / g)) ^ (1 / g)
            IccParametricCurve::Sigmoid(g) => {
                if g.abs() < EPSILON {
                    None
                } else {
                    Some((1. - (1. - x).powf(1. / g)).powf(1. / g))
                }
            }
            //                   y = (1 - (1 - x) ^ (1 / g)) ^ (1 / g)
            //               y ^ g = (1 - (1 - x) ^ (1 / g))
            //           1 - y ^ g = (1 - x) ^ (1 / g)
            //     (1 - y ^ g) ^ g = 1 - x
            // 1 - (1 - y ^ g) ^ g = x
            IccParametricCurve::SigmoidInv(g) => {
                Some(1. - (1. - x.powf(*g)).powf(*g))
            }
        }
    }

    /// Composes two parametric curves, if possible.
    pub fn compose_with(&self, other: &IccParametricCurve) -> Option<IccParametricCurve> {
        use IccParametricCurve::*;
        if self.is_identity() {
            return Some(*other);
        } else if other.is_identity() {
            return Some(*self);
        }
        match (*self, *other) {
            (Gamma(a), Gamma(b)) => Some(Gamma(a * b)),
            (Gamma(a), GammaInv(b)) if b != 0. => Some(Gamma(a / b)),
            (GammaInv(a), Gamma(b)) if a != 0. => Some(Gamma(b / a)),
            (GammaInv(a), GammaInv(b)) => Some(GammaInv(a * b)),
            // I don't think you can really compose any of the other functions in the general case
            _ => None,
        }
    }

    /// Returns true if this curve is actually the identity.
    pub fn is_identity(&self) -> bool {
        use IccParametricCurve::*;
        match *self {
            Gamma(g) | GammaInv(g) => g == 1.,
            LinGamma(g, a, b) | LinGammaInv(g, a, b) => g == 1. && a == 1. && b == 0.,
            LinBGamma(g, a, b, c) | LinBGammaInv(g, a, b, c) => {
                g == 1. && a == 1. && b == 0. && c == 0.
            }
            LinLinGamma(g, a, b, c, _) | LinLinGammaInv(g, a, b, c, _) => {
                g == 1. && a == 1. && b == 0. && c == 1.
            }
            LinLinOffGamma(g, a, b, c, _, e, f) | LinLinOffGammaInv(g, a, b, c, _, e, f) => {
                g == 1. && a == 1. && b == 0. && c == 1. && e == 0. && f == 0.
            }
            LinOffGamma(g, a, b, c) | LinOffGammaInv(g, a, b, c) => {
                g == 1. && a == 1. && b == 0. && c == 0.
            }
            LogGamma(..) | LogGammaInv(..) => false,
            LinPow(..) | LinPowInv(..) => false,
            Sigmoid(..) | SigmoidInv(..) => false,
        }
    }
}

/// Tone curve segment types.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum CurveType {
    Const(f64),
    IccParam(IccParametricCurve),
    Table(Vec<u16>),
    Sampled(Vec<f64>),
    // TODO: custom curves?
}

/// A tone curve segment.
#[derive(Debug, Clone, PartialEq)]
pub struct CurveSegment {
    /// The domain of this segment.
    ///
    /// Bounds may be infinite.
    pub domain: Range<f64>,
    /// The curve in this segments.
    pub curve: CurveType,
}

impl CurveSegment {
    /// Evaluates this curve segment at the given position.
    ///
    /// Expect potentially nonsensical data or None if x is outside the domain.
    pub fn eval(&self, x: f64) -> Option<f64> {
        match self.curve {
            CurveType::Const(a) => Some(a),
            CurveType::IccParam(curve) => curve.eval(x),
            CurveType::Table(ref table) => {
                let table_len = table.len();
                let lower = ((x * table_len as f64).floor() as usize)
                    .max(0)
                    .min(table_len - 1);
                let upper = ((x * table_len as f64).ceil() as usize)
                    .max(0)
                    .min(table_len - 1);
                let a = table[lower] as f64;
                let b = table[upper] as f64;
                let mut p = (x * table_len as f64 - lower as f64) / (upper as f64 - lower as f64);
                if !p.is_finite() {
                    p = 0.;
                }
                Some((a + ((b - a) * p)) / 65535.)
            }
            CurveType::Sampled(ref samples) => {
                let x = (x - self.domain.start) / (self.domain.end - self.domain.start);
                let sample_count = samples.len();
                let lower = ((x * sample_count as f64).floor() as usize)
                    .max(0)
                    .min(sample_count - 1);
                let upper = ((x * sample_count as f64).ceil() as usize)
                    .max(0)
                    .min(sample_count - 1);
                let lower_sample = samples[lower];
                let upper_sample = samples[upper];

                let mut p = x as f64 - lower as f64 / (upper as f64 - lower as f64);
                if !p.is_finite() {
                    p = 0.;
                }

                Some(lower_sample + (upper_sample - lower_sample) * p)
            }
        }
    }

    /// Returns true if this segment is the identity function in its domain.
    pub fn is_identity(&self) -> bool {
        match self.curve {
            CurveType::Const(_) => false,
            CurveType::IccParam(curve) => curve.is_identity(),
            CurveType::Table(ref table) => {
                for (i, y) in table.iter().enumerate() {
                    let x = i as f64 / table.len() as f64;
                    let expected_y = (self.domain.end - self.domain.start) * x + self.domain.start;
                    let expected_y = (expected_y * 65535.) as u16;
                    if *y != expected_y {
                        return false;
                    }
                }
                true
            }
            CurveType::Sampled(ref samples) => {
                for (i, y) in samples.iter().enumerate() {
                    let x = i as f64 / samples.len() as f64;
                    let expected_y = (self.domain.end - self.domain.start) * x + self.domain.start;
                    if (y - expected_y).abs() > 1e-3 {
                        return false;
                    }
                }
                true
            }
        }
    }
}

/// A gamma tone curve.
#[derive(Debug, Clone, PartialEq)]
pub struct ToneCurve {
    /// The segments of this tone curve.
    ///
    /// Must be sorted by domain in ascending order, otherwise strange things will happen.
    pub segments: Vec<CurveSegment>,
}

impl ToneCurve {
    /// Creates a new tone curve.
    ///
    /// Segments must be sorted by domain in ascending order (currently not checked).
    pub fn new(segments: Vec<CurveSegment>) -> Self {
        ToneCurve { segments }
    }

    /// Creates a new tone curve from a table of values between 0..1.
    ///
    /// 0 maps to 0 and 65535 maps to 1. Evaluating the curve outside 0..1 will return the closest
    /// value.
    pub fn new_table(values: Vec<u16>) -> Self {
        ToneCurve::new(vec![CurveSegment {
            domain: -f64::INFINITY..f64::INFINITY,
            curve: CurveType::Table(values),
        }])
    }

    /// Creates a new tone curve from a table of values between 0..1.
    ///
    /// Evaluating this curve outside 0..1 will return either the first or the last value in the
    /// table.
    pub fn new_tabulated(values: Vec<f64>) -> Self {
        let first_value = *values.first().unwrap_or(&0.);
        let last_value = *values.last().unwrap_or(&0.);

        ToneCurve::new(vec![
            CurveSegment {
                domain: -f64::INFINITY..0.,
                curve: CurveType::Const(first_value),
            },
            CurveSegment {
                domain: 0.0..1.0,
                curve: CurveType::Sampled(values),
            },
            CurveSegment {
                domain: 1.0..f64::INFINITY,
                curve: CurveType::Const(last_value),
            },
        ])
    }

    /// Creates a new parametric tone curve.
    /// Will return None if the given parametric curve type is unknown.
    pub fn new_icc_parametric(p_type: u16, params: &[f64]) -> Option<Self> {
        IccParametricCurve::from_type(p_type, false, params).map(|curve| {
            ToneCurve::new(vec![CurveSegment {
                domain: -f64::INFINITY..f64::INFINITY,
                curve: CurveType::IccParam(curve),
            }])
        })
    }

    /// Creates a new gamma curve.
    pub fn new_gamma(gamma: f64) -> Self {
        Self::new_icc_parametric(0, &[gamma]).unwrap()
    }

    /// Tries to evaluate the tone curve at the given value. Returns None if undefined.
    pub fn eval(&self, x: f64) -> Option<f64> {
        self.segments
            .binary_search_by(|segment| {
                if segment.domain.start >= x {
                    Ordering::Greater
                } else if segment.domain.end < x {
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            })
            .ok()
            .map_or(None, |index| self.segments[index].eval(x))
    }

    /// Creates an inverted version of this tone curve.
    ///
    /// Attempts an analytical inversion and falls back to samples.
    pub fn inverted(&self) -> Self {
        let mut sample_count = 4096;

        for segment in &self.segments {
            if let CurveType::Sampled(samples) = &segment.curve {
                sample_count = samples.len().max(sample_count);
            }
        }

        self.inverted_with_samples(sample_count)
    }

    /// Attempts an analytical inversion and falls back to inverting with samples.
    ///
    /// # Panics
    /// - if the number of samples is less than 2
    pub fn inverted_with_samples(&self, samples: usize) -> Self {
        if samples < 2 {
            panic!("inversion of tone curve with less than 2 samples");
        }

        // try analytical inversion
        if self.segments.len() == 1 {
            let segment = &self.segments[0];
            if let CurveType::IccParam(p) = segment.curve {
                return ToneCurve::new(vec![CurveSegment {
                    domain: -f64::INFINITY..f64::INFINITY,
                    curve: CurveType::IccParam(p.inverted()),
                }]);
            }
        }

        // perform very simple inversion using binary search

        let mut table = Vec::with_capacity(samples);

        let is_ascending = self.eval(0.0001).unwrap_or(0.) < self.eval(0.9999).unwrap_or(1.);

        for i in 0..samples {
            let y = i as f64 / (samples - 1) as f64;

            let mut lower = 0f64;
            let mut upper = 1f64;
            let threshold = 1. / samples as f64;

            while (upper - lower).abs() > threshold {
                let mid = (upper - lower) / 2. + lower;
                match self.eval(mid) {
                    Some(value) => {
                        if (is_ascending && value < y) || (!is_ascending && value > y) {
                            lower = mid;
                        } else {
                            upper = mid;
                        }
                    }
                    None => {
                        // shrug.jpg
                        upper = mid;
                    }
                }
            }

            let mid = (upper - lower) / 2. + lower;
            table.push(mid);
        }

        ToneCurve::new_tabulated(table)
    }

    /// Composes this tone curve with another and returns an approximation.
    pub fn compose_with_approx(&self, other: &ToneCurve) -> ToneCurve {
        if self.segments.len() == 1 && other.segments.len() == 1 {
            // try analytical composition
            let own_segment = &self.segments[0];
            let other_segment = &other.segments[0];

            let domain_start = own_segment.domain.start.max(other_segment.domain.start);
            let domain_end = own_segment.domain.end.min(other_segment.domain.end);

            match (&own_segment.curve, &other_segment.curve) {
                (CurveType::IccParam(own_curve), CurveType::IccParam(other_curve)) => {
                    if let Some(composed) = own_curve.compose_with(other_curve) {
                        return ToneCurve::new(vec![CurveSegment {
                            domain: domain_start..domain_end,
                            curve: CurveType::IccParam(composed),
                        }]);
                    }
                }
                (CurveType::Const(a), _) => {
                    return ToneCurve::new(vec![CurveSegment {
                        domain: domain_start..domain_end,
                        curve: CurveType::Const(*a),
                    }]);
                }
                _ => (),
            }
        }

        let mut segments = Vec::with_capacity(3);

        // sample -1..2 with -1..0 and 1..2 at lower resolution
        let sample_rate_oob = 1024;
        let mut samples = Vec::new();
        for i in 0..sample_rate_oob {
            let x = i as f64 / sample_rate_oob as f64 - 1.;
            samples.push(
                self.eval(other.eval(x).unwrap_or(f64::NAN))
                    .unwrap_or(f64::NAN),
            );
        }
        segments.push(CurveSegment {
            domain: -1.0..0.0,
            curve: CurveType::Sampled(samples),
        });

        let sample_rate = 4096; // TODO: dynamic sample rate
        let mut samples = Vec::new();
        for i in 0..sample_rate {
            let x = i as f64 / sample_rate as f64;
            samples.push(
                self.eval(other.eval(x).unwrap_or(f64::NAN))
                    .unwrap_or(f64::NAN),
            );
        }
        segments.push(CurveSegment {
            domain: 0.0..1.0,
            curve: CurveType::Sampled(samples),
        });

        let mut samples = Vec::new();
        for i in 0..sample_rate_oob {
            let x = i as f64 / sample_rate_oob as f64 + 1.;
            samples.push(
                self.eval(other.eval(x).unwrap_or(f64::NAN))
                    .unwrap_or(f64::NAN),
            );
        }
        segments.push(CurveSegment {
            domain: 1.0..2.0,
            curve: CurveType::Sampled(samples),
        });

        ToneCurve::new(segments)
    }

    /// Returns true if this is an identity curve where defined.
    pub fn is_identity(&self) -> bool {
        self.segments.iter().find(|x| !x.is_identity()).is_none()
    }
}
