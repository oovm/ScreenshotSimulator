use crate::color::{CLab, Cxyz, D50};
use crate::link::link;
use crate::pipeline::Pipeline;
use crate::profile::{ColorSpace, IccProfile, Intent, IntentDirection, ProfileClass};
use crate::util::lcms_solve_matrix;
use cgmath::{Matrix3, Vector3};

const PERCEPTUAL_BLACK: Cxyz = Cxyz {
    x: 0.00336,
    y: 0.0034731,
    z: 0.00287,
};
const ZERO: Cxyz = Cxyz {
    x: 0.,
    y: 0.,
    z: 0.,
};

/// This function shouldn’t exist at all, but there is such a quantity of broken black point tags
/// that we must somehow fix chromaticity to avoid tinting when doing black point compensation.
/// This function does just that. There is a special flag for using the black point tag
/// (Rust note: there is not), but it’s turned off by default because it’s bogus on most profiles.
/// The detection algorithm involves turning the black point neutral and only using the L component.
pub(crate) fn detect_black_point(profile: &IccProfile, intent: Intent) -> Cxyz {
    let dev_class = profile.device_class;

    // Make sure the device class is adequate
    if dev_class == ProfileClass::Link
        || dev_class == ProfileClass::Abstract
        || dev_class == ProfileClass::NamedColor
    {
        return ZERO;
    }

    // Make sure intent is adequate
    if intent != Intent::Perceptual
        && intent != Intent::RelativeColorimetric
        && intent != Intent::Saturation
    {
        return ZERO;
    }

    // v4 + perceptual & saturation intents does have its own black point, and it is
    // well specified enough to use it. Black point tag is deprecated in V4.
    if profile.version > 0x4000000 && (intent == Intent::Perceptual || intent == Intent::Saturation)
    {
        // Matrix shaper share MRC & perceptual intents
        if profile.is_matrix_shaper() {
            return black_point_as_darker_colorant(profile, Intent::RelativeColorimetric);
        }

        // Get perceptual black out of v4 profiles. That is fixed for perceptual & saturation intents
        return PERCEPTUAL_BLACK;
    }

    // That is about v2 profiles.

    // If output profile, discount ink-limiting and that's all
    if intent == Intent::RelativeColorimetric
        && dev_class == ProfileClass::Output
        && profile.color_space == ColorSpace::CMYK
    {
        return black_point_using_perceptual_black(profile);
    }

    // Nope, compute BP using current intent.
    return black_point_as_darker_colorant(profile, intent);
}

fn end_points_by_space(space: ColorSpace) -> Option<([f64; 4], [f64; 4])> {
    // only most common spaces

    let rgb_black = [0., 0., 0., 0.];
    let rgb_white = [1., 1., 1., 0.];
    let cmyk_black = [1., 1., 1., 1.]; // 400% of ink
    let cmyk_white = [0., 0., 0., 0.];
    let lab_black = [0., 0.5, 0.5, 0.]; // V4 Lab encoding
    let lab_white = [1., 0.5, 0.5, 0.];
    let cmy_black = [1., 1., 1., 0.];
    let cmy_white = [0., 0., 0., 0.];
    let gray_black = [0.; 4];
    let gray_white = [1., 0., 0., 0.];

    match space {
        ColorSpace::Gray => Some((gray_white, gray_black)),
        ColorSpace::RGB => Some((rgb_white, rgb_black)),
        ColorSpace::Lab => Some((lab_white, lab_black)),
        ColorSpace::CMYK => Some((cmyk_white, cmyk_black)),
        ColorSpace::CMY => Some((cmy_white, cmy_black)),
        _ => None,
    }
}

/// Use darker colorants to obtain black point. This works in the relative colorimetric intent and
/// assumes more ink results in darker colors. No ink limit is assumed.
fn black_point_as_darker_colorant(profile: &IccProfile, intent: Intent) -> Cxyz {
    // If the profile does not support the input direction, assume black point 0
    if !profile.is_intent_supported(intent, IntentDirection::Input) {
        return ZERO;
    }

    // Try to get black by using black colorant
    let space = profile.color_space;

    // This function returns darker colorant in 16 bits for several spaces
    let (black, channels) = match end_points_by_space(space) {
        Some(p) => p,
        None => return ZERO,
    };

    // TODO: check if this isn’t supposed to be total_channels
    if channels.len() != profile.color_space.channels() {
        return ZERO;
    }

    // Lab will be used as the output space, but lab2 will avoid recursion
    let lab_profile = IccProfile::new_lab2(D50.into()).unwrap();

    // Create the transform
    let pipeline = match link(
        &[&profile, &lab_profile],
        &[intent, intent],
        &[false, false],
        &[0., 0.],
    ) {
        Ok(pipeline) => pipeline,
        Err(_) => return ZERO,
    };

    // Convert black to Lab
    let mut lab = CLab {
        l: 0.,
        a: 0.,
        b: 0.,
    };
    pipeline.transform(&black, lab.as_slice_mut());

    // Force it to be neutral, clip to max. L* of 50
    lab.a = 0.;
    lab.b = 0.;
    lab.l = lab.l.min(50.);

    // Convert from Lab (which is now clipped) to XYZ.
    let black_xyz = lab.into_xyz(D50);

    return black_xyz;
}

