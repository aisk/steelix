//! Parts of `mixed_future_annotations.py` kept as Python source and run by
//! `rt::exec_python!` in `exec_module` (steelix does not lower them yet):
//!
//! - line 8, `def kept`: yield
//! - line 12, `class Box`: class definition, annotated assignment

use pyo3::prelude::*;
use steelix_runtime as rt;

// mixed_future_annotations.py:4
pub fn native<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    Ok(x.str()?.into_any())
}

/// Executes the body of `mixed_future_annotations.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let __future__ = rt::import_names(m, "__future__", &["annotations"])?;
    m.set_global("annotations", rt::import_from(&__future__, "annotations")?)?;
    rt::def!(m, native(x: "'int'") -> "'str'")?;
    // mixed_future_annotations.py:8, kept as Python: yield at 9:5
    rt::exec_python!(
        m,
        line = 8,
        future_annotations,
        r#"
def kept(x: int) -> Iterator[str]:
    yield str(x)
"#
    )?;
    // mixed_future_annotations.py:12, kept as Python: class definition at 12:1, annotated assignment at 13:5
    rt::exec_python!(
        m,
        line = 12,
        future_annotations,
        r#"
class Box:
    size: Size
"#
    )?;
    rt::builtins::print(
        m.py(),
        (
            m.global("native")?.getattr("__annotations__")?,
            m.global("kept")?.getattr("__annotations__")?,
            m.global("Box")?.getattr("__annotations__")?,
        ),
    )?;
    Ok(())
}
