//! In-place operators for augmented assignment (`x += y`).
//!
//! PyO3 only exposes the binary operators (`add`, `sub`, ...), which never
//! mutate. `x += y` must call `__iadd__` first so that, for example, a list is
//! extended in place, so these wrap the `PyNumber_InPlace*` functions. The
//! names follow the `operator` module.

use pyo3::IntoPyObjectExt;
use pyo3::ffi;
use pyo3::prelude::*;

macro_rules! in_place {
    ($($(#[$doc:meta])* $name:ident => $ffi:ident;)*) => {$(
        $(#[$doc])*
        pub fn $name<'py>(
            target: &Bound<'py, PyAny>,
            value: impl IntoPyObject<'py>,
        ) -> PyResult<Bound<'py, PyAny>> {
            let py = target.py();
            let value = value.into_bound_py_any(py)?;
            // SAFETY: both pointers are valid for the duration of the call and
            // the result is a new reference or NULL with an exception set.
            unsafe { Bound::from_owned_ptr_or_err(py, ffi::$ffi(target.as_ptr(), value.as_ptr())) }
        }
    )*};
}

in_place! {
    /// `target += value`
    iadd => PyNumber_InPlaceAdd;
    /// `target -= value`
    isub => PyNumber_InPlaceSubtract;
    /// `target *= value`
    imul => PyNumber_InPlaceMultiply;
    /// `target @= value`
    imatmul => PyNumber_InPlaceMatrixMultiply;
    /// `target /= value`
    itruediv => PyNumber_InPlaceTrueDivide;
    /// `target //= value`
    ifloordiv => PyNumber_InPlaceFloorDivide;
    /// `target %= value`
    imod => PyNumber_InPlaceRemainder;
    /// `target <<= value`
    ilshift => PyNumber_InPlaceLshift;
    /// `target >>= value`
    irshift => PyNumber_InPlaceRshift;
    /// `target &= value`
    iand => PyNumber_InPlaceAnd;
    /// `target |= value`
    ior => PyNumber_InPlaceOr;
    /// `target ^= value`
    ixor => PyNumber_InPlaceXor;
}

/// `target **= value`
pub fn ipow<'py>(
    target: &Bound<'py, PyAny>,
    value: impl IntoPyObject<'py>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = target.py();
    let value = value.into_bound_py_any(py)?;
    // SAFETY: as above; `Py_None` is the "no modulus" argument of `pow`.
    unsafe {
        Bound::from_owned_ptr_or_err(
            py,
            ffi::PyNumber_InPlacePower(target.as_ptr(), value.as_ptr(), ffi::Py_None()),
        )
    }
}
