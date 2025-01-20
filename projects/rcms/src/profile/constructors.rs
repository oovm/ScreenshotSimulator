use crate::color::build_rgb_to_xyz_transfer_matrix;
use crate::color::{CxyY, Cxyz, D50, D65};
use crate::fixed::s15f16;
use crate::pipeline::{Pipeline, PipelineStage};
use crate::profile::mlu::Mlu;
use crate::profile::{ColorSpace, IccProfile, IccTag, IccValue, ProfileClass};
use crate::tone_curve::ToneCurve;
use std::convert::TryFrom;

impl IccProfile {
    fn set_description(&mut self, description: &str) {
        let mut desc_mlu = Mlu::new();
        let mut copyright = Mlu::new();

        desc_mlu.insert("en", "US", description.into());
        copyright.insert("en", "US", "No copyright, use freely".into());

        self.insert_tag(IccTag::ProfileDescription, IccValue::Mlu(desc_mlu));
        self.insert_tag(IccTag::Copyright, IccValue::Mlu(copyright));
    }

    /// Creates a new RGB color profile.
    fn new_rgb_base(white_point: CxyY, primaries: Option<(CxyY, CxyY, CxyY)>) -> Option<Self> {
        let mut profile = IccProfile::new(ProfileClass::Display, ColorSpace::RGB);
        profile.set_version(4, 3);
        profile.set_description("RGB color profile");

        profile.insert_tag(IccTag::MediaWhitePoint, IccValue::Cxyz(D50));
        let chad = Cxyz::from(white_point).adaptation_matrix(D50, None)?;

        let mut chad_vec = Vec::with_capacity(9);
        chad_vec.resize(9, s15f16::ZERO);
        for i in 0..3 {
            for j in 0..3 {
                chad_vec[i * 3 + j] = match s15f16::try_from(chad[i][j]) {
                    Ok(n) => n,
                    Err(_) => return None,
                };
            }
        }

        // this is a v4 tag but many CMS understand and read it regardless
        profile.insert_tag(
            IccTag::ChromaticAdaptation,
            IccValue::S15Fixed16Array(chad_vec),
        );

        if let Some(primaries) = primaries {
            let max_white = CxyY {
                x: white_point.x,
                y: white_point.y,
                Y: 1.,
            };

            let m_colorants = match build_rgb_to_xyz_transfer_matrix(max_white, primaries) {
                Some(mat) => mat,
                None => return None,
            };

            profile.insert_tag(
                IccTag::RedColorant,
                IccValue::Cxyz(Cxyz {
                    x: m_colorants[0][0],
                    y: m_colorants[1][0],
                    z: m_colorants[2][0],
                }),
            );
            profile.insert_tag(
                IccTag::GreenColorant,
                IccValue::Cxyz(Cxyz {
                    x: m_colorants[0][1],
                    y: m_colorants[1][1],
                    z: m_colorants[2][1],
                }),
            );
            profile.insert_tag(
                IccTag::BlueColorant,
                IccValue::Cxyz(Cxyz {
                    x: m_colorants[0][2],
                    y: m_colorants[1][2],
                    z: m_colorants[2][2],
                }),
            );

            profile.insert_tag(
                IccTag::Chromaticity,
                IccValue::Chromaticity(primaries.0, primaries.1, primaries.2),
            );
        }

        Some(profile)
    }

    /// Creates a new RGB color profile.
    pub fn new_rgb_with_curves(
        white_point: CxyY,
        primaries: (CxyY, CxyY, CxyY),
        curves: (ToneCurve, ToneCurve, ToneCurve),
    ) -> Option<Self> {
        let mut profile = Self::new_rgb_base(white_point, Some(primaries))?;

        let rg_eq = curves.0 == curves.1;
        let rb_eq = curves.0 == curves.2;

        profile.insert_tag(IccTag::RedTRC, IccValue::Curve(curves.0));

        if rg_eq {
            profile.link_tag(IccTag::GreenTRC, IccTag::RedTRC);
        } else {
            profile.insert_tag(IccTag::GreenTRC, IccValue::Curve(curves.1));
        }

        if rb_eq {
            profile.link_tag(IccTag::BlueTRC, IccTag::RedTRC);
        } else {
            profile.insert_tag(IccTag::BlueTRC, IccValue::Curve(curves.2));
        }

        Some(profile)
    }

