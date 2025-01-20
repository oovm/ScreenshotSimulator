//! Color transform pipelines.

use crate::color::{CLab, Cxyz, D50};
use crate::tone_curve::ToneCurve;
use cgmath::{Matrix3, Vector3};
use std::error::Error;
use std::{f64, fmt};

/// Maximum amount of color channels the pipeline evaluator can handle.
pub const MAX_STAGE_CHANNELS: usize = 32;

/// Indexes a CLUT.
fn clut_index(cin: usize, cout: usize, size: usize, pos: &[usize]) -> usize {
    let mut index = 0;
    let mut unit = cout;
    for i in 0..cin {
        let j = cin - 1 - i;
        let x = pos[j];
        index += x * unit;
        unit *= size;
    }
    index
}

/// LCMS stage types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageType {
    /// `cvst`
    CurveSet = 0x63767374,

    /// `matf`
    Matrix = 0x6D617466,
    /// `clut`
    CLut = 0x636C7574,

    /// `bACS`
    BAcs = 0x62414353,
    /// `eACS`
    EAcs = 0x65414353,

    /// (non-ICC) `l2x `
    ///
    /// Converts between PCS; 3 inputs & 3 outputs.
    Xyz2Lab = 0x6C327820,

    /// (non-ICC) `x2l `
    ///
    /// Converts between PCS; 3 inputs & 3 outputs.
    Lab2Xyz = 0x78326C20,
    /// (non-ICC) `ncl `
    NamedColor = 0x6E636C20,
    /// (non-ICC) `2 4 `
    LabV2toV4 = 0x32203420,
    /// (non-ICC) `4 2 `
    LabV4toV2 = 0x34203220,

    /// (non-ICC) `idn `
    ///
    /// Copies input to output.
    Identity = 0x69646E20,

    // Float to floatPCS
    /// (non-ICC) `d2l `
    Lab2FloatPCS = 0x64326C20,
    /// (non-ICC) `l2d `
    FloatPCS2Lab = 0x6C326420,
    /// (non-ICC) `d2x `
    XYZ2FloatPCS = 0x64327820,
    /// (non-ICC) `x2d `
    FloatPCS2XYZ = 0x78326420,

    /// (non-ICC) `clp `
    ///
    /// Copies input to output and clamps all values below zero to zero.
    ClipNegatives = 0x636c7020,
}

/// The inner function of a pipeline stage.
#[derive(Debug, Clone, PartialEq)]
pub enum StageKernel {
    /// Copies input to output.
    ///
    /// The parameter indicates the number of channels.
    Identity(usize),
    /// Applies a set of tone curves.
    ///
    /// The number of tone curves is the number of input and output channels.
    CurveSet(Vec<ToneCurve>),
    /// Applies a matrix and an optional offset.
    ///
    /// The number of columns is the number of inputs, and the number of rows the number of outputs.
    Matrix {
        rows: usize,
        matrix: Vec<f64>,
        offset: Option<Vec<f64>>,
    },
    /// Applies a color lookup table.
    CLut {
        /// Input and output channel count.
        channels: (usize, usize),
        /// LUT size in a single dimension.
        size: usize,
        /// LUT grid points.
        data: Vec<u16>,
    },
    /// Converts XYZ to L\*a\*b\*.
    ///
    /// This kernel always has 3 input and 3 output channels.
    Xyz2Lab,
    /// Converts L\*a\*b\* to XYZ.
    ///
    /// This kernel always has 3 input and 3 output channels.
    Lab2Xyz,
    /// Clamps negative values to 0.
    ///
    /// The parameter indicates the number of channels.
    ClipNegatives(usize),
}

impl StageKernel {
    /// Returns the number of input channels.
    pub fn input_channels(&self) -> usize {
        match self {
            Self::Identity(n) => *n,
            Self::CurveSet(c) => c.len(),
            Self::Matrix { rows, matrix, .. } => matrix.len() / rows,
            Self::CLut { channels, .. } => channels.0,
            Self::Xyz2Lab | Self::Lab2Xyz => 3,
            Self::ClipNegatives(n) => *n,
        }
    }

