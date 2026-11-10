use pyo3::prelude::*;
use steelix_runtime as rt;

/// Executes the body of `json/shadowed.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    rt::builtins::print(m.py(), ("never imported",))?;
    Ok(())
}