// PCS -> PCS round trip transform, always uses relative intent on the device -> pcs
fn create_roundtrip_xform(profile: &IccProfile, intent: Intent) -> Option<Pipeline> {
    let lab_profile = IccProfile::new_lab4(D50.into())?;
    let bpc: [bool; 4] = [false, false, false, false];
    let states: [f64; 4] = [1., 1., 1., 1.];
    let profiles: [&IccProfile; 4] = [&lab_profile, &profile, &profile, &lab_profile];
    let intents: [Intent; 4] = [
        Intent::RelativeColorimetric,
        intent,
        Intent::RelativeColorimetric,
        Intent::RelativeColorimetric,
    ];

    link(&profiles, &intents, &bpc, &states).ok()
}

/// Get a black point of output CMYK profile, discounting any ink-limiting embedded
/// in the profile. For doing that, we use perceptual intent in input direction:
/// Lab (0, 0, 0) -> [Perceptual] Profile -> CMYK -> [Rel. colorimetric] Profile -> Lab
fn black_point_using_perceptual_black(profile: &IccProfile) -> Cxyz {
    // Is the intent supported by the profile?
    if !profile.is_intent_supported(Intent::Perceptual, IntentDirection::Input) {
        return ZERO;
    }

    let round_trip = match create_roundtrip_xform(profile, Intent::Perceptual) {
        Some(pipeline) => pipeline,
        None => return ZERO,
    };

    let lab_in = CLab {
        l: 0.,
        a: 0.,
        b: 0.,
    };
    let mut lab_out = CLab {
        l: 0.,
        a: 0.,
        b: 0.,
    };
    round_trip.transform(lab_in.as_slice(), lab_out.as_slice_mut());

    // clip Lab to reasonable limits
    lab_out.a = 0.;
    lab_out.b = 0.;
    lab_out.l = lab_out.l.min(50.);

    lab_out.into_xyz(D50)
}

