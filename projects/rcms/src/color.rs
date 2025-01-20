//! Color representations.

use crate::util::{lcms_mat3_eval, lcms_mat3_per};
use cgmath::prelude::*;
use cgmath::{Matrix3, Vector3};

/// A CIE XYZ color.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Cxyz {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// A CIE xyY color.
#[repr(C)]
#[allow(non_snake_case)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CxyY {
    pub x: f64,
    pub y: f64,
    pub Y: f64,
}

/// A CIE L\*a\*b\* color.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CLab {
    pub l: f64,
    pub a: f64,
    pub b: f64,
}

/// The D50 white point.
pub const D50: Cxyz = Cxyz {
    x: 0.9642,
    y: 1.,
    z: 0.8249,
};

/// The D65 white point.
pub const D65: Cxyz = Cxyz {
    x: 0.95047,
    y: 1.,
    z: 1.08883,
};

impl From<Cxyz> for CxyY {
    fn from(this: Cxyz) -> CxyY {
        let sum = this.x + this.y + this.z;
        CxyY {
            x: this.x / sum,
            y: this.y / sum,
            Y: this.y,
        }
    }
}

impl From<CxyY> for Cxyz {
    fn from(this: CxyY) -> Cxyz {
        Cxyz {
            x: this.x / this.y * this.Y,
            y: this.Y,
            z: (1. - this.x - this.y) / this.y * this.Y,
        }
    }
}

// this is safe because repr(C) means these structs will be laid out identically to [f64; 3]
macro_rules! impl_as_slice {
    ($($i:ident),+) => {
        $(
        impl $i {
            /// Returns a reference to this color as a slice of floats.
            pub fn as_slice(&self) -> &[f64; 3] {
                unsafe { &*(self as *const Self as *const [f64; 3]) }
            }
            /// Returns a reference to this color as a mutable slice of floats.
            pub fn as_slice_mut(&mut self) -> &mut [f64; 3] {
                unsafe { &mut *(self as *mut Self as *mut [f64; 3]) }
            }
        }
        )+
    }
}
impl_as_slice!(Cxyz, CxyY, CLab);

fn xyz2lab_f(t: f64) -> f64 {
    let limit = (24. / 116.) * (24. / 116.) * (24. / 116.);

    if t <= limit {
        (841. / 108.) * t + (16. / 116.)
    } else {
        t.cbrt()
    }
}

fn xyz2lab_f_inv(t: f64) -> f64 {
    let limit = 24. / 116.;

    if t <= limit {
        (108. / 841.) * (t - (16. / 116.))
    } else {
        t * t * t
    }
}

impl Cxyz {
    pub const MAX_ENCODABLE: f64 = 1.0 + 32767.0 / 32768.0;

    /// Converts this color to L\*a\*b\* with the given white point.
    ///
    /// Can handle negative values “in some cases.”
    pub fn into_lab(self, white_point: Cxyz) -> CLab {
        let fx = xyz2lab_f(self.x / white_point.x);
        let fy = xyz2lab_f(self.y / white_point.y);
        let fz = xyz2lab_f(self.z / white_point.z);

        CLab {
            l: 116. * fy - 16.,
            a: 500. * (fx - fy),
            b: 200. * (fy - fz),
        }
    }

    /// Returns the final chromatic adaptation from this illuminant to the given illuminant.
    ///
    /// If no cone matrix is specified, the Bradford matrix will be used.
    pub fn adaptation_matrix(
        &self,
        to_illuminant: Cxyz,
        cone_matrix: Option<Matrix3<f64>>,
    ) -> Option<Matrix3<f64>> {
        /// Bradford matrix
        const BRADFORD: Matrix3<f64> = Matrix3 {
            x: Vector3 {
                x: 0.8951,
                y: 0.2664,
                z: -0.1614,
            },
            y: Vector3 {
                x: -0.7502,
                y: 1.7135,
                z: 0.0367,
            },
            z: Vector3 {
                x: 0.0389,
                y: -0.0685,
                z: 1.0296,
            },
        };

        compute_chromatic_adaptation(cone_matrix.unwrap_or(BRADFORD), *self, to_illuminant)
    }

    /// Adapts this color to a given illuminant.
    ///
    /// Returns None if there is no adaptation matrix from the source to the illuminant.
    pub fn adapt_to_illuminant(&self, source_white_point: Cxyz, illuminant: Cxyz) -> Option<Cxyz> {
        match source_white_point.adaptation_matrix(illuminant, None) {
            Some(bradford) => {
                let vec = lcms_mat3_eval(bradford, (*self).into());
                Some(Cxyz {
                    x: vec.x,
                    y: vec.y,
                    z: vec.z,
                })
            }
            None => return None,
        }
    }
}

