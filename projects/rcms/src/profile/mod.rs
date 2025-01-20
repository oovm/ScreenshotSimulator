//! Color profiles.

use crate::color::{CxyY, Cxyz, D50};
use crate::fixed::{s15f16, u16f16};
use crate::pipeline::{Pipeline, PipelineStage};
use crate::tone_curve::ToneCurve;
use cgmath::prelude::*;
use cgmath::{Matrix3, Vector3};
pub use io::{DataDeserError, DeserError, SerError};
use mlu::Mlu;
use std::collections::HashMap;
use std::fmt;
pub use types::*;

mod constructors;
mod io;
pub mod mlu;
mod types;

/// Data stored in an ICC tag.
#[derive(Debug, Clone, PartialEq)]
pub enum IccTagData {
    /// This tag points to another tag.
    Linked(u32),
    /// This tag contains a value.
    Value(IccValue),
    /// This tag contains an unknown value.
    Raw(Vec<u8>),
}

type RawTag = u32;

/// An 8-bit LUT.
#[derive(Debug, Clone, PartialEq)]
pub struct Lut8 {
    pub input_channels: u8,
    pub output_channels: u8,
    pub clut_grid_points: u8,
    pub matrix: Matrix3<f64>,
    pub input_table: Vec<u8>,
    pub clut: Vec<u8>,
    pub output_table: Vec<u8>,
}

impl Lut8 {
    pub fn as_pipeline(&self) -> Pipeline {
        todo!()
    }
}

/// A 16-bit LUT.
#[derive(Debug, Clone, PartialEq)]
pub struct Lut16 {
    pub input_channels: u8,
    pub output_channels: u8,
    pub clut_grid_points: u8,
    pub matrix: Matrix3<f64>,
    pub input_table: Vec<u16>,
    pub clut: Vec<u16>,
    pub output_table: Vec<u16>,
}

impl Lut16 {
    pub fn as_pipeline(&self) -> Pipeline {
        todo!()
    }
}

/// A color lookup table.
#[derive(Debug, Clone, PartialEq)]
pub enum CLut {
    Lut8(Vec<u8>),
    Lut16(Vec<u16>),
}

/// An A to B LUT.
#[derive(Debug, Clone, PartialEq)]
pub struct LutAToB {
    pub clut: CLut,
    pub matrix: Matrix3<f64>,
    pub offset: Vector3<f64>,
    pub a: Vec<ToneCurve>,
    pub b: Vec<ToneCurve>,
    pub m: Vec<ToneCurve>,
}

impl LutAToB {
    pub fn as_pipeline(&self) -> Pipeline {
        todo!()
    }
}

/// A B to A LUT.
#[derive(Debug, Clone, PartialEq)]
pub struct LutBToA {
    pub clut: CLut,
    pub matrix: Matrix3<f64>,
    pub offset: Vector3<f64>,
    pub a: Vec<ToneCurve>,
    pub b: Vec<ToneCurve>,
    pub m: Vec<ToneCurve>,
}

impl LutBToA {
    pub fn as_pipeline(&self) -> Pipeline {
        todo!()
    }
}

/// Data values found in ICC profiles.
#[derive(Debug, Clone, PartialEq)]
pub enum IccValue {
    /// XYZType
    Cxyz(Cxyz),
    /// chromaticityType
    Chromaticity(CxyY, CxyY, CxyY),
    /// colorantOrderType
    ColorantOrder(Vec<u8>),
    /// s15Fixed16ArrayType
    S15Fixed16Array(Vec<s15f16>),
    /// u16Fixed16ArrayType
    U16Fixed16Array(Vec<u16f16>),
    /// signatureType
    Signature(u32),
    /// textType
    Text(String),
    /// multiLocalizedUnicodeType
    Mlu(Mlu),
    /// curveType
    Curve(ToneCurve),
    /// lut8Type
    Lut8(Lut8),
    /// lut16Type
    Lut16(Lut16),
    /// lutAToBType
    LutAToB(LutAToB),
    /// lutBToAType
    LutBToA(LutBToA),
    /// multiProcessElementsType
    Pipeline(Pipeline),
    /// dataType
    Data { flags: u32, data: Vec<u8> },
}

