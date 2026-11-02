//! Mixed mode: running the parts of a module that are still Python source.

use std::ffi::{CString, c_int};

use pyo3::exceptions::PyValueError;
use pyo3::ffi;
use pyo3::prelude::*;
use pyo3::types::{PySet, PyString};

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
///
/// Statements from the body of a module level `if`, `for` or `while` keep
/// their indentation, so that column offsets match the file too. They are
/// compiled as the body of an `if True:` on the line before.
///
/// Module level annotations: CPython 3.14 compiles a module's annotations
/// into one `__annotate__` function, which [`module_annotate`] defines up
/// front, and records which annotated assignments have run in the set
/// `__conditional_annotations__` (by index in the module). A block compiled
/// on its own numbers its annotations from 0 and replaces both globals.
/// `first_annotation` is the module's index of the block's first annotation:
/// the block's indices are added to the module's set, and the module's
/// `__annotate__` and set are put back afterwards.
pub fn exec_python(
    module: &Module<'_>,
    line: usize,
    source: &str,
    future_annotations: bool,
    first_annotation: Option<usize>,
) -> PyResult<()> {
    // Blank lines in front put the first statement on line `line`.
    let padded = if source.starts_with([' ', '\t']) {
        "\n".repeat(line.saturating_sub(2)) + "if True:\n" + source
    } else {
        "\n".repeat(line.saturating_sub(1)) + source
    };
    let code = compile(module, padded, future_annotations)?;
    let globals = module.globals();
    let saved = match first_annotation {
        Some(_) => (
            globals.get_item("__annotate__")?,
            globals.get_item("__conditional_annotations__")?,
        ),
        None => (None, None),
    };
    let result = eval(module, &code);
    if let (Some(annotate), Some(executed), Some(first)) = (saved.0, saved.1, first_annotation) {
        if let Some(own) = globals.get_item("__conditional_annotations__")?
            && !own.is(&executed)
        {
            for index in own.try_iter()? {
                executed.call_method1("add", (index?.extract::<usize>()? + first,))?;
            }
        }
        globals.set_item("__conditional_annotations__", executed)?;
        globals.set_item("__annotate__", annotate)?;
    }
    result
}

/// Defines the module's `__annotate__` for its module level annotations
/// (`(line, "name: annotation")`, in source order), as CPython does before
/// running the first statement of a module.
///
/// The annotated assignments themselves are kept as Python and run later by
/// [`exec_python!`](crate::exec_python), each block compiled on its own. This
/// gives the module one `__annotate__` covering all of them, with the
/// annotations on their original lines, as if the whole module had been
/// compiled at once. Like CPython's, it only returns the annotations whose
/// assignments have run (recorded in `__conditional_annotations__`).
pub fn module_annotate(module: &Module<'_>, annotations: &[(usize, &str)]) -> PyResult<()> {
    let mut source = String::new();
    let mut current = 1;
    for &(line, text) in annotations {
        if line > current {
            source.push_str(&"\n".repeat(line - current));
            current = line;
        } else if !source.is_empty() {
            source.push_str("; ");
        }
        source.push_str(text);
        current += text.matches('\n').count();
    }
    source.push('\n');
    let code = compile(module, source, false)?;
    eval(module, &code)?;
    // None of the assignments has run yet.
    module
        .globals()
        .set_item("__conditional_annotations__", PySet::empty(module.py())?)
}

/// Runs module code in the module's globals.
fn eval(module: &Module<'_>, code: &Bound<'_, PyAny>) -> PyResult<()> {
    let globals = module.globals();
    // SAFETY: `code` is a code object and `globals` a dict; the result is a
    // new reference or NULL.
    unsafe {
        Bound::from_owned_ptr_or_err(
            module.py(),
            ffi::PyEval_EvalCode(code.as_ptr(), globals.as_ptr(), globals.as_ptr()),
        )?;
    }
    Ok(())
}

/// Compiles `source` as module code with the module's file name.
fn compile<'py>(
    module: &Module<'py>,
    source: String,
    future_annotations: bool,
) -> PyResult<Bound<'py, PyAny>> {
    let py = module.py();
    let filename = match module.globals().get_item("__file__")? {
        Some(file) if file.is_instance_of::<PyString>() => file,
        _ => PyString::new(py, "<steelix>").into_any(),
    };
    let source = CString::new(source)
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
    unsafe {
        Bound::from_owned_ptr_or_err(
            py,
            ffi::Py_CompileStringObject(
                source.as_ptr(),
                filename.as_ptr(),
                ffi::Py_file_input,
                &mut flags,
                -1,
            ),
        )
    }
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
/// `from __future__ import annotations`. `first_annotation = N` marks a block
/// with module level annotations, the first of which is the module's
/// annotation number `N` (see [`module_annotate`]).
#[macro_export]
macro_rules! exec_python {
    ($module:expr, line = $line:literal, future_annotations, $source:literal $(,)?) => {
        $crate::python::exec_python(
            $module,
            $line,
            $crate::python::strip_first_newline($source),
            true,
            None,
        )
    };
    ($module:expr, line = $line:literal, first_annotation = $first:literal, $source:literal $(,)?) => {
        $crate::python::exec_python(
            $module,
            $line,
            $crate::python::strip_first_newline($source),
            false,
            Some($first),
        )
    };
    ($module:expr, line = $line:literal, $source:literal $(,)?) => {
        $crate::python::exec_python(
            $module,
            $line,
            $crate::python::strip_first_newline($source),
            false,
            None,
        )
    };
}

/// The source of `exec_python!` without the line break after `r#"`.
#[doc(hidden)]
pub fn strip_first_newline(source: &str) -> &str {
    source.strip_prefix('\n').unwrap_or(source)
}