    /// Returns the number of output channels.
    pub fn output_channels(&self) -> usize {
        match self {
            Self::Identity(n) => *n,
            Self::CurveSet(c) => c.len(),
            Self::Matrix { rows, .. } => *rows,
            Self::CLut { channels, .. } => channels.1,
            Self::Xyz2Lab | Self::Lab2Xyz => 3,
            Self::ClipNegatives(n) => *n,
        }
    }

    /// Transforms a color.
    pub fn transform(&self, input: &[f64], output: &mut [f64]) {
        match self {
            Self::Identity(n) => {
                for i in 0..*n {
                    output[i] = input[i];
                }
            }
            Self::CurveSet(c) => {
                for (i, curve) in c.iter().enumerate() {
                    output[i] = curve.eval(input[i]).unwrap_or(f64::NAN);
                }
            }
            Self::Matrix {
                rows,
                matrix,
                offset,
            } => {
                let in_channels = matrix.len() / *rows;
                let out_channels = *rows;
                for i in 0..out_channels {
                    let mut value = 0.;
                    for j in 0..in_channels {
                        value += input[j] * matrix[rows * j + i];
                    }
                    if let Some(offset) = offset {
                        value += offset[i];
                    }
                    output[i] = value;
                }
            }
            Self::CLut {
                channels: (cin, cout),
                size,
                data,
            } => {
                if *size == 2 {
                    // lerp

                    // divide & conquer algorithm for lerping in n dimensions
                    // FIXME: untested
                    fn lerp_dq(
                        chan: (usize, usize, usize),
                        corners: &[u16],
                        pos: &[f64],
                        ipos: &[usize],
                        dim: usize,
                    ) -> f64 {
                        let x = pos[dim];

                        let mut sub_ipos = [0; MAX_STAGE_CHANNELS];

                        // copy ipos into sub_ipos
                        for i in 0..ipos.len() {
                            sub_ipos[i] = ipos[i];
                        }

                        if dim == 0 {
                            let (cin, cout, channel) = chan;
                            sub_ipos[0] = 0;
                            let a = corners[clut_index(cin, cout, 2, &sub_ipos) + channel];
                            sub_ipos[0] = 1;
                            let b = corners[clut_index(cin, cout, 2, &sub_ipos) + channel];

                            ((b - a) as f64 * x + a as f64) / 65535.
                        } else {
                            sub_ipos[dim - 1] = 0;
                            let a = lerp_dq(chan, corners, pos, &sub_ipos, dim - 1);

                            sub_ipos[dim - 1] = 1;
                            let b = lerp_dq(chan, corners, pos, &sub_ipos, dim - 1);
                            (b - a) * x + a
                        }
                    }

                    let top_ipos = [0; MAX_STAGE_CHANNELS];
                    for i in 0..*cout {
                        output[i] = lerp_dq((*cin, *cout, i), data, input, &top_ipos, cin - 1);
                    }
                } else {
                    // simple lookup

                    let mut pos = [0; MAX_STAGE_CHANNELS];
                    for i in 0..*cin {
                        pos[i] = (input[i] * *size as f64) as usize;
                    }
                    let index = clut_index(*cin, *cout, *size, &pos);

                    for i in 0..*cout {
                        output[i] = data[index + i] as f64 / 65535.;
                    }
                }
            }
            Self::Xyz2Lab => {
                // From 0..1.0 to XYZ
                let xyz = Cxyz {
                    x: input[0] * Cxyz::MAX_ENCODABLE,
                    y: input[1] * Cxyz::MAX_ENCODABLE,
                    z: input[2] * Cxyz::MAX_ENCODABLE,
                };

                let lab = xyz.into_lab(D50);

                // From V4 Lab to 0..1.0
                output[0] = lab.l / 100.;
                output[1] = (lab.a + 128.) / 255.;
                output[2] = (lab.b + 128.) / 255.;
            }
            Self::Lab2Xyz => {
                // V4 rules
                let lab = CLab {
                    l: input[0] * 100.,
                    a: input[1] * 255. - 128.,
                    b: input[2] * 255. - 128.,
                };

                let xyz = lab.into_xyz(D50);

                output[0] = xyz.x / Cxyz::MAX_ENCODABLE;
                output[1] = xyz.y / Cxyz::MAX_ENCODABLE;
                output[2] = xyz.z / Cxyz::MAX_ENCODABLE;
            }
            Self::ClipNegatives(n) => {
                for i in 0..*n {
                    output[i] = input[i].max(0.);
                }
            }
        }
    }