impl IccValue {
    /// Tries to turn this value into a pipeline.
    ///
    /// Works for all LUT types.
    pub fn as_pipeline(&self) -> Option<Pipeline> {
        match self {
            IccValue::Lut8(lut) => Some(lut.as_pipeline()),
            IccValue::Lut16(lut) => Some(lut.as_pipeline()),
            IccValue::LutAToB(lut) => Some(lut.as_pipeline()),
            IccValue::LutBToA(lut) => Some(lut.as_pipeline()),
            IccValue::Pipeline(pipe) => Some(pipe.clone()),
            _ => None,
        }
    }
}

/// An ICC color profile.
#[derive(Clone, PartialEq)]
pub struct IccProfile {
    pub created: time::OffsetDateTime,
    pub version: u32,
    pub device_class: ProfileClass,
    pub color_space: ColorSpace,
    pub pcs: ColorSpace,
    pub rendering_intent: Intent,
    pub preferred_cmm: u32,
    pub platform: u32,
    pub flags: u32,
    pub manufacturer: u32,
    pub model: u32,
    pub attributes: u64,
    pub creator: u32,
    pub id: u128,
    pub tags: HashMap<RawTag, IccTagData>,
}

// LUT tags
const DEVICE_TO_PCS_16: [IccTag; 4] = [
    IccTag::AToB0, // Perceptual
    IccTag::AToB1, // Relative colorimetric
    IccTag::AToB2, // Saturation
    IccTag::AToB1, // Absolute colorimetric
];

const DEVICE_TO_PCS_FLOAT: [IccTag; 4] = [
    IccTag::DToB0, // Perceptual
    IccTag::DToB1, // Relative colorimetric
    IccTag::DToB2, // Saturation
    IccTag::DToB3, // Absolute colorimetric
];

const PCS_TO_DEVICE_16: [IccTag; 4] = [
    IccTag::BToA0, // Perceptual
    IccTag::BToA1, // Relative colorimetric
    IccTag::BToA2, // Saturation
    IccTag::BToA1, // Absolute colorimetric
];

const PCS_TO_DEVICE_FLOAT: [IccTag; 4] = [
    IccTag::BToD0, // Perceptual
    IccTag::BToD1, // Relative colorimetric
    IccTag::BToD2, // Saturation
    IccTag::BToD3, // Absolute colorimetric
];

impl IccProfile {
    /// Creates a new empty profile.
    pub fn new(device_class: ProfileClass, color_space: ColorSpace) -> Self {
        IccProfile {
            created: time::OffsetDateTime::now(),
            version: 0x02100000, // default version
            device_class,
            color_space,
            pcs: ColorSpace::XYZ,
            rendering_intent: Intent::Perceptual,
            preferred_cmm: 0,
            platform: 0,
            manufacturer: 0,
            model: 0,
            flags: 0,
            id: 0,
            attributes: 0,
            creator: 0,
            tags: HashMap::new(),
        }
    }

    /// Returns the major and minor version.
    pub fn version(&self) -> (u32, u32) {
        fn hex2dec_digits(n: u32) -> u32 {
            format!("{:x}", n).parse().unwrap_or(0)
        }

        let major = hex2dec_digits(self.version >> 24);
        let minor = hex2dec_digits(self.version >> 20);

        (major, minor)
    }

    /// Returns the major and minor version.
    pub fn set_version(&mut self, major: u32, minor: u32) {
        fn dec2hex_digits(n: u32) -> u32 {
            u32::from_str_radix(&format!("{}", n), 16).unwrap_or(0)
        }

        self.version = (dec2hex_digits(major) << 24) | (dec2hex_digits(minor) << 20);
    }

    fn get_tag_recursively(&self, key: RawTag, depth: usize) -> Option<&IccValue> {
        if depth > 30 {
            return None;
        }

        self.tags.get(&key).map_or(None, |data| match data {
            IccTagData::Linked(key) => self.get_tag_recursively(*key, depth + 1),
            IccTagData::Value(value) => Some(value),
            IccTagData::Raw(_) => None,
        })
    }

    /// Returns the value of the given tag if it’s available and parsed. Follows linked tags.
    pub fn get_tag(&self, key: IccTag) -> Option<&IccValue> {
        self.get_tag_recursively(key.into(), 0)
    }

    /// Inserts a tag.
    pub fn insert_tag(&mut self, key: IccTag, value: IccValue) {
        self.tags.insert(key.into(), IccTagData::Value(value));
    }

