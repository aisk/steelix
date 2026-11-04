use pyo3::prelude::*;
use steelix_runtime as rt;

/// Executes the body of `ns/deep/leaf.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global("LEAF", "leaf")?;
        rt::builtins::print(m.py(), ("importing", m.global("__name__")?))?;
        Ok(())
    })
}