    /// Creates a new RGB color profile.
    pub fn new_rgb(white_point: CxyY, primaries: (CxyY, CxyY, CxyY), gamma: f64) -> Option<Self> {
        let mut profile = Self::new_rgb_base(white_point, Some(primaries))?;

        profile.insert_tag(IccTag::RedTRC, IccValue::Curve(ToneCurve::new_gamma(gamma)));
        profile.link_tag(IccTag::GreenTRC, IccTag::RedTRC);
        profile.link_tag(IccTag::BlueTRC, IccTag::RedTRC);

        Some(profile)
    }

    fn new_srgb_tone_curve() -> ToneCurve {
        // sRGB Curves are defined by:
        //
        // If R'sRGB, G'sRGB, B'sRGB < 0.04045
        //
        //    R =  R'sRGB / 12.92
        //    G =  G'sRGB / 12.92
        //    B =  B'sRGB / 12.92
        //
        // else if  R'sRGB,G'sRGB, B'sRGB >= 0.04045
        //
        //    R = ((R'sRGB + 0.055) / 1.055) ^ 2.4
        //    G = ((G'sRGB + 0.055) / 1.055) ^ 2.4
        //    B = ((B'sRGB + 0.055) / 1.055) ^ 2.4
        ToneCurve::new_icc_parametric(3, &[2.4, 1. / 1.055, 0.055 / 1.055, 1. / 12.92, 0.04045])
            .expect("this should not have happened (failed to create sRGB tone curve)")
    }

    /// Creates a new sRGB profile.
    pub fn new_srgb() -> Self {
        let primaries = (
            CxyY {
                x: 0.6400,
                y: 0.3300,
                Y: 1.0,
            },
            CxyY {
                x: 0.3000,
                y: 0.6000,
                Y: 1.0,
            },
            CxyY {
                x: 0.1500,
                y: 0.0600,
                Y: 1.0,
            },
        );
        let mut profile = Self::new_rgb_base(D65.into(), Some(primaries))
            .expect("this should not have happened (failed to create sRGB)");

        profile.insert_tag(IccTag::RedTRC, IccValue::Curve(Self::new_srgb_tone_curve()));
        profile.link_tag(IccTag::GreenTRC, IccTag::RedTRC);
        profile.link_tag(IccTag::BlueTRC, IccTag::RedTRC);

        profile.set_description("sRGB color profile");

        profile
    }

    /// Creates a new ACEScg profile.
    pub fn new_aces_cg() -> Self {
        let mut profile = Self::new_rgb(
            CxyY {
                x: 0.32168,
                y: 0.33767,
                Y: 1.,
            },
            (
                CxyY {
                    x: 0.713,
                    y: 0.293,
                    Y: 1.,
                },
                CxyY {
                    x: 0.165,
                    y: 0.830,
                    Y: 1.,
                },
                CxyY {
                    x: 0.128,
                    y: 0.044,
                    Y: 1.,
                },
            ),
            1.,
        )
        .expect("this should not have happened (failed to creates ACEScg)");
        profile.set_description("compatible with ACEScg");
        profile
    }

    /// Creates a new Display P3 profile.
    pub fn new_display_p3() -> Self {
        let primaries = (
            CxyY {
                x: 0.680,
                y: 0.320,
                Y: 1.0,
            },
            CxyY {
                x: 0.265,
                y: 0.690,
                Y: 1.0,
            },
            CxyY {
                x: 0.150,
                y: 0.060,
                Y: 1.0,
            },
        );
        let mut profile = Self::new_rgb_base(D65.into(), Some(primaries))
            .expect("this should not have happened (failed to create Display P3)");

        profile.insert_tag(IccTag::RedTRC, IccValue::Curve(Self::new_srgb_tone_curve()));
        profile.link_tag(IccTag::GreenTRC, IccTag::RedTRC);
        profile.link_tag(IccTag::BlueTRC, IccTag::RedTRC);

        profile.set_description("compatible with Display P3");

        profile
    }

    /// Creates a new Adobe RGB (1998) compatible profile.
    pub fn new_adobe_rgb() -> Self {
        let mut profile = Self::new_rgb(
            CxyY {
                x: 0.3127,
                y: 0.3290,
                Y: 1.0,
            },
            (
                CxyY {
                    x: 0.6400,
                    y: 0.3300,
                    Y: 1.0,
                },
                CxyY {
                    x: 0.2100,
                    y: 0.0700,
                    Y: 1.0,
                },
                CxyY {
                    x: 0.1500,
                    y: 0.0600,
                    Y: 1.0,
                },
            ),
            563. / 256.,
        )
        .expect("this should not have happened (failed to create Adobe RGB)");

        profile.set_description("compatible with Adobe RGB (1998)");

        profile
    }