    pub fn can_merge_with(&self, other: &StageKernel) -> bool {
        match (self, other) {
            (StageKernel::Matrix { .. }, StageKernel::Matrix { .. }) => true,
            (StageKernel::Xyz2Lab, StageKernel::Lab2Xyz) => true,
            (StageKernel::Lab2Xyz, StageKernel::Xyz2Lab) => true,
            (StageKernel::CurveSet(_), StageKernel::CurveSet(_)) => true,
            _ => false,
        }
    }

    /// Returning Err will always guarantee that nothing was mutated.
    pub fn merge_with(&mut self, other: &StageKernel) -> Result<(), ()> {
        match (self, other) {
            (
                StageKernel::Matrix {
                    rows: rows_a,
                    matrix: matrix_a,
                    offset: offset_a,
                },
                StageKernel::Matrix {
                    rows: rows_b,
                    matrix: matrix_b,
                    offset: offset_b,
                },
            ) => {
                // result_a = matrix_a * in_color + offset_a
                // result_b = matrix_b * result_a + offset_b
                // result_b = matrix_b * (matrix_a * in_color + offset_a) + offset_b
                // result_b = (matrix_b * matrix_a) * in_color + (matrix_b * offset_a + offset_b)

                let out_cols = matrix_a.len() / *rows_a;
                let out_rows = *rows_b;
                let mut out_matrix = Vec::with_capacity(out_cols * out_rows);

                // multiply mat_b * mat_a to get out_matrix
                for i in 0..out_cols {
                    for j in 0..out_rows {
                        let mut value = 0.;

                        for k in 0..*rows_a {
                            value += matrix_b[*rows_b * k + j] * matrix_a[*rows_a * i + k];
                        }

                        out_matrix.push(value);
                    }
                }

                // calculate mat_b * off_a + off_b to get out_offset
                let mut out_offset = Vec::with_capacity(out_rows);

                for i in 0..out_rows {
                    let mut off_value = offset_b.as_ref().map(|off| off[i]).unwrap_or(0.);
                    for j in 0..*rows_a {
                        off_value += matrix_b[*rows_b * j + i]
                            * offset_a.as_ref().map(|off| off[j]).unwrap_or(0.);
                    }
                    out_offset.push(off_value);
                }

                // set out_offset to None if it’s just zeroes
                let out_offset = if out_offset.iter().find(|x| **x != 0.).is_some() {
                    Some(out_offset)
                } else {
                    None
                };

                *rows_a = out_rows;
                *matrix_a = out_matrix;
                *offset_a = out_offset;
                Ok(())
            }
            a @ (StageKernel::Xyz2Lab, StageKernel::Lab2Xyz)
            | a @ (StageKernel::Lab2Xyz, StageKernel::Xyz2Lab) => {
                // these cancel each other out
                *a.0 = StageKernel::Identity(3);
                Ok(())
            }
            (StageKernel::CurveSet(curves_a), StageKernel::CurveSet(curves_b)) => {
                for (i, curve) in curves_a.iter_mut().enumerate() {
                    *curve = curves_b[i].compose_with_approx(curve);
                }

                Ok(())
            }
            _ => Err(()),
        }
    }

