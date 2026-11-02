use pyo3::prelude::*;
use steelix_runtime as rt;

// vars_argument.py:6
pub fn double<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    x.mul(2)
}

/// Executes the body of `vars_argument.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    // `vars(obj)` reads the namespace of `obj`, not the module's, so the module
    // still calls its functions directly (design §7).
    m.set_global("types", rt::import(m, "types")?)?;
    rt::def!(m, double(x))?;
    m.set_global(
        "point",
        rt::call!(m.global("types")?.getattr("SimpleNamespace")?, x = 1, y = 2)?,
    )?;
    rt::builtins::print(
        py,
        (
            rt::builtin(py, "vars")?.call1((m.global("point")?,))?,
            double(m, &rt::int(py, 21))?,
        ),
    )?;
    Ok(())
}
