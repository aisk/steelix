use pyo3::prelude::*;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// A nested `if` stays nested, as in the source (clippy's `collapsible_if` is
// allowed at the crate root).
// nested_if.py:5
#[rt::frame(m, "check", 5)]
pub fn check<'py>(
    m: &rt::Module<'py>,
    keep: &Bound<'py, PyAny>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    if keep.is_truthy()? {
        if !value.is_none() {
            rt::builtins::print(py, ("keep", value))?;
        }
    }
    if value.is_instance(&rt::builtin(py, "tuple")?)? {
        if value.contains(1)? && !value.contains(true)? {
            return Ok(rt::bool(py, false));
        }
    }
    Ok(rt::bool(py, true))
}

/// Executes the body of `nested_if.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::def!(m, check(keep, value))?;
    rt::builtins::print(
        py,
        (
            check(m, &rt::bool(py, true), &rt::int(py, 1))?,
            check(m, &rt::bool(py, false), &rt::int(py, 2))?,
            check(m, &rt::bool(py, true), &rt::none(py))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            check(m, &rt::bool(py, true), PyTuple::new(py, [1, 2])?.as_any())?,
            check(m, &rt::bool(py, true), PyTuple::new(py, [true])?.as_any())?,
            check(m, &rt::bool(py, false), PyTuple::new(py, [0])?.as_any())?,
        ),
    )?;
    Ok(())
}