    pub fn is_identity(&self) -> bool {
        match self {
            StageKernel::Identity(_) => true,
            StageKernel::Matrix {
                rows,
                matrix,
                offset,
            } => {
                let cols = matrix.len() / rows;
                if *rows == cols {
                    for i in 0..cols {
                        for j in 0..*rows {
                            let ident_value = if i == j { 1. } else { 0. };

                            if matrix[i * rows + j] != ident_value {
                                return false;
                            }
                        }
                    }
                }
                if let Some(offset) = offset {
                    if offset.iter().find(|x| **x != 0.).is_some() {
                        return false;
                    }
                }
                true
            }
            StageKernel::CurveSet(curves) => {
                for curve in curves {
                    if !curve.is_identity() {
                        return false;
                    }
                }
                true
            }
            _ => false,
        }
    }
}

/// A pipeline stage.
#[derive(Debug, Clone, PartialEq)]
pub struct PipelineStage {
    /// The stage type. This is an arbitrary tag only loosely related to the kernel.
    /// Mainly, the kernel should implement the function of what this type purports this stage to
    /// be.
    pub ty: StageType,
    /// The kernel.
    pub kernel: StageKernel,
}

impl PipelineStage {
    pub fn new_identity(channels: usize) -> Self {
        PipelineStage {
            ty: StageType::Identity,
            kernel: StageKernel::Identity(channels),
        }
    }

    /// Creates a new CurveSet pipeline stage.
    pub fn new_curve_set(curves: Vec<ToneCurve>) -> Self {
        PipelineStage {
            ty: StageType::CurveSet,
            kernel: StageKernel::CurveSet(curves),
        }
    }

    /// Creates a new CurveSet pipeline stage with `y = x` tone curves.
    pub fn new_ident_curve_set(channels: usize) -> Self {
        let mut curves = Vec::with_capacity(channels);
        for _ in 0..channels {
            curves.push(ToneCurve::new_gamma(1.));
        }
        PipelineStage {
            ty: StageType::Identity,
            kernel: StageKernel::CurveSet(curves),
        }
    }

    /// Creates a new matrix stage.
    ///
    /// The matrix should be column-major, like in the following 3×3 example:
    ///
    /// ```text
    ///  ⎡0 3 6⎤   ⎡in[0]⎤   ⎡out[0]⎤
    ///  ⎢1 4 7⎥ * ⎢in[1]⎥ = ⎢out[1]⎥
    ///  ⎣2 5 8⎦   ⎣in[2]⎦   ⎣out[2]⎦
    /// ```
    ///
    /// The number of columns corresponds to the number of input channels, and the number of
    /// rows to the number of output channels.
    pub fn new_matrix(rows: usize, matrix: Vec<f64>, offset: Option<Vec<f64>>) -> Self {
        PipelineStage {
            ty: StageType::Matrix,
            kernel: StageKernel::Matrix {
                rows,
                matrix,
                offset,
            },
        }
    }

    /// Convenience function for new_matrix.
    pub fn new_matrix3(matrix: Matrix3<f64>, offset: Option<Vector3<f64>>) -> Self {
        let mut mat_vec = Vec::with_capacity(9);
        mat_vec.resize(9, 0.);
        for i in 0..3 {
            for j in 0..3 {
                mat_vec[i * 3 + j] = matrix[i][j];
            }
        }

        let off_vec = offset.map(|off| vec![off.x, off.y, off.z]);

        Self::new_matrix(3, mat_vec, off_vec)
    }

    /// Creates a new stage that converts Lab V2 to Lab V4.
    pub fn new_labv2_to_v4() -> PipelineStage {
        const V2_TO_V4: [f64; 9] = [
            65535. / 65280.,
            0.,
            0.,
            0.,
            65535. / 65280.,
            0.,
            0.,
            0.,
            65535. / 65280.,
        ];

        let mut stage = Self::new_matrix(3, V2_TO_V4.to_vec(), None);
        stage.ty = StageType::LabV2toV4;
        stage
    }

