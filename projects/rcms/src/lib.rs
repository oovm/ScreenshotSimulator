//! Color management library.
//!
//! Heavily based on Little CMS.
//!
//! # Examples
//! ## Converting a Color from sRGB to Display P3
//! See [`link`](link/fn.link.html) for details on linking color profiles.
//!
//! ```
//! # use rcms::{*, color::*, link::link, profile::Intent};
//! let srgb_profile = IccProfile::new_srgb();
//! let p3_profile = IccProfile::new_display_p3();
//!
//! // a light-bluish color in sRGB
//! // colors are represented as arrays of floating-point numbers, usually in the range
//! // from 0 to 1.
//! let some_color = [0.3, 0.5, 0.9];
//!
//! // create a transform from sRGB to Display P3
//! let transform_pipeline = link(
//!     &[&srgb_profile, &p3_profile],
//!     &[Intent::Perceptual, Intent::Perceptual],
//!     &[false, false],
//!     &[0., 0.],
//! ).expect("failed to create color transform");
//!
//! // this array will hold the output color in Display P3
//! let mut out_color = [0.; 3];
//! transform_pipeline.transform(&some_color, &mut out_color);
//!
//! // reference values from ColorSync
//! let out_reference: [f64; 3] = [0.3462, 0.4949, 0.8724];
//!
//! // check that apart from floating point inaccuracies, the output color is correct
//! assert!((out_color[0] - out_reference[0]).abs() < 1e-3);
//! assert!((out_color[1] - out_reference[1]).abs() < 1e-3);
//! assert!((out_color[2] - out_reference[2]).abs() < 1e-3);
//! ```

#[macro_use]
mod util;

mod black_point;
pub mod color;
pub mod fixed;
pub mod link;
pub mod pipeline;
pub mod profile;
pub mod tone_curve;

pub use profile::IccProfile;
pub use tone_curve::ToneCurve;
