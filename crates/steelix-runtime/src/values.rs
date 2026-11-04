//! Python values built from the literals of generated code: numbers,
//! strings, slices and f-strings.

use pyo3::IntoPyObjectExt;
use pyo3::ffi;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyBytes, PyComplex, PyFloat, PyInt, PySlice, PyString, PyTuple};

/// Builds a Python `int` from a Rust integer literal.
pub fn int<'py, T>(py: Python<'py>, value: T) -> Bound<'py, PyAny>
where
    T: IntoPyObject<
            'py,
            Target = PyInt,
            Output = Bound<'py, PyInt>,
            Error = std::convert::Infallible,
        >,
{
    let Ok(obj) = value.into_pyobject(py);
    obj.into_any()
}

/// Builds a Python `int` that does not fit in `i128`, from its decimal digits.
pub fn bigint<'py>(py: Python<'py>, digits: &str) -> PyResult<Bound<'py, PyAny>> {
    py.get_type::<PyInt>().call1((digits,))
}

/// Builds a Python `float`.
pub fn float(py: Python<'_>, value: f64) -> Bound<'_, PyAny> {
    PyFloat::new(py, value).into_any()
}

/// Builds a Python `complex`, for an imaginary literal like `2j`.
pub fn complex(py: Python<'_>, real: f64, imag: f64) -> Bound<'_, PyAny> {
    PyComplex::from_doubles(py, real, imag).into_any()
}

/// Builds a Python `str`.
pub fn str<'py>(py: Python<'py>, value: &str) -> Bound<'py, PyAny> {
    PyString::new(py, value).into_any()
}

/// Builds a Python `str` containing lone surrogates, which a Rust `&str` can
/// not hold. `encoded` is the UTF-8 encoding produced with `surrogatepass`.
pub fn str_surrogatepass<'py>(py: Python<'py>, encoded: &[u8]) -> PyResult<Bound<'py, PyAny>> {
    PyBytes::new(py, encoded).call_method1("decode", ("utf-8", "surrogatepass"))
}

/// Builds a Python `bytes`.
pub fn bytes<'py>(py: Python<'py>, value: &[u8]) -> Bound<'py, PyAny> {
    PyBytes::new(py, value).into_any()
}

/// Returns `True` or `False`.
pub fn bool(py: Python<'_>, value: bool) -> Bound<'_, PyAny> {
    PyBool::new(py, value).to_owned().into_any()
}

/// Returns `None`.
pub fn none(py: Python<'_>) -> Bound<'_, PyAny> {
    py.None().into_bound(py)
}

/// `start:stop` in a subscript: a `slice` object whose step is `None`.
///
/// `PySlice::new` only takes integers, but Python slices hold any object and
/// keep omitted parts as `None`, which `__getitem__` implementations can see.
pub fn slice<'py>(
    py: Python<'py>,
    start: impl IntoPyObject<'py>,
    stop: impl IntoPyObject<'py>,
) -> PyResult<Bound<'py, PySlice>> {
    slice_step(py, start, stop, py.None())
}

/// `start:stop:step` in a subscript.
pub fn slice_step<'py>(
    py: Python<'py>,
    start: impl IntoPyObject<'py>,
    stop: impl IntoPyObject<'py>,
    step: impl IntoPyObject<'py>,
) -> PyResult<Bound<'py, PySlice>> {
    let start = start.into_bound_py_any(py)?;
    let stop = stop.into_bound_py_any(py)?;
    let step = step.into_bound_py_any(py)?;
    // SAFETY: valid object pointers; `PySlice_New` returns a new reference.
    unsafe {
        Bound::from_owned_ptr_or_err(
            py,
            ffi::PySlice_New(start.as_ptr(), stop.as_ptr(), step.as_ptr()),
        )
        .map(|slice| slice.cast_into_unchecked())
    }
}

/// `format(value, spec)`, which an f-string replacement field `{value:spec}`
/// calls. A field without a spec, `{value}`, passes `""`.
pub fn format<'py>(
    value: &Bound<'py, PyAny>,
    spec: impl IntoPyObject<'py>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = value.py();
    let spec = spec.into_bound_py_any(py)?;
    if value.is_exact_instance_of::<PyString>() && spec.is_empty()? {
        // `FORMAT_SIMPLE` returns exact strings unchanged.
        return Ok(value.clone());
    }
    // SAFETY: valid object pointers; the result is a new reference or NULL.
    unsafe { Bound::from_owned_ptr_or_err(py, ffi::PyObject_Format(value.as_ptr(), spec.as_ptr())) }
}

/// `ascii(value)`, the `!a` conversion of an f-string.
pub fn ascii<'py>(value: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyString>> {
    // SAFETY: valid object pointer; the result is a new `str` reference or NULL.
    unsafe {
        Bound::from_owned_ptr_or_err(value.py(), ffi::PyObject_ASCII(value.as_ptr()))
            .map(|s| s.cast_into_unchecked())
    }
}

/// Concatenates the pieces of an f-string, like `"".join(pieces)`.
///
/// ```ignore
/// rt::fstring(py, ("x = ", rt::format(&x, ".2f")?, "!"))?
/// ```
pub fn fstring<'py>(
    py: Python<'py>,
    pieces: impl IntoPyObject<'py, Target = PyTuple>,
) -> PyResult<Bound<'py, PyAny>> {
    let pieces = pieces.into_bound_py_any(py)?;
    let empty = PyString::new(py, "");
    // SAFETY: valid object pointers; the result is a new reference or NULL.
    unsafe {
        Bound::from_owned_ptr_or_err(py, ffi::PyUnicode_Join(empty.as_ptr(), pieces.as_ptr()))
    }
}