    /// Creates a new stage that converts Lab V4 to Lab V2.
    pub fn new_labv4_to_v2() -> PipelineStage {
        const V4_TO_V2: [f64; 9] = [
            65280. / 65535.,
            0.,
            0.,
            0.,
            65280. / 65535.,
            0.,
            0.,
            0.,
            65280. / 65535.,
        ];

        let mut stage = Self::new_matrix(3, V4_TO_V2.to_vec(), None);
        stage.ty = StageType::LabV4toV2;
        stage
    }

    /// Creates a new stage that converts XYZ to Lab.
    pub fn new_xyz_to_lab() -> PipelineStage {
        PipelineStage {
            ty: StageType::Xyz2Lab,
            kernel: StageKernel::Xyz2Lab,
        }
    }

    /// Creates a new stage that converts Lab to XYZ.
    pub fn new_lab_to_xyz() -> PipelineStage {
        PipelineStage {
            ty: StageType::Lab2Xyz,
            kernel: StageKernel::Lab2Xyz,
        }
    }

    /// Creates a new stage that clamps all negative values to zero.
    pub fn new_clip_negatives(channels: usize) -> PipelineStage {
        PipelineStage {
            ty: StageType::ClipNegatives,
            kernel: StageKernel::ClipNegatives(channels),
        }
    }

    pub fn new_normalize_from_lab_float() -> PipelineStage {
        todo!()
    }

    pub fn new_normalize_from_xyz_float() -> PipelineStage {
        todo!()
    }

    pub fn new_normalize_to_lab_float() -> PipelineStage {
        todo!()
    }

    pub fn new_normalize_to_xyz_float() -> PipelineStage {
        todo!()
    }

    /// Returns the number of input channels of this stage.
    pub fn input_channels(&self) -> usize {
        self.kernel.input_channels()
    }

    /// Returns the number of output channels of this stage.
    pub fn output_channels(&self) -> usize {
        self.kernel.output_channels()
    }

    /// Transforms a color using the kernel.
    pub fn transform(&self, input: &[f64], output: &mut [f64]) {
        self.kernel.transform(input, output)
    }

    /// Returns true if this stage can be merged with the other stage.
    pub fn can_merge_with(&self, other: &PipelineStage) -> bool {
        self.kernel.can_merge_with(&other.kernel)
    }

    /// Merges this stage with another.
    pub fn merge_with(&mut self, other: &PipelineStage) -> Result<(), ()> {
        self.kernel.merge_with(&other.kernel)?;
        Ok(())
    }

    /// Returns true if this stage essentially has no effect.
    pub fn is_identity(&self) -> bool {
        self.kernel.is_identity()
    }
}

/// A color transform pipeline.
///
/// A pipeline can be built using `append_stage` and can transform a color using `transform`.
#[derive(Debug, Clone, PartialEq)]
pub struct Pipeline {
    stages: Vec<PipelineStage>,
    input_channels: usize,
    output_channels: usize,
}

impl Pipeline {
    /// Creates a new empty pipeline.
    pub fn new() -> Self {
        Pipeline {
            stages: Vec::new(),
            input_channels: 0,
            output_channels: 0,
        }
    }

    /// Returns the number of input channels.
    ///
    /// # Examples
    /// ```
    /// # use rcms::pipeline::*;
    /// let mut pipeline = Pipeline::new();
    ///
    /// // this is a 3 by 3 matrix and hence has 3 input and 3 output channels
    /// pipeline.append_stage(PipelineStage::new_matrix(
    ///     3,
    ///     vec![1., 0., 0., 0., 1., 0., 0., 0., 1.],
    ///     None,
    /// ));
    ///
    /// assert_eq!(pipeline.input_channels(), 3);
    /// ```
    pub fn input_channels(&self) -> usize {
        self.input_channels
    }

    /// Returns the number of output channels.
    pub fn output_channels(&self) -> usize {
        self.output_channels
    }

    /// Updates the number of channels from the first and last stage of this pipeline.
    fn update_channels(&mut self) {
        self.input_channels = self.stages.first().map_or(0, |s| s.input_channels());
        self.output_channels = self.stages.last().map_or(0, |s| s.output_channels());
    }

