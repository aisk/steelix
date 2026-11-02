//! Unpacking assignment: `a, b = value` and `a, *rest, b = value`.
//!
//! This follows CPython's `_PyEval_UnpackIterableStackRef` step by step, so
//! the iterator is consumed the same way and the error messages match.

use std::ffi::CStr;

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::ffi;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyIterator, PyList, PyTuple};

/// `[a, b] = value`: unpacks exactly `N` items.
///
/// ```ignore
/// let [key, value] = rt::unpack(&item)?;
/// ```
pub fn unpack<'py, const N: usize>(value: &Bound<'py, PyAny>) -> PyResult<[Bound<'py, PyAny>; N]> {
    let mut iter = iterate(value)?;
    let items = take::<N>(&mut iter, |got| {
        format!("not enough values to unpack (expected {N}, got {got})")
    })?;
    match iter.next() {
        None => Ok(items),
        Some(Err(err)) => Err(err),
        Some(Ok(_)) => {
            // Only for exact lists, tuples and dicts the size is reported.
            let size = if value.is_exact_instance_of::<PyList>()
                || value.is_exact_instance_of::<PyTuple>()
                || value.is_exact_instance_of::<PyDict>()
            {
                Some(value.len()?)
            } else {
                None
            };
            Err(PyValueError::new_err(match size {
                Some(size) if size > N => {
                    format!("too many values to unpack (expected {N}, got {size})")
                }
                _ => format!("too many values to unpack (expected {N})"),
            }))
        }
    }
}

/// `a, *rest, b = value`: unpacks `B` items before the starred target and `A`
/// after it; the starred target gets a list of everything in between.
///
/// ```ignore
/// let ([first], rest, []) = rt::unpack_star(&value)?;
/// ```
#[allow(clippy::type_complexity)]
pub fn unpack_star<'py, const B: usize, const A: usize>(
    value: &Bound<'py, PyAny>,
) -> PyResult<(
    [Bound<'py, PyAny>; B],
    Bound<'py, PyAny>,
    [Bound<'py, PyAny>; A],
)> {
    let expected = B + A;
    let mut iter = iterate(value)?;
    let before = take::<B>(&mut iter, |got| {
        format!("not enough values to unpack (expected at least {expected}, got {got})")
    })?;
    let rest = PyList::empty(value.py());
    for item in iter {
        rest.append(item?)?;
    }
    let len = rest.len();
    if len < A {
        return Err(PyValueError::new_err(format!(
            "not enough values to unpack (expected at least {expected}, got {})",
            B + len
        )));
    }
    let after = std::array::from_fn(|i| {
        rest.get_item(len - A + i)
            .expect("index is within the list")
    });
    rest.del_slice(len - A, len)?;
    Ok((before, rest.into_any(), after))
}

fn iterate<'py>(value: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyIterator>> {
    value.try_iter().map_err(|err| {
        let py = value.py();
        let ty = value.get_type_ptr();
        // SAFETY: reading slots of the type of a valid object.
        let (not_iterable, type_name) = unsafe {
            (
                (*ty).tp_iter.is_none() && ffi::PySequence_Check(value.as_ptr()) == 0,
                CStr::from_ptr((*ty).tp_name).to_string_lossy(),
            )
        };
        if err.is_instance_of::<PyTypeError>(py) && not_iterable {
            PyTypeError::new_err(format!("cannot unpack non-iterable {type_name} object"))
        } else {
            err
        }
    })
}

fn take<'py, const N: usize>(
    iter: &mut Bound<'py, PyIterator>,
    message: impl Fn(usize) -> String,
) -> PyResult<[Bound<'py, PyAny>; N]> {
    let mut items = Vec::with_capacity(N);
    while items.len() < N {
        match iter.next() {
            Some(item) => items.push(item?),
            None => return Err(PyValueError::new_err(message(items.len()))),
        }
    }
    Ok(items
        .try_into()
        .unwrap_or_else(|_| unreachable!("exactly N items were collected")))
}
