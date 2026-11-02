use pyo3::prelude::*;
use steelix_runtime as rt;

// call_errors_uncaught.py:4
pub fn connect<'py>(
    py: Python<'py>,
    host: &Bound<'py, PyAny>,
    port: &Bound<'py, PyAny>,
    timeout: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(host.clone())
}

/// Executes the body of `call_errors_uncaught.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    globals.set_item(
        "__doc__",
        "An uncaught error from a call that can not be bound at transpile time.",
    )?;
    rt::def!(globals, connect(host, port, *, timeout))?;
    rt::builtins::print(
        py,
        (connect(
            py,
            &rt::str(py, "localhost"),
            &rt::int(py, 80),
            &rt::int(py, 1),
        )?,),
    )?;
    rt::call!(rt::global(globals, "connect")?, "localhost", timeout = 5)?;
    Ok(())
}
