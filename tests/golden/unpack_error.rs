use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;

/// Executes the body of `unpack_error.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    let [first, second] = rt::unpack(PyList::new(py, [1, 2])?.as_any())?;
    globals.set_item("first", first)?;
    globals.set_item("second", second)?;
    rt::builtins::print(
        py,
        (
            rt::global(globals, "first")?,
            rt::global(globals, "second")?,
        ),
    )?;
    let [a, b] = rt::unpack(PyList::new(py, [1, 2, 3])?.as_any())?;
    globals.set_item("a", a)?;
    globals.set_item("b", b)?;
    Ok(())
}
