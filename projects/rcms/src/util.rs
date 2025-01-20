use cgmath::prelude::*;
use cgmath::{Matrix3, Vector3};

/// Creates a repr(primitive) enum with conversion to/from the primitive type.
macro_rules! enum_primitive {
    (
        $(#[$meta:meta])* pub $name:ident ($ty:tt)
        { $($(#[$vmeta:meta])* $variant:ident = $value:expr,)+ }
    ) => {
        enum_primitive!(__ $($meta)*; pub; $name; $ty; $($($vmeta)*; $variant, $value),+);
    };
    (
        __ $($meta:meta)*; $prefix:tt; $name:ident; $ty:tt;
        $($($vmeta:meta)*; $variant:ident, $value:expr),+
    ) => {
        $(#[$meta])*
        #[repr($ty)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        $prefix enum $name {
            $($(#[$vmeta])* $variant = $value,)+
        }

        impl std::convert::TryFrom<$ty> for $name {
            type Error = $ty;
            fn try_from(value: $ty) -> Result<$name, $ty> {
                match value {
                    $(
                        $value => Ok($name::$variant),
                    )+
                    _ => Err(value),
                }
            }
        }

        impl Into<$ty> for $name {
            fn into(self) -> $ty {
                self as $ty
            }
        }
    };
}

pub(crate) fn lcms_mat3_per(a: Matrix3<f64>, b: Matrix3<f64>) -> Matrix3<f64> {
    // LCMS Mat3per’s arguments are swapped
    b * a
}

pub(crate) fn lcms_mat3_eval(a: Matrix3<f64>, v: Vector3<f64>) -> Vector3<f64> {
    // LCMS matrix multiplication is transposed for some reason, so here’s an extra function
    // (this took me four hours to track down)
    a.transpose() * v
}

pub(crate) fn lcms_solve_matrix(matrix: Matrix3<f64>, b: Vector3<f64>) -> Option<Vector3<f64>> {
    match matrix.invert() {
        Some(inverse) => Some(lcms_mat3_eval(inverse, b)),
        None => None,
    }
}
