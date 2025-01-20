//! Profile linking.

use crate::black_point::{detect_black_point, detect_dest_black_point};
use crate::color::{CxyY, Cxyz, D50};
use crate::pipeline::{Pipeline, PipelineError, PipelineStage};
use crate::profile::{ColorSpace, IccProfile, Intent, ProfileClass};
use crate::util::{lcms_mat3_eval, lcms_mat3_per};
use cgmath::prelude::*;
use cgmath::{Matrix3, Vector3};
use std::error::Error;
use std::fmt;

/// Linking errors.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum LinkError {
    /// A pipeline error.
    Pipeline(PipelineError),
    /// Incompatible color spaces at a given index.
    IncompatibleSpaces(usize, ColorSpace, ColorSpace),
    /// Failure computing absolute intent at a given index.
    AbsoluteIntentError(usize),
    /// Missing device link LUT at a given index.
    NoDeviceLinkLut(usize),
    /// Missing input LUT at a given index.
    NoInputLut(usize),
    /// Missing output LUT at a given index.
    NoOutputLut(usize),
}

impl From<PipelineError> for LinkError {
    fn from(this: PipelineError) -> LinkError {
        LinkError::Pipeline(this)
    }
}

impl fmt::Display for LinkError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LinkError::Pipeline(err) => write!(f, "{}", err),
            LinkError::IncompatibleSpaces(i, a, b) => write!(
                f,
                "incompatible color spaces at index {}: converting from {} to {}",
                i, a, b
            ),
            LinkError::AbsoluteIntentError(i) => {
                write!(f, "absolute intent error for profile at index {}", i)
            }
            LinkError::NoDeviceLinkLut(i) => {
                write!(f, "missing device link LUT for profile at index {}", i)
            }
            LinkError::NoInputLut(i) => write!(f, "missing input LUT for profile at index {}", i),
            LinkError::NoOutputLut(i) => write!(f, "missing output LUT for profile at index {}", i),
        }
    }
}

impl Error for LinkError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            LinkError::Pipeline(err) => Some(err),
            _ => None,
        }
    }
}

impl ColorSpace {
    /// Returns true if this color space is compatible with the other for linking.
    ///
    /// Commutative.
    pub fn is_compatible_with(self, other: ColorSpace) -> bool {
        if self == other {
            // they're the same
            true
        } else if self == ColorSpace::S4Color && other == ColorSpace::CMYK
            || self == ColorSpace::CMYK && other == ColorSpace::S4Color
        {
            // mch4 substitution of cmyk
            true
        } else if self == ColorSpace::XYZ && other == ColorSpace::Lab
            || self == ColorSpace::Lab && other == ColorSpace::XYZ
        {
            // xyz/lab
            true
        } else {
            false
        }
    }
}

/// Links profiles.
///
/// # Parameters
/// - `profiles`: a list of ICC profiles. This will most commonly be just two profiles (an input and
///   an output profile) but more can be used in-between to link more finicky profiles.
/// - `intents`: rendering intents for each profile.
/// - `black_point_compensation`: set to true to use black point compensation for the conversion to
///   the profile at the given index. Will be overridden automatically in certain cases.
/// - `adaptation_states`: for absolute colorimetric intents: a float value from 0 to 1 indicating
///   how adapted the observer is.
///
/// # Panics
/// - if parameters are not all the same length
pub fn link(
    profiles: &[&IccProfile],
    intents: &[Intent],
    black_point_compensation: &[bool],
    adaptation_states: &[f64],
) -> Result<Pipeline, LinkError> {
    let mut pipeline = Pipeline::new();

    let mut current_cs = profiles[0].color_space;

    for i in 0..profiles.len() {
        let profile = &profiles[i];
        let intent = intents[i];

        let mut bpc = black_point_compensation[i];

        // Check if black point is really needed or allowed. Note that
        // following Adobe's document:
        // BPC does not apply to devicelink profiles, nor to abs colorimetric,
        // and applies always on V4 perceptual and saturation.
        if intent == Intent::AbsoluteColorimetric {
            bpc = false;
        }

        if intent == Intent::Perceptual || intent == Intent::Saturation {
            if profile.version() >= (4, 0) {
                bpc = true;
            }
        }

        let is_device_link = profile.device_class == ProfileClass::Link.into()
            || profile.device_class == ProfileClass::Abstract.into();

        let use_as_input = if i == 0 && !is_device_link {
            // First profile is used as input unless devicelink or abstract
            true
        } else {
            // Else use profile in the input direction if current space is not PCS
            current_cs != ColorSpace::XYZ && current_cs != ColorSpace::Lab
        };

        let (cs_in, cs_out) = if use_as_input || is_device_link {
            (profile.color_space, profile.pcs)
        } else {
            (profile.pcs, profile.color_space)
        };

        if !cs_in.is_compatible_with(current_cs) {
            return Err(LinkError::IncompatibleSpaces(i, cs_in, current_cs));
        }

        if is_device_link
            || (profile.device_class == ProfileClass::NamedColor && profiles.len() == 1)
        {
            let mut profile_lut = match profile.device_link_lut(intent) {
                Some(lut) => lut,
                None => return Err(LinkError::NoDeviceLinkLut(i)),
            };

            let (m, off) = if profile.device_class == ProfileClass::Abstract && i > 0 {
                compute_conversion(i as u32, profiles, intent, bpc, adaptation_states[i])?
            } else {
                (Matrix3::identity(), Vector3::zero())
            };

            add_conversion(&mut pipeline, i, current_cs, cs_in, m, off)?;
            pipeline.append(&mut profile_lut)?;
        } else if use_as_input {
            let mut profile_lut = match profile.input_lut(intent) {
                Some(lut) => lut,
                None => return Err(LinkError::NoInputLut(i)),
            };
            pipeline.append(&mut profile_lut)?;
        } else {
            let mut profile_lut = match profile.output_lut(intent) {
                Some(lut) => lut,
                None => return Err(LinkError::NoOutputLut(i)),
            };

            let (m, off) =
                compute_conversion(i as u32, profiles, intent, bpc, adaptation_states[i])?;
            add_conversion(&mut pipeline, i, current_cs, cs_in, m, off)?;
            pipeline.append(&mut profile_lut)?;
        }

        current_cs = cs_out;
    }

    Ok(pipeline)
}