    /// Creates a Lab V2 identity profile.
    pub fn new_lab2(white_point: CxyY) -> Option<Self> {
        let mut profile = Self::new_rgb_base(white_point, None)?;

        profile.device_class = ProfileClass::Abstract;
        profile.color_space = ColorSpace::Lab;
        profile.pcs = ColorSpace::Lab;

        profile.set_description("built-in Lab identity");

        // An identity LUT is all we need
        let mut lut = Pipeline::new();
        if let Err(_) = lut.prepend_stage(PipelineStage::new_identity(3)) {
            return None;
        }
        profile.insert_tag(IccTag::AToB0, IccValue::Pipeline(lut));

        Some(profile)
    }

    /// Creates a Lab V4 identity profile.
    pub fn new_lab4(white_point: CxyY) -> Option<Self> {
        let mut profile = Self::new_rgb_base(white_point, None)?;
        profile.set_version(4, 3);

        profile.device_class = ProfileClass::Abstract;
        profile.color_space = ColorSpace::Lab;
        profile.pcs = ColorSpace::Lab;

        profile.set_description("built-in Lab identity");

        // An identity LUT is all we need
        let mut lut = Pipeline::new();
        if let Err(_) = lut.prepend_stage(PipelineStage::new_ident_curve_set(3)) {
            return None;
        }
        profile.insert_tag(IccTag::AToB0, IccValue::Pipeline(lut));

        Some(profile)
    }
}

#[test]
fn create_profile_sanity_check() {
    IccProfile::new_srgb();
    IccProfile::new_aces_cg();
    IccProfile::new_display_p3();
    IccProfile::new_adobe_rgb();
    IccProfile::new_lab2(D50.into());
    IccProfile::new_lab4(D50.into());
}

#[test]
fn round_trips() {
    use crate::link::link;
    use crate::profile::Intent;

    let srgb = IccProfile::new_srgb();
    let acescg = IccProfile::new_aces_cg();

    let srgb2aces = link(
        &[&srgb, &acescg],
        &[Intent::RelativeColorimetric, Intent::RelativeColorimetric],
        &[false, false],
        &[0., 0.],
    )
    .unwrap();

    let aces2srgb = link(
        &[&acescg, &srgb],
        &[Intent::RelativeColorimetric, Intent::RelativeColorimetric],
        &[false, false],
        &[0., 0.],
    )
    .unwrap();

    let pixel = [0.1, 0.3, 0.5];
    let mut pixel_aces = [0.; 3];
    let mut pixel_roundtrip = [0.; 3];

    srgb2aces.transform(&pixel, &mut pixel_aces);
    aces2srgb.transform(&pixel_aces, &mut pixel_roundtrip);

    println!("{:?} -> {:?} -> {:?}", pixel, pixel_aces, pixel_roundtrip);

    for i in 0..3 {
        assert!((pixel_roundtrip[i] - pixel[i]).abs() < 1e-5, "oh no");
    }

    // -----

    let srgb2aces = link(
        &[&srgb, &acescg],
        &[Intent::RelativeColorimetric, Intent::Perceptual],
        &[false, false],
        &[0., 0.],
    )
    .unwrap();

    let aces2srgb = link(
        &[&acescg, &srgb],
        &[Intent::RelativeColorimetric, Intent::Perceptual],
        &[false, false],
        &[0., 0.],
    )
    .unwrap();

    let pixel = [0.5, 0.1, 0.7];
    let mut pixel_srgb = [0.; 3];
    let mut pixel_roundtrip = [0.; 3];

    aces2srgb.transform(&pixel, &mut pixel_srgb);
    srgb2aces.transform(&pixel_srgb, &mut pixel_roundtrip);

    println!("{:?} -> {:?} -> {:?}", pixel, pixel_srgb, pixel_roundtrip);

    let pixel_srgb_ref = [0.87143475, 0.22537376, 0.89735174];

    for i in 0..3 {
        assert!((pixel_roundtrip[i] - pixel[i]).abs() < 1e-4, "oh no");
        assert!(
            (pixel_srgb[i] - pixel_srgb_ref[i]).abs() < 1e-4,
            "oh no (2)"
        );
    }
}
