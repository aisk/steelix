//! Shortcuts for frequently used builtins, called with positional arguments:
//! `rt::builtins::sorted(py, (&xs,))?` is `sorted(xs)`.
//!
//! They look the builtin up on every call instead of caching it, so patching
//! `builtins.print` keeps working.

use pyo3::call::PyCallArgs;
use pyo3::prelude::*;

macro_rules! shortcuts {
    ($($name:ident)*) => {$(
        #[doc = concat!("`", stringify!($name), "(*args)`")]
        pub fn $name<'py>(
            py: Python<'py>,
            args: impl PyCallArgs<'py>,
        ) -> PyResult<Bound<'py, PyAny>> {
            crate::builtin(py, stringify!($name))?.call1(args)
        }
    )*};
}

// Keep in sync with `_SHORTCUT_BUILTINS` in `src/steelix/lower/calls.py`.
shortcuts! {
    all any callable chr dict divmod enumerate filter float frozenset getattr
    hasattr hash id int issubclass list map max min next open ord print range
    reversed round set setattr sorted sum tuple zip
}