impl CLab {
    /// Converts this color to XYZ with the given white point.
    /// May return negative values.
    pub fn into_xyz(self, white_point: Cxyz) -> Cxyz {
        let y = (self.l + 16.) / 116.;
        let x = y + 0.002 * self.a;
        let z = y - 0.005 * self.b;

        Cxyz {
            x: xyz2lab_f_inv(x) * white_point.x,
            y: xyz2lab_f_inv(y) * white_point.y,
            z: xyz2lab_f_inv(z) * white_point.z,
        }
    }
}

impl CxyY {
    /// Creates a white point from the given temperature.
    ///
    /// Will return None if the temperature is out of bounds.
    pub fn from_temp(kelvin: f64) -> Option<CxyY> {
        let t = kelvin;
        let t2 = t * t;
        let t3 = t2 * t;

        let x = if t >= 4000. && t <= 7000. {
            // for correlated color temperature (T) between 4000K and 7000K:
            -4.6070 * (1e9 / t3) + 2.9678 * (1e6 / t2) + 0.09911 * (1e3 / t) + 0.244063
        } else if t > 7000. && t <= 25000. {
            // or for correlated color temperature (T) between 7000K and 25000K:
            -2.0064 * (1e9 / t3) + 1.9018 * (1e6 / t2) + 0.24748 * (1e3 / t) + 0.237040
        } else {
            return None;
        };

        let y = -3. * (x * x) + 2.87 * x + 0.275;

        // wave factors (not used, but here for futures extensions)

        // M1 = (-1.3515 - 1.7703 * x + 5.9114 * y) / (0.0241 + 0.2562 * x - 0.7341 *
        // y); M2 = (0.03 - 31.4424 * x + 30.0717 * y) / (0.0241 + 0.2562 * x -
        // 0.7341 * y);

        Some(CxyY { x, y, Y: 1. })
    }

    /// Uses Robertson's method to attempt to obtain a temperature from a given white point.
    pub fn to_temp(&self) -> Option<f64> {
        #[derive(Debug, Clone, Copy)]
        struct ISOTemperature(f64, f64, f64, f64);

        impl ISOTemperature {
            /// Temperature in microreciprocal kelvin.
            fn mirek(&self) -> f64 {
                self.0
            }
            /// U coordinate of intersection with blackbody locus.
            fn ut(&self) -> f64 {
                self.1
            }
            /// V coordinate of intersection with blackbody locus.
            fn vt(&self) -> f64 {
                self.2
            }
            /// Slope of temperature line.
            fn tt(&self) -> f64 {
                self.3
            }
        }

        const ISO_TEMP_DATA: [ISOTemperature; 31] = [
            ISOTemperature(0., 0.18006, 0.26352, -0.24341),
            ISOTemperature(10., 0.18066, 0.26589, -0.25479),
            ISOTemperature(20., 0.18133, 0.26846, -0.26876),
            ISOTemperature(30., 0.18208, 0.27119, -0.28539),
            ISOTemperature(40., 0.18293, 0.27407, -0.30470),
            ISOTemperature(50., 0.18388, 0.27709, -0.32675),
            ISOTemperature(60., 0.18494, 0.28021, -0.35156),
            ISOTemperature(70., 0.18611, 0.28342, -0.37915),
            ISOTemperature(80., 0.18740, 0.28668, -0.40955),
            ISOTemperature(90., 0.18880, 0.28997, -0.44278),
            ISOTemperature(100., 0.19032, 0.29326, -0.47888),
            ISOTemperature(125., 0.19462, 0.30141, -0.58204),
            ISOTemperature(150., 0.19962, 0.30921, -0.70471),
            ISOTemperature(175., 0.20525, 0.31647, -0.84901),
            ISOTemperature(200., 0.21142, 0.32312, -1.0182),
            ISOTemperature(225., 0.21807, 0.32909, -1.2168),
            ISOTemperature(250., 0.22511, 0.33439, -1.4512),
            ISOTemperature(275., 0.23247, 0.33904, -1.7298),
            ISOTemperature(300., 0.24010, 0.34308, -2.0637),
            ISOTemperature(325., 0.24702, 0.34655, -2.4681),
            ISOTemperature(350., 0.25591, 0.34951, -2.9641),
            ISOTemperature(375., 0.26400, 0.35200, -3.5814),
            ISOTemperature(400., 0.27218, 0.35407, -4.3633),
            ISOTemperature(425., 0.28039, 0.35577, -5.3762),
            ISOTemperature(450., 0.28863, 0.35714, -6.7262),
            ISOTemperature(475., 0.29685, 0.35823, -8.5955),
            ISOTemperature(500., 0.30505, 0.35907, -11.324),
            ISOTemperature(525., 0.31320, 0.35968, -15.628),
            ISOTemperature(550., 0.32129, 0.36011, -23.325),
            ISOTemperature(575., 0.32931, 0.36038, -40.770),
            ISOTemperature(600., 0.33724, 0.36051, -116.45),
        ];

        let mut di = 0.;
        let mut mi = 0.;
        let xs = self.x;
        let ys = self.y;

        // convert (x, y) to CIE 1960 (u, white_point)
        let us = (2. * xs) / (-xs + 6. * ys + 1.5);
        let vs = (3. * ys) / (-xs + 6. * ys + 1.5);

        for j in 0..ISO_TEMP_DATA.len() {
            let uj = ISO_TEMP_DATA[j].ut();
            let vj = ISO_TEMP_DATA[j].vt();
            let tj = ISO_TEMP_DATA[j].tt();
            let mj = ISO_TEMP_DATA[j].mirek();

            let dj = ((vs - vj) - tj * (us - uj)) / (1. + tj * tj).sqrt();

            if j != 0 && di / dj < 0. {
                // found a match
                return Some(1_000_000. / (mi + (di / (di - dj)) * (mj - mi)));
            }

            di = dj;
            mi = mj;
        }

        // not found
        None
    }
}

