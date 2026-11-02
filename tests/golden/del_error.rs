use pyo3::prelude::*;
use steelix_runtime as rt;

// del_error.py:1
pub fn drop<'py>(m: &rt::Module<'py>, flag: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "drop", 1, |frame| {
        let mut value: Option<Bound<'py, PyAny>> = None;
        frame.line(2);
        if flag.is_truthy()? {
            value = Some(rt::int(py, 1));
        }
        frame.line(4);
        rt::del_local(&mut value, "value")?;
        Ok(rt::str(py, "dropped"))
    })
}

/// Executes the body of `del_error.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        rt::def!(m, drop(flag))?;
        frame.line(8);
        rt::builtins::print(py, (drop(m, &rt::bool(py, true))?,))?;
        frame.line(9);
        rt::builtins::print(py, (drop(m, &rt::bool(py, false))?,))?;
        Ok(())
    })
}