/// Calculates the black point of a destination profile.
/// This algorithm comes from the Adobe paper disclosing its black point compensation method.
pub(crate) fn detect_dest_black_point(profile: &IccProfile, intent: Intent) -> Cxyz {
    let dev_class = profile.device_class;

    // Make sure the device class is adequate
    if dev_class == ProfileClass::Link
        || dev_class == ProfileClass::Abstract
        || dev_class == ProfileClass::NamedColor
    {
        return ZERO;
    }

    // Make sure intent is adequate
    if intent != Intent::Perceptual
        && intent != Intent::RelativeColorimetric
        && intent != Intent::Saturation
    {
        return ZERO;
    }

    // v4 + perceptual & saturation intents does have its own black point, and it is
    // well specified enough to use it. Black point tag is deprecated in V4.
    if profile.version >= 0x4000000
        && (intent == Intent::Perceptual || intent == Intent::Saturation)
    {
        if profile.is_matrix_shaper() {
            // Matrix shapers share MRC & perceptual intents
            return black_point_as_darker_colorant(profile, Intent::RelativeColorimetric);
        }

        // Get perceptual black out of v4 profiles. This is fixed for perceptual & saturation intents
        return PERCEPTUAL_BLACK;
    }

    // Check if the profile is lut based and gray, rgb or cmyk (7.2 in Adobe's document)
    let color_space = profile.color_space;
    if !profile.is_clut(intent, IntentDirection::Output)
        || (color_space != ColorSpace::Gray
            && color_space != ColorSpace::RGB
            && color_space != ColorSpace::CMYK)
    {
        // In this case, handle as input case
        return detect_black_point(profile, intent);
    }

    // It is one of the valid cases!, use Adobe algorithm

    // Set a first guess, that should work on good profiles.
    let initial_lab = if intent == Intent::RelativeColorimetric {
        // calculate initial Lab as source black point
        let ini_xyz = detect_black_point(profile, intent);

        // convert the XYZ to lab
        ini_xyz.into_lab(D50)
    } else {
        // set the initial Lab to zero, that should be the black point for perceptual and saturation
        CLab {
            l: 0.,
            a: 0.,
            b: 0.,
        }
    };

    // Step 2
    // ======

    // Create a roundtrip. Define a Transform BT for all x in L*a*b*
    let round_trip = match create_roundtrip_xform(profile, intent) {
        Some(pipeline) => pipeline,
        None => return ZERO,
    };

    // Compute ramps

    let mut in_ramp = [0.; 256];
    let mut out_ramp = [0.; 256];

    for l in 0..256 {
        let lab = CLab {
            l: l as f64 * 100. / 255.,
            a: initial_lab.a.max(-50.).min(50.),
            b: initial_lab.b.max(-50.).min(50.),
        };

        let in_lab = [lab.l, lab.a, lab.b];
        let mut out_lab = [0.; 3];
        round_trip.transform(&in_lab, &mut out_lab);

        in_ramp[l] = lab.l;
        out_ramp[l] = out_lab[0];
    }

    // Make monotonic
    for l in (1..=254).rev() {
        out_ramp[l] = out_ramp[l].min(out_ramp[l + 1]);
    }

    // Check
    if out_ramp[0] >= out_ramp[255] {
        return ZERO;
    }

    // Test for mid range straight (only on relative colorimetric)
    let mut nearly_straight_midrange = true;
    let min_l = out_ramp[0];
    let max_l = out_ramp[255];

    if intent == Intent::RelativeColorimetric {
        for l in 0..256 {
            if !(in_ramp[l] <= min_l + 0.2 * (max_l - min_l)
                || (in_ramp[l] - out_ramp[l]).abs() < 4.)
            {
                nearly_straight_midrange = false;
                break;
            }
        }

        // If the mid range is straight (as determined above) then the
        // DestinationBlackPoint shall be the same as initialLab.
        // Otherwise, the DestinationBlackPoint shall be determined
        // using curve fitting.
        if nearly_straight_midrange {
            return initial_lab.into_xyz(D50);
        }
    }

    // curve fitting: The round-trip curve normally looks like a nearly constant section at the black point,
    // with a corner and a nearly straight line to the white point.
    let mut y_ramp = [0.; 256];
    for l in 0..256 {
        y_ramp[l] = (out_ramp[l] - min_l) / (max_l - min_l);
    }

    // find the black point using the least squares error quadratic curve fitting
    let (lo, hi) = if intent == Intent::RelativeColorimetric {
        (0.1, 0.5)
    } else {
        // Perceptual and saturation
        (0.03, 0.25)
    };

    // Capture shadow points for the fitting.
    let mut x = [0.; 256];
    let mut y = [0.; 256];
    let mut n = 0;
    for l in 0..256 {
        let ff = y_ramp[l];

        if ff >= lo && ff < hi {
            x[n] = in_ramp[l];
            y[n] = y_ramp[l];
            n += 1;
        }
    }

    // No suitable points
    if n < 3 {
        return ZERO;
    }

    CLab {
        // fit and get the vertex of quadratic curve
        // clip to zero L* if the vertex is negative
        l: root_of_least_squares_fit_quadratic_curve(n, x, y).max(0.),
        a: initial_lab.a,
        b: initial_lab.b,
    }
    .into_xyz(D50)
}

// Least Squares Fit of a Quadratic Curve to Data
// http://www.personal.psu.edu/jhm/f90/lectures/lsq2.html
fn root_of_least_squares_fit_quadratic_curve(n: usize, x: [f64; 256], y: [f64; 256]) -> f64 {
    if n < 4 {
        return 0.;
    }

    let mut sum_x = 0.;
    let mut sum_x2 = 0.;
    let mut sum_x3 = 0.;
    let mut sum_x4 = 0.;
    let mut sum_y = 0.;
    let mut sum_yx = 0.;
    let mut sum_yx2 = 0.;

    for i in 0..n {
        let xn = x[i];
        let yn = y[i];

        sum_x += xn;
        sum_x2 += xn * xn;
        sum_x3 += xn * xn * xn;
        sum_x4 += xn * xn * xn * xn;

        sum_y += yn;
        sum_yx += yn * xn;
        sum_yx2 += yn * xn * xn;
    }

    let matrix = Matrix3::from_cols(
        (n as f64, sum_x, sum_x2).into(),
        (sum_x, sum_x2, sum_x3).into(),
        (sum_x2, sum_x3, sum_x4).into(),
    );

    let vec = Vector3::new(sum_y, sum_yx, sum_yx2);

    let res = match lcms_solve_matrix(matrix, vec) {
        Some(res) => res,
        None => return 0.,
    };

    let a = res[2];
    let b = res[1];
    let c = res[0];

    if a.abs() < 1e-10 {
        (-c / b).max(50.).min(0.)
    } else {
        let d = b * b - 4. * a * c;
        if d <= 0. {
            0.
        } else {
            let rt = (-b + d.sqrt()) / (2. * a);
            rt.max(50.).min(0.)
        }
    }
}