    /// Inserts a linked tag.
    pub fn link_tag(&mut self, key: IccTag, link_to: IccTag) {
        self.tags
            .insert(key.into(), IccTagData::Linked(link_to.into()));
    }
}

impl IccProfile {
    /// Returns a media white point.
    ///
    /// Apparently fixes some issues found in certain old profiles.
    pub fn media_white_point(&self) -> Cxyz {
        match self.get_tag(IccTag::MediaWhitePoint) {
            Some(_)
                if self.version() < (4, 0) && self.device_class == ProfileClass::Display.into() =>
            {
                D50
            }
            Some(IccValue::Cxyz(wp)) => *wp,
            _ => D50,
        }
    }

    /// Returns the chromatic adaptation matrix.
    pub fn adaptation_matrix(&self) -> Matrix3<f64> {
        match self.get_tag(IccTag::ChromaticAdaptation) {
            Some(IccValue::S15Fixed16Array(values)) if values.len() == 9 => {
                return Matrix3::from([
                    [values[0].into(), values[3].into(), values[6].into()],
                    [values[1].into(), values[4].into(), values[7].into()],
                    [values[2].into(), values[5].into(), values[8].into()],
                ])
            }
            _ => (),
        }

        if self.version() < (4, 0) && self.device_class == ProfileClass::Display.into() {
            match self.get_tag(IccTag::MediaWhitePoint) {
                Some(IccValue::Cxyz(wp)) => match wp.adaptation_matrix(D50, None) {
                    Some(mat) => mat,
                    None => Matrix3::identity(),
                },
                _ => Matrix3::identity(),
            }
        } else {
            Matrix3::identity()
        }
    }

    fn device_link_float_tag(&self, tag: IccTag) -> Option<Pipeline> {
        let mut pipeline = self.get_tag(tag).map_or(None, IccValue::as_pipeline)?;

        let pcs = self.pcs;
        let spc = self.color_space;

        if spc == ColorSpace::Lab {
            if let Err(_) = pipeline.prepend_stage(PipelineStage::new_normalize_to_lab_float()) {
                return None;
            }
        } else if spc == ColorSpace::XYZ {
            if let Err(_) = pipeline.prepend_stage(PipelineStage::new_normalize_to_xyz_float()) {
                return None;
            }
        }

        if pcs == ColorSpace::Lab {
            if let Err(_) = pipeline.append_stage(PipelineStage::new_normalize_from_lab_float()) {
                return None;
            }
        } else if pcs == ColorSpace::XYZ {
            if let Err(_) = pipeline.append_stage(PipelineStage::new_normalize_from_xyz_float()) {
                return None;
            }
        }

        Some(pipeline)
    }

    pub fn device_link_lut(&self, intent: Intent) -> Option<Pipeline> {
        if self.device_class == ProfileClass::NamedColor {
            todo!("named color lists")
        }

        // float tag takes precedence
        let tag_float = DEVICE_TO_PCS_FLOAT[intent as usize];
        if let Some(pipeline) = self.device_link_float_tag(tag_float) {
            return Some(pipeline);
        }

        let tag_float = DEVICE_TO_PCS_FLOAT[0];
        if let Some(pipeline) = self.get_tag(tag_float).map_or(None, IccValue::as_pipeline) {
            return Some(pipeline);
        }

        let mut tag_16 = DEVICE_TO_PCS_16[intent as usize];

        if !self.tags.contains_key(&tag_16.into()) {
            tag_16 = DEVICE_TO_PCS_16[0];
        }

        if let Some(mut pipeline) = self.get_tag(tag_16).map_or(None, IccValue::as_pipeline) {
            if self.color_space == ColorSpace::Lab {
                if let Err(_) = pipeline.prepend_stage(PipelineStage::new_labv4_to_v2()) {
                    return None;
                }
            }

            if self.pcs == ColorSpace::Lab {
                if let Err(_) = pipeline.append_stage(PipelineStage::new_labv2_to_v4()) {
                    return None;
                }
            }

            Some(pipeline)
        } else {
            None
        }
    }

