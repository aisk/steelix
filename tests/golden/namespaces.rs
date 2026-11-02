//! Parts of `namespaces.py` kept as Python source and run by
//! `rt::exec_python!` in `exec_module` (steelix does not lower them yet):
//!
//! - line 8, `def local_names`: locals() in a function

use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use steelix_runtime as rt;

/// Executes the body of `namespaces.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    m.set_global(
        "__doc__",
        "`dir()`, `vars()` and `locals()` without arguments read the current scope.",
    )?;
    m.set_global("x", 1)?;
    m.global("print")?.call1((
        m.dir()?.contains("x")?,
        m.dir()?.rich_compare(
            m.global("sorted")?
                .call1((m.globals().clone().into_any(),))?,
            CompareOp::Eq,
        )?,
    ))?;
    m.global("print")?.call1((
        m.globals()
            .clone()
            .into_any()
            .is(m.globals().clone().into_any()),
        m.globals()
            .clone()
            .into_any()
            .is(m.globals().clone().into_any()),
    ))?;
    // Kept as Python: locals() in a function at 10:19
    rt::exec_python!(
        m,
        line = 8,
        r#"
def local_names(a):
    b = a + 1
    return sorted(locals()), dir()
"#
    )?;
    m.global("print")?
        .call1((m.global("local_names")?.call1((1,))?,))?;
    Ok(())
}