/// Computes the conversion layer
fn compute_conversion(
    i: u32,
    profiles: &[&IccProfile],
    intent: Intent,
    bpc: bool,
    adaptation_state: f64,
) -> Result<(Matrix3<f64>, Vector3<f64>), LinkError> {
    let i = i as usize;
    if intent == Intent::AbsoluteColorimetric {
        let prev_profile = &profiles[i - 1];
        let profile = &profiles[i];

        let wp_in = prev_profile.media_white_point();
        let cam_in = prev_profile.adaptation_matrix();
        let wp_out = profile.media_white_point();
        let cam_out = profile.adaptation_matrix();

        Ok((
            compute_absolute_intent(adaptation_state, wp_in, cam_in, wp_out, cam_out, i)?,
            Vector3::zero(),
        ))
    } else if bpc {
        let bp_in = detect_black_point(&profiles[i - 1], intent);
        let bp_out = detect_dest_black_point(&profiles[i], intent);

        // If black points are equal, then do nothing
        if bp_in != bp_out {
            let (m, off) = compute_black_point_compensation(bp_in, bp_out);
            // Offset should be adjusted because the encoding. We encode XYZ normalized to
            // 0..1.0.
            // To do that, we divide by MAX_ENCODEABLE_XYZ. The conversion stage goes XYZ -> XYZ
            // so we have first to convert from encoded to XYZ and then convert back to encoded.
            // y = Mx + Off
            // x = x'c
            // y = M x'c + Off
            // y = y'c; y' = y / c
            // y' = (Mx'c + Off) /c = Mx' + (Off / c)
            Ok((m, off / Cxyz::MAX_ENCODABLE))
        } else {
            Ok((Matrix3::identity(), Vector3::zero()))
        }
    } else {
        Ok((Matrix3::identity(), Vector3::zero()))
    }
}

/// Returns if m should be applied
fn is_empty_layer(m: Matrix3<f64>, off: Vector3<f64>) -> bool {
    let ident = Matrix3::<f64>::identity();
    let mut diff = 0.;

    for i in 0..9 {
        diff += (m[i / 3][i % 3] - ident[i / 3][i % 3]).abs();
    }

    for i in 0..3 {
        diff += off[i].abs();
    }

    diff < 0.002
}

fn add_conversion(
    result: &mut Pipeline,
    i: usize,
    in_pcs: ColorSpace,
    out_pcs: ColorSpace,
    mat: Matrix3<f64>,
    off: Vector3<f64>,
) -> Result<(), LinkError> {
    // Handle PCS mismatches. A specialized stage is added to the LUT in such case
    match (in_pcs, out_pcs) {
        (ColorSpace::XYZ, ColorSpace::XYZ) => {
            if !is_empty_layer(mat, off) {
                result.append_stage(PipelineStage::new_matrix3(mat, Some(off)))?;
            }
        }
        (ColorSpace::XYZ, ColorSpace::Lab) => {
            if !is_empty_layer(mat, off) {
                result.append_stage(PipelineStage::new_matrix3(mat, Some(off)))?;
            }
            result.append_stage(PipelineStage::new_xyz_to_lab())?;
        }
        (ColorSpace::Lab, ColorSpace::XYZ) => {
            result.append_stage(PipelineStage::new_lab_to_xyz())?;
            if !is_empty_layer(mat, off) {
                result.append_stage(PipelineStage::new_matrix3(mat, Some(off)))?;
            }
        }
        (ColorSpace::Lab, ColorSpace::Lab) => {
            if !is_empty_layer(mat, off) {
                result.append_stage(PipelineStage::new_lab_to_xyz())?;
                result.append_stage(PipelineStage::new_matrix3(mat, Some(off)))?;
                result.append_stage(PipelineStage::new_xyz_to_lab())?;
            }
        }
        _ => {
            // On colorspaces other than PCS, check for same space
            if in_pcs != out_pcs {
                return Err(LinkError::IncompatibleSpaces(i, in_pcs, out_pcs));
            }
        }
    }

    Ok(())
}

