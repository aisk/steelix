use pyo3::prelude::*;
use steelix_runtime as rt;

// call_errors_uncaught.py:4
pub fn connect<'py>(
    m: &rt::Module<'py>,
    host: &Bound<'py, PyAny>,
    port: &Bound<'py, PyAny>,
    timeout: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(host.clone())
}

/// Executes the body of `call_errors_uncaught.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "An uncaught error from a call that can not be bound at transpile time.",
        )?;
        rt::def!(m, connect(host, port, *, timeout))?;
        frame.line(8);
        rt::builtins::print(
            py,
            (connect(
                m,
                &rt::str(py, "localhost"),
                &rt::int(py, 80),
                &rt::int(py, 1),
            )?,),
        )?;
        frame.line(9);
        rt::call!(m.global("connect")?, "localhost", timeout = 5)?;
        Ok(())
    })
}