    /// Reads the DToAX tag, adjusting the encoding of Lab or XYZ if needed.
    fn input_float_tag(&self, tag: IccTag) -> Option<Pipeline> {
        let mut pipeline = self.get_tag(tag).map_or(None, IccValue::as_pipeline)?;
        let spc = self.color_space;
        let pcs = self.pcs;

        // input and output of transform are in lcms 0..1 encoding. If XYZ or Lab spaces are used,
        // these need to be normalized into the appropriate ranges (Lab=100,0,0; XYZ=1.0,1.0,1.0)
        if spc == ColorSpace::Lab {
            if let Err(_) = pipeline.prepend_stage(PipelineStage::new_normalize_to_lab_float()) {
                return None;
            }
        } else if spc == ColorSpace::XYZ {
            if let Err(_) = pipeline.prepend_stage(PipelineStage::new_normalize_to_xyz_float()) {
                return None;
            }
        }

        if pcs == ColorSpace::Lab {
            if let Err(_) = pipeline.append_stage(PipelineStage::new_normalize_from_lab_float()) {
                return None;
            }
        } else if pcs == ColorSpace::XYZ {
            if let Err(_) = pipeline.append_stage(PipelineStage::new_normalize_from_xyz_float()) {
                return None;
            }
        }

        Some(pipeline)
    }

    /// Reads the DToAX tag, adjusting the encoding of Lab or XYZ if needed.
    fn output_float_tag(&self, tag: IccTag) -> Option<Pipeline> {
        let mut pipeline = self.get_tag(tag).map_or(None, IccValue::as_pipeline)?;
        let spc = self.color_space;
        let pcs = self.pcs;

        // If PCS is Lab or XYZ, the floating point tag is accepting data in the space encoding,
        // and since the formatter has already accommodated to 0..1.0, we should undo this change
        if pcs == ColorSpace::Lab {
            if let Err(_) = pipeline.prepend_stage(PipelineStage::new_normalize_to_lab_float()) {
                return None;
            }
        } else if pcs == ColorSpace::XYZ {
            if let Err(_) = pipeline.prepend_stage(PipelineStage::new_normalize_to_xyz_float()) {
                return None;
            }
        }

        // the output can be Lab or XYZ, in which case normalization is needed on the end of the
        // pipeline
        if spc == ColorSpace::Lab {
            if let Err(_) = pipeline.append_stage(PipelineStage::new_normalize_from_lab_float()) {
                return None;
            }
        } else if spc == ColorSpace::XYZ {
            if let Err(_) = pipeline.append_stage(PipelineStage::new_normalize_from_xyz_float()) {
                return None;
            }
        }

        Some(pipeline)
    }

    pub fn input_lut(&self, intent: Intent) -> Option<Pipeline> {
        // On named color, take the appropriate tag
        if self.device_class == ProfileClass::NamedColor {
            todo!("named color lists")
        }

        // float tag takes precedence
        let tag_float = DEVICE_TO_PCS_FLOAT[intent as usize];
        if let Some(pipeline) = self.input_float_tag(tag_float) {
            return Some(pipeline);
        }

        let mut tag_16 = DEVICE_TO_PCS_16[intent as usize];

        // Revert to perceptual if no tag is found
        if !self.tags.contains_key(&tag_16.into()) {
            tag_16 = DEVICE_TO_PCS_16[0];
        }

        if let Some(mut pipeline) = self.get_tag(tag_16).map_or(None, IccValue::as_pipeline) {
            // data for lab16 needs to be adjusted on output
            if self.pcs == ColorSpace::Lab {
                if let Some(IccValue::Lut16(_)) = self.get_tag(tag_16) {
                    // if the input is Lab, add a conversion at the beginning
                    if self.color_space == ColorSpace::Lab {
                        if let Err(_) = pipeline.prepend_stage(PipelineStage::new_labv4_to_v2()) {
                            return None;
                        }
                    }
                    // add a matrix for V2 to V4 conversion
                    if let Err(_) = pipeline.append_stage(PipelineStage::new_labv2_to_v4()) {
                        return None;
                    }
                    return Some(pipeline);
                }
            }
            Some(pipeline)
        } else if self.color_space == ColorSpace::Gray {
            // self.build_gray_input_matrix_pipeline()
            todo!()
        } else {
            self.build_rgb_input_matrix_shaper()
        }
    }

