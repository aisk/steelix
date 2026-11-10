use pyo3::prelude::*;
use steelix_runtime as rt;

/// Executes the body of `ns/deep/leaf.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    m.set_global("LEAF", "leaf")?;
    rt::builtins::print(m.py(), ("importing", m.global("__name__")?))?;
    Ok(())
}
