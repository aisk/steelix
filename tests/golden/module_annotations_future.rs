//! Parts of `module_annotations_future.py` kept as Python source and run by
//! `rt::exec_python!` in `exec_module` (steelix does not lower them yet):
//!
//! - line 6, `annassign` statement: annotated assignment at module level
//! - line 10, `class Box`: class definition
//! - line 14, `annassign` statement: annotated assignment at module level

use pyo3::prelude::*;
use pyo3::types::PyDict;
use steelix_runtime as rt;

/// Executes the body of `module_annotations_future.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    // The module's `__annotations__`, which CPython creates before running it
    m.set_global("__annotations__", PyDict::new(py))?;
    // With `from __future__ import annotations`, module annotations go into one
    // `__annotations__` dict, created before the first statement as in CPython.
    let __future__ = rt::import_names(m, "__future__", &["annotations"])?;
    m.set_global("annotations", rt::import_from(&__future__, "annotations")?)?;
    rt::builtins::print(py, (m.dir()?.contains("__annotations__")?,))?;
    // module_annotations_future.py:6, kept as Python: annotated assignment at module level at 6:1
    rt::exec_python!(
        m,
        line = 6,
        future_annotations,
        r#"
A: int = 1
"#
    )?;
    rt::builtins::print(py, (m.global("__annotations__")?,))?;
    // module_annotations_future.py:10, kept as Python: class definition at 10:1
    rt::exec_python!(
        m,
        line = 10,
        future_annotations,
        r#"
class Box:
    pass
"#
    )?;
    // module_annotations_future.py:14, kept as Python: annotated assignment at module level at 14:1
    rt::exec_python!(
        m,
        line = 14,
        future_annotations,
        r#"
B: Box = Box()
"#
    )?;
    rt::builtins::print(py, (m.global("__annotations__")?,))?;
    Ok(())
}