    pub fn output_lut(&self, intent: Intent) -> Option<Pipeline> {
        // float tag takes precedence
        let tag_float = PCS_TO_DEVICE_FLOAT[intent as usize];
        if let Some(pipeline) = self.output_float_tag(tag_float) {
            return Some(pipeline);
        }

        let mut tag_16 = PCS_TO_DEVICE_16[intent as usize];

        // Revert to perceptual if no tag is found
        if !self.tags.contains_key(&tag_16.into()) {
            tag_16 = PCS_TO_DEVICE_16[0];
        }

        if let Some(mut pipeline) = self.get_tag(tag_16).map_or(None, IccValue::as_pipeline) {
            // TODO: changeInterpolationToTrilinear for Lab

            if self.pcs == ColorSpace::Lab {
                if let Some(IccValue::Lut16(_)) = self.get_tag(tag_16) {
                    // add a matrix for V4 to V2 conversion
                    if let Err(_) = pipeline.prepend_stage(PipelineStage::new_labv4_to_v2()) {
                        return None;
                    }
                    // if the output is Lab, add a conversion at the end
                    if self.color_space == ColorSpace::Lab {
                        if let Err(_) = pipeline.prepend_stage(PipelineStage::new_labv2_to_v4()) {
                            return None;
                        }
                    }
                    return Some(pipeline);
                }
            }
            Some(pipeline)
        } else if self.color_space == ColorSpace::Gray {
            // self.build_gray_output_pipeline()
            todo!()
        } else {
            self.build_rgb_output_matrix_shaper()
        }
    }

    /// Reads colorants as a matrix if tags exist.
    fn colorant_matrix(&self) -> Option<Matrix3<f64>> {
        let r = self.get_tag(IccTag::RedColorant);
        let g = self.get_tag(IccTag::GreenColorant);
        let b = self.get_tag(IccTag::BlueColorant);

        match (r, g, b) {
            (Some(IccValue::Cxyz(r)), Some(IccValue::Cxyz(g)), Some(IccValue::Cxyz(b))) => {
                Some(Matrix3::from_cols((*r).into(), (*g).into(), (*b).into()))
            }
            _ => None,
        }
    }

    fn build_rgb_input_matrix_shaper(&self) -> Option<Pipeline> {
        const INP_ADJ: f64 = 1. / Cxyz::MAX_ENCODABLE;

        // XYZ PCS is encoded in 1.15 format, and the matrix output comes in the 0..0xffff range, so
        // we need to adjust the output by a factor of (0x10000/0xffff) to put the data in a 1.16
        // range, and then a >> 1 to obtain 1.15. The total factor is (65536.0)/(65535.0*2)
        let cmat = self.colorant_matrix()? * INP_ADJ;

        let curves = vec![
            match self.get_tag(IccTag::RedTRC) {
                Some(IccValue::Curve(t)) => t.clone(),
                _ => return None,
            },
            match self.get_tag(IccTag::GreenTRC) {
                Some(IccValue::Curve(t)) => t.clone(),
                _ => return None,
            },
            match self.get_tag(IccTag::BlueTRC) {
                Some(IccValue::Curve(t)) => t.clone(),
                _ => return None,
            },
        ];

        let mut pipeline = Pipeline::new();

        if let Err(_) = pipeline.append_stage(PipelineStage::new_curve_set(curves)) {
            return None;
        }
        if let Err(_) = pipeline.append_stage(PipelineStage::new_matrix3(cmat, None)) {
            return None;
        }

        Some(pipeline)
    }

    fn build_rgb_output_matrix_shaper(&self) -> Option<Pipeline> {
        const OUT_ADJ: f64 = Cxyz::MAX_ENCODABLE;

        let cmat = self.colorant_matrix()?;

        // see rgb input matrix shaper
        let inv = cmat.invert()? * OUT_ADJ;

        let curves = vec![
            match self.get_tag(IccTag::RedTRC) {
                Some(IccValue::Curve(t)) => t.inverted(),
                _ => return None,
            },
            match self.get_tag(IccTag::GreenTRC) {
                Some(IccValue::Curve(t)) => t.inverted(),
                _ => return None,
            },
            match self.get_tag(IccTag::BlueTRC) {
                Some(IccValue::Curve(t)) => t.inverted(),
                _ => return None,
            },
        ];

        let mut pipeline = Pipeline::new();

        // Note that it is certainly possible that a single profile would have a LUT-based tag for
        // output working in Lab and a matrix-shaper for the fallback cases.
        // This is not allowed by the spec, but this code is tolerant to those cases.
        if self.pcs == ColorSpace::Lab {
            if let Err(_) = pipeline.append_stage(PipelineStage::new_lab_to_xyz()) {
                return None;
            }
        }

        if let Err(_) = pipeline.append_stage(PipelineStage::new_matrix3(inv, None)) {
            return None;
        }
        if let Err(_) = pipeline.append_stage(PipelineStage::new_curve_set(curves)) {
            return None;
        }

        Some(pipeline)
    }

