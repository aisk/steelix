pub mod count;
pub mod greet;
pub mod r#static;

use pyo3::prelude::*;
use steelix_runtime as rt;

/// Executes the body of `plugins/__init__.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    rt::frame!(m, "<module>", 1, |_| {
        rt::builtins::print(m.py(), ("plugins: imported",))?;
        Ok(())
    })
}
