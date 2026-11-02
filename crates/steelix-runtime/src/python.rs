//! Mixed mode: running the parts of a module that are still Python source.

use std::ffi::{CString, c_int};

use pyo3::exceptions::PyValueError;
use pyo3::ffi;
use pyo3::prelude::*;
use pyo3::types::PyString;

use crate::Module;

/// `CO_FUTURE_ANNOTATIONS`, the compiler flag of `from __future__ import annotations`.
const FUTURE_ANNOTATIONS: c_int = 0x100_0000;

/// Compiles `source` and runs it in the module's globals, like the module
/// level statements it was cut from.
///
/// The code gets the module's file name and its original line numbers
/// (`source` starts at line `line`), so tracebacks, `inspect.getsource` and
/// the functions and classes it defines look exactly as they would in
/// CPython. Use it through [`exec_python!`](crate::exec_python).
pub fn exec_python(
    module: &Module<'_>,
    line: usize,
    source: &str,
    future_annotations: bool,
) -> PyResult<()> {
    let py = module.py();
    let globals = module.globals();
    let filename = match globals.get_item("__file__")? {
        Some(file) if file.is_instance_of::<PyString>() => file,
        _ => PyString::new(py, "<steelix>").into_any(),
    };
    // Blank lines in front put the first statement on line `line`.
    let padded = "\n".repeat(line.saturating_sub(1)) + source;
    let source = CString::new(padded)
        .map_err(|_| PyValueError::new_err("source code cannot contain null bytes"))?;
    let mut flags = ffi::PyCompilerFlags {
        cf_flags: if future_annotations {
            FUTURE_ANNOTATIONS
        } else {
            0
        },
        cf_feature_version: c_int::from(py.version_info().minor),
    };
    // SAFETY: a valid C string, `str` object and flags; the result is a new reference or NULL.
    let code = unsafe {
        Bound::from_owned_ptr_or_err(
            py,
            ffi::Py_CompileStringObject(
                source.as_ptr(),
                filename.as_ptr(),
                ffi::Py_file_input,
                &mut flags,
                -1,
            ),
        )?
    };
    // SAFETY: `code` is a code object and `globals` a dict; the result is a
    // new reference or NULL.
    unsafe {
        Bound::from_owned_ptr_or_err(
            py,
            ffi::PyEval_EvalCode(code.as_ptr(), globals.as_ptr(), globals.as_ptr()),
        )?;
    }
    Ok(())
}

/// Runs module level statements that are kept as Python source (mixed mode,
/// design §13), in place and in the module's globals.
///
/// The source is a raw string literal starting on the line after `r#"`, at
/// line `line` of the Python file:
///
/// ```ignore
/// rt::exec_python!(m, line = 12, r#"
/// class Point:
///     ...
/// "#)?;
/// ```
///
/// `future_annotations` before the source compiles it like a module with
/// `from __future__ import annotations`.
#[macro_export]
macro_rules! exec_python {
    ($module:expr, line = $line:literal, future_annotations, $source:literal $(,)?) => {
        $crate::python::exec_python(
            $module,
            $line,
            $crate::python::strip_first_newline($source),
            true,
        )
    };
    ($module:expr, line = $line:literal, $source:literal $(,)?) => {
        $crate::python::exec_python(
            $module,
            $line,
            $crate::python::strip_first_newline($source),
            false,
        )
    };
}

/// The source of `exec_python!` without the line break after `r#"`.
#[doc(hidden)]
pub fn strip_first_newline(source: &str) -> &str {
    source.strip_prefix('\n').unwrap_or(source)
}
