//! Parts of `mixed_future_annotations.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 8, `def kept`: yield

use pyo3::prelude::*;
use steelix_runtime as rt;

// mixed_future_annotations.py:4
pub fn native<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "native", 4, |_| { Ok(x.str()?.into_any()) })
}

pub mod box_ {
    use super::*;

    // mixed_future_annotations.py:12
    /// The body of `class Box`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Box", 12, |_| {
            ns.setup_annotations()?;
            ns.annotate("size", "Size")?;
            Ok(())
        })
    }
}

/// Executes the body of `mixed_future_annotations.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    rt::frame!(m, "<module>", 1, |_| {
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
        m.set_global("Box", rt::class!(m, "Box"(), box_::body, line = 12)?)?;
        rt::builtins::print(
            m.py(),
            (
                m.global("native")?.getattr("__annotations__")?,
                m.global("kept")?.getattr("__annotations__")?,
                m.global("Box")?.getattr("__annotations__")?,
            ),
        )?;
        Ok(())
    })
}