    /// Returns true if the profile is implemented as a matrix-shaper.
    pub(crate) fn is_matrix_shaper(&self) -> bool {
        match self.color_space {
            ColorSpace::Gray => self.tags.contains_key(&IccTag::GrayTRC.into()),
            ColorSpace::RGB => {
                self.tags.contains_key(&IccTag::RedColorant.into())
                    && self.tags.contains_key(&IccTag::GreenColorant.into())
                    && self.tags.contains_key(&IccTag::BlueColorant.into())
                    && self.tags.contains_key(&IccTag::RedTRC.into())
                    && self.tags.contains_key(&IccTag::GreenTRC.into())
                    && self.tags.contains_key(&IccTag::BlueTRC.into())
            }
            _ => false,
        }
    }

    /// Returns true if the intent is implemented as a CLUT.
    pub(crate) fn is_clut(&self, intent: Intent, direction: IntentDirection) -> bool {
        // For devicelinks, the supported intent is that one stated in the header
        if self.device_class == ProfileClass::Link {
            return self.rendering_intent == intent;
        }

        let tag_table = match direction {
            IntentDirection::Input => DEVICE_TO_PCS_16,
            IntentDirection::Output => PCS_TO_DEVICE_16,
            IntentDirection::Proof => {
                return self.is_intent_supported(intent, IntentDirection::Input)
                    && self
                        .is_intent_supported(Intent::RelativeColorimetric, IntentDirection::Output)
            }
        };

        self.tags.contains_key(&tag_table[intent as usize].into())
    }

    pub(crate) fn is_intent_supported(&self, intent: Intent, direction: IntentDirection) -> bool {
        if self.is_clut(intent, direction) {
            return true;
        }

        // Is there any matrix-shaper? If so, the intent is supported. This is a bit odd, since V2
        // matrix shapers do not fully support relative colorimetric because they cannot deal with
        // non-zero black points, but many profiles claims that, and this is certainly not true for
        // V4 profiles. Lets answer "yes" no matter the accuracy
        // would be less than optimal in rel.col and v2 case
        self.is_matrix_shaper()
    }
}

#[allow(dead_code)]
pub(crate) enum IntentDirection {
    Input,
    Output,
    Proof,
}

impl fmt::Debug for IccProfile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Profile {{ created: {}, {:?}, version 0x{:x?}, class: {:?}, pcs {:?}, {:?} intent",
            self.created,
            self.color_space,
            self.version,
            self.device_class,
            self.pcs,
            self.rendering_intent
        )?;
        for (k, v) in &self.tags {
            write!(f, ", {:?}: ", DebugFmtTag(*k))?;
            match v {
                IccTagData::Value(val) => write!(f, "{:?}", val)?,
                IccTagData::Linked(tag) => write!(f, "(-> {:?})", DebugFmtTag(*tag))?,
                IccTagData::Raw(ref buf) => write!(f, "<raw blob of size {}>", buf.len())?,
            }
        }
        write!(f, " }}")?;
        Ok(())
    }
}

struct DebugFmtTag(u32);
impl fmt::Debug for DebugFmtTag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use std::ffi::CStr;
        let x = self.0;
        let a = (x >> 24) as u8;
        let b = ((x >> 16) & 0xFF_u32) as u8;
        let c = ((x >> 8) & 0xFF_u32) as u8;
        let d = (x & 0xFF) as u8;
        let bytes = [a, b, c, d, 0];
        match CStr::from_bytes_with_nul(&bytes) {
            Ok(cstr) => write!(f, "{}", cstr.to_string_lossy()),
            Err(_) => write!(f, "({:02x} {:02x} {:02x} {:02x})", a, b, c, d),
        }
    }
}
