use pyo3::prelude::*;
use steelix_runtime as rt;

// builtin_names.py:9
pub fn show<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "show", 9, |_| {
        rt::builtins::print(py, (m.global("answer")?,))?;
        rt::builtins::print(py, (m.global("__annotations__")?,))?;
        Ok(rt::none(py))
    })
}

/// Executes the body of `builtin_names.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    rt::frame!(m, "<module>", 1, |_| {
        // Names that are not globals are looked up in the namespace of `builtins`,
        // like `LOAD_GLOBAL` does, not among the attributes of the module object:
        // `__annotations__` is an attribute of every module but not a builtin.
        m.set_global("builtins", rt::import(m, "builtins")?)?;
        m.global("builtins")?.setattr("answer", 42)?;
        rt::def!(m, show())?;
        show(m)?;
        Ok(())
    })
}