/// Black point compensation. Implemented as a linear scaling in XYZ. Black points
/// should come relative to the white point. Fills a matrix/offset element m
/// which is organized as a 4x4 matrix.
fn compute_black_point_compensation(
    black_point_in: Cxyz,
    black_point_out: Cxyz,
) -> (Matrix3<f64>, Vector3<f64>) {
    // Now we need to compute a matrix plus an offset m such that
    // [m]*bpin + off = bpout
    // [m]*D50  + off = D50
    //
    // This is a linear scaling in the form ax+b, where
    // a = (bpout - D50) / (bpin - D50)
    // b = - D50* (bpout - bpin) / (bpin - D50)
    let tx = black_point_in.x - D50.x;
    let ty = black_point_in.y - D50.y;
    let tz = black_point_in.z - D50.z;

    let ax = (black_point_out.x - D50.x) / tx;
    let ay = (black_point_out.y - D50.y) / ty;
    let az = (black_point_out.z - D50.z) / tz;

    let bx = -D50.x * (black_point_out.x - black_point_in.x) / tx;
    let by = -D50.y * (black_point_out.y - black_point_in.y) / ty;
    let bz = -D50.z * (black_point_out.z - black_point_in.z) / tz;

    (
        Matrix3::from_diagonal((ax, ay, az).into()),
        Vector3::new(bx, by, bz),
    )
}

/// Join scalings to obtain relative input to absolute and then to relative output.
/// Result is stored in a 3x3 matrix
fn compute_absolute_intent(
    adaptation_state: f64,
    wp_in: Cxyz,
    cam_in: Matrix3<f64>,
    wp_out: Cxyz,
    cam_out: Matrix3<f64>,
    i: usize,
) -> Result<Matrix3<f64>, LinkError> {
    if adaptation_state == 1. {
        // Observer is fully adapted. Keep chromatic adaptation.
        // That is the standard V4 behaviour
        Ok(Matrix3::from_diagonal(
            (wp_in.x / wp_out.x, wp_in.y / wp_out.y, wp_in.z / wp_out.z).into(),
        ))
    } else {
        // Incomplete adaptation. This is an advanced feature.
        let scale = Matrix3::from_diagonal(
            (wp_in.x / wp_out.x, wp_in.y / wp_out.y, wp_in.z / wp_out.z).into(),
        );

        if adaptation_state == 0. {
            let m2 = lcms_mat3_per(cam_out, scale);
            // m2 holds CHAD from output white to D50 times abs. col. scaling

            // Observer is not adapted, undo the chromatic adaptation
            // let m = mat3_per(m2, cam_out);

            let cam_in_inv = match cam_in.invert() {
                Some(m) => m,
                None => return Err(LinkError::AbsoluteIntentError(i)),
            };

            Ok(lcms_mat3_per(m2, cam_in_inv))
        } else {
            let m2 = match cam_in.invert() {
                Some(m) => m,
                None => return Err(LinkError::AbsoluteIntentError(i)),
            };

            let m3 = lcms_mat3_per(m2, scale);

            // m3 holds CHAD from input white to D50 times abs. col. scaling
            let temp_src = match chad_to_temp(cam_in) {
                Some(v) => v,
                None => return Err(LinkError::AbsoluteIntentError(i)),
            };
            let temp_dest = match chad_to_temp(cam_out) {
                Some(v) => v,
                None => return Err(LinkError::AbsoluteIntentError(i)),
            };

            if temp_src < 0. || temp_dest < 0. {
                // something went wrong
                return Err(LinkError::AbsoluteIntentError(i));
            }

            if scale.is_identity() && (temp_src - temp_dest).abs() < 0.01 {
                return Ok(Matrix3::identity());
            }

            let temp = (1. - adaptation_state) * temp_dest + adaptation_state * temp_src;

            let mixed_chad = match temp_to_chad(temp) {
                Some(m) => m,
                None => return Err(LinkError::AbsoluteIntentError(i)),
            };

            Ok(lcms_mat3_per(m3, mixed_chad))
        }
    }
}

/// Approximate a blackbody illuminant based on CHAD information
fn chad_to_temp(chad: Matrix3<f64>) -> Option<f64> {
    let inverse = match chad.invert() {
        Some(inverse) => inverse,
        None => return None,
    };

    let d50_xyz = D50;
    let s = Vector3::new(d50_xyz.x, d50_xyz.y, d50_xyz.z);
    let d = lcms_mat3_eval(inverse, s);

    let dest: CxyY = Cxyz {
        x: d.x,
        y: d.y,
        z: d.z,
    }
    .into();

    dest.to_temp()
}

/// Compute a CHAD based on a given temperature
fn temp_to_chad(temp: f64) -> Option<Matrix3<f64>> {
    let chromaticity_of_white: Cxyz = match CxyY::from_temp(temp) {
        Some(c) => c.into(),
        None => return None,
    };
    chromaticity_of_white.adaptation_matrix(D50, None)
}