impl From<Vector3<f64>> for Cxyz {
    fn from(this: Vector3<f64>) -> Self {
        Self {
            x: this.x,
            y: this.y,
            z: this.z,
        }
    }
}

impl From<Cxyz> for Vector3<f64> {
    fn from(this: Cxyz) -> Vector3<f64> {
        Vector3 {
            x: this.x,
            y: this.y,
            z: this.z,
        }
    }
}

/// Computes a chromatic adaptation matrix using chad as the cone matrix
fn compute_chromatic_adaptation(
    chad: Matrix3<f64>,
    source_wp: Cxyz,
    dest_wp: Cxyz,
) -> Option<Matrix3<f64>> {
    let inverse = match chad.invert() {
        Some(inverse) => inverse,
        None => return None,
    };

    let cone_source_xyz: Vector3<f64> = source_wp.into();
    let cone_dest_xyz: Vector3<f64> = dest_wp.into();

    let cone_source_rgb = lcms_mat3_eval(chad, cone_source_xyz);
    let cone_dest_rgb = lcms_mat3_eval(chad, cone_dest_xyz);

    // build matrix
    let cone = Matrix3::from_diagonal(Vector3::new(
        cone_dest_rgb.x / cone_source_rgb.x,
        cone_dest_rgb.y / cone_source_rgb.y,
        cone_dest_rgb.z / cone_source_rgb.z,
    ));

    // normalize
    Some(lcms_mat3_per(inverse, lcms_mat3_per(cone, chad)))
}

/// Given a white point and primaries, builds a transfer matrix from RGB to CIE XYZ.
///
/// This is just an approximation and does not handle all the non-linear aspects of the RGB to XYZ
/// process, and assumes that the gamma correction is transitive in the transformation chain.
///
/// The algorithm:
///
/// - First, the absolute conversion matrix is built using primaries in XYZ. This matrix is next
///   inverted
/// - The source white point is evaluated across this matrix, obtaining the coefficients of the
///   transformation
/// - Then, these coefficients are applied to the original matrix
pub fn build_rgb_to_xyz_transfer_matrix(
    white_point: CxyY,
    primaries: (CxyY, CxyY, CxyY),
) -> Option<Matrix3<f64>> {
    let xn = white_point.x;
    let yn = white_point.y;
    let xr = primaries.0.x;
    let yr = primaries.0.y;
    let xg = primaries.1.x;
    let yg = primaries.1.y;
    let xb = primaries.2.x;
    let yb = primaries.2.y;

    let primaries = Matrix3::from_cols(
        (xr, xg, xb).into(),
        (yr, yg, yb).into(),
        (1. - xr - yr, 1. - xg - yg, 1. - xb - yb).into(),
    );

    let result = match primaries.invert() {
        Some(result) => result,
        None => return None,
    };

    let white_point = Vector3::new(xn / yn, 1., (1. - xn - yn) / yn);

    let coef = lcms_mat3_eval(result, white_point);

    let mat = Matrix3::from_cols(
        (coef.x * xr, coef.y * xg, coef.z * xb).into(),
        (coef.x * yr, coef.y * yg, coef.z * yb).into(),
        (
            coef.x * (1. - xr - yr),
            coef.y * (1. - xg - yg),
            coef.z * (1. - xb - yb),
        )
            .into(),
    );

    Cxyz::from(white_point)
        .adaptation_matrix(D50, None)
        .map(|bradford| lcms_mat3_per(bradford, mat))
}

#[test]
fn xyz_to_xyy_sanity_check() {
    let a: CxyY = D50.into();
    let b: Cxyz = a.into();

    assert!((D50.x - b.x).abs() < 1e-5);
    assert!((D50.y - b.y).abs() < 1e-5);
    assert!((D50.z - b.z).abs() < 1e-5);
}