    /// Returns the inner pipeline stages.
    ///
    /// # Examples
    /// ```
    /// # use rcms::ToneCurve;
    /// # use rcms::pipeline::*;
    /// let stage_a = PipelineStage::new_curve_set(vec![
    ///     ToneCurve::new_gamma(1.0),
    ///     ToneCurve::new_gamma(1.0),
    ///     ToneCurve::new_gamma(1.0),
    /// ]);
    /// let stage_b = PipelineStage::new_identity(3);
    ///
    /// let mut pipeline = Pipeline::new();
    /// pipeline.append_stage(stage_a.clone());
    /// pipeline.append_stage(stage_b.clone());
    ///
    /// let stages = pipeline.stages();
    /// assert_eq!(stages[0], stage_a);
    /// assert_eq!(stages[1], stage_b);
    /// ```
    pub fn stages(&self) -> &[PipelineStage] {
        &self.stages
    }

    /// Prepends a stage.
    ///
    /// The new stage must have the same number of output channels as the first stage has input
    /// channels.
    pub fn prepend_stage(&mut self, stage: PipelineStage) -> Result<(), PipelineError> {
        let stage_output = stage.output_channels();
        let next_input = self.input_channels();

        if !self.stages.is_empty() && stage_output != next_input {
            return Err(PipelineError::ChannelMismatch(stage_output, next_input));
        }

        self.stages.insert(0, stage);
        self.update_channels();

        Ok(())
    }

    /// Appends a stage.
    ///
    /// The new stage must have the same number of input channels as the last stage has input
    /// channels.
    ///
    /// # Examples
    /// ```
    /// # use rcms::pipeline::*;
    /// let mut pipeline = Pipeline::new();
    ///
    /// // this stage has 1 input and 3 outputs.
    /// // an empty pipeline accepts any number of channels
    /// pipeline.append_stage(PipelineStage::new_matrix(
    ///     3,
    ///     vec![1., 2., 3.],
    ///     None,
    /// )).unwrap();
    ///
    /// // the pipeline now has 3 outputs
    /// assert_eq!(pipeline.output_channels(), 3);
    ///
    /// // this stage has 3 inputs and 1 output.
    /// // the 3 inputs are compatible with the 3 outputs of the pipeline as it is currently, so
    /// // this action will succeed.
    /// pipeline.append_stage(PipelineStage::new_matrix(
    ///     1,
    ///     vec![1., 2., 3.],
    ///     None,
    /// )).unwrap();
    ///
    /// // this stage as 2 inputs and 2 outputs and cannot be added to this pipeline
    /// pipeline.append_stage(PipelineStage::new_identity(2)).unwrap_err();
    /// ```
    pub fn append_stage(&mut self, stage: PipelineStage) -> Result<(), PipelineError> {
        let prev_output = self.output_channels();
        let stage_input = stage.input_channels();

        if !self.stages.is_empty() && prev_output != stage_input {
            return Err(PipelineError::ChannelMismatch(prev_output, stage_input));
        }

        self.stages.push(stage);
        self.update_channels();

        Ok(())
    }

    /// Removes the first stage and returns it.
    pub fn pop_front_stage(&mut self) -> Option<PipelineStage> {
        let stage = self.stages.remove(0);
        self.update_channels();
        Some(stage)
    }

    /// Removes the last stage and returns it.
    pub fn pop_back_stage(&mut self) -> Option<PipelineStage> {
        let stage = self.stages.pop();
        self.update_channels();
        stage
    }

    /// Appends another pipeline to this one.
    ///
    /// The number of output channels of this pipeline and the number of input channels of the other
    /// must match.
    ///
    /// This has the same semantics as Vec::append.
    pub fn append(&mut self, other: &mut Pipeline) -> Result<(), PipelineError> {
        let prev_output = self.output_channels();
        let next_input = other.input_channels();

        if !self.stages.is_empty() && !other.stages.is_empty() && prev_output != next_input {
            return Err(PipelineError::ChannelMismatch(prev_output, next_input));
        }

        self.stages.append(&mut other.stages);
        self.update_channels();

        Ok(())
    }

