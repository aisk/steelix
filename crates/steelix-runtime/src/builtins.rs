//! Shortcuts for frequently used builtins.
//!
//! They look the builtin up on every call instead of caching it, so patching
//! `builtins.print` keeps working.

use pyo3::call::PyCallArgs;
use pyo3::prelude::*;

/// `print(*args)`
pub fn print<'py>(py: Python<'py>, args: impl PyCallArgs<'py>) -> PyResult<Bound<'py, PyAny>> {
    crate::builtins_module(py)?.getattr("print")?.call1(args)
}