    /// Evaluates the pipeline for a single color.
    ///
    /// The input and output arrays should be appropriately sized:
    ///
    /// - input should be at least input_channels in size
    /// - output should be at least output_channels in size
    pub fn transform(&self, input: &[f64], output: &mut [f64]) {
        let mut phase = 0;
        let mut storage = [[0.; MAX_STAGE_CHANNELS], [0.; MAX_STAGE_CHANNELS]];

        for i in 0..self.input_channels() {
            storage[phase][i] = input[i];
        }

        for stage in &self.stages {
            let next_phase = phase ^ 1;

            let (a, b) = storage.split_at_mut(1);
            let (src_value, dest_value) = if next_phase == 0 { (b, a) } else { (a, b) };
            stage.transform(&src_value[0], &mut dest_value[0]);

            phase = next_phase;
        }

        for i in 0..self.output_channels() {
            output[i] = storage[phase][i];
        }
    }

    /// Attemps to (destructively) optimize this pipeline by merging and eliminating unnecessary
    /// stages.
    ///
    /// Note that performing this action may resample tone curves. This will reduce the domain in
    /// which they are defined to a potentially smaller one.
    ///
    /// # Examples
    /// ```
    /// # use rcms::{*, link::*, profile::*};
    /// let srgb = IccProfile::new_srgb();
    /// let aces_cg = IccProfile::new_aces_cg();
    ///
    /// let color = [0.1, 0.2, 0.3];
    ///
    /// let mut unoptimized_pipeline = link(
    ///     &[&srgb, &aces_cg],
    ///     &[Intent::Perceptual, Intent::Perceptual],
    ///     &[false, false],
    ///     &[0., 0.],
    /// ).expect("failed to link profiles");
    ///
    /// let mut optimized_pipeline = unoptimized_pipeline.clone();
    /// optimized_pipeline.optimize();
    ///
    /// let mut out_color = [0.; 3];
    /// let mut out_color2 = [0.; 3];
    ///
    /// unoptimized_pipeline.transform(&color, &mut out_color);
    /// optimized_pipeline.transform(&color, &mut out_color2);
    ///
    /// assert!((out_color[0] - out_color2[0]).abs() < 1e-5);
    /// assert!((out_color[1] - out_color2[1]).abs() < 1e-5);
    /// assert!((out_color[2] - out_color2[2]).abs() < 1e-5);
    /// ```
    pub fn optimize(&mut self) {
        if self.stages.len() < 2 {
            return;
        }

        let mut i = 0;
        while i < self.stages.len() {
            let (stages_left, stages_right) = self.stages.split_at_mut(i + 1);
            let stage = &mut stages_left[i];
            let next_stage = &stages_right.get(0);

            let mut did_merge_next = false;

            if let Some(next_stage) = next_stage {
                if stage.can_merge_with(next_stage) {
                    if let Ok(()) = stage.merge_with(next_stage) {
                        did_merge_next = true;
                    }
                }
            }

            let is_redundant = stage.is_identity();

            if did_merge_next {
                self.stages.remove(i + 1);
            }
            if is_redundant {
                self.stages.remove(i);
            }

            // OpenColorIO notes how stages like these will often be symmetric like A B B A, so
            // in the case where B B are merged and removed, the resulting A A might also be
            // optimizable
            if is_redundant {
                i = i.saturating_sub(1);
            } else {
                i += 1;
            }
        }
    }
}

/// Pipeline errors.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum PipelineError {
    /// Output (0) and input (1) channels do not match.
    ChannelMismatch(usize, usize),
}

impl fmt::Display for PipelineError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PipelineError::ChannelMismatch(o, i) => write!(
                f,
                "pipeline channel mismatch: {} output channels feeding into {} input channels",
                o, i
            ),
        }
    }
}

impl Error for PipelineError {}
