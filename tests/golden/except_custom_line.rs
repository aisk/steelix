use pyo3::prelude::*;
use pyo3::types::PyDict;
use steelix_runtime as rt;

// except_custom_line.py:5
#[rt::frame(m, "handle", 5, frame)]
pub fn handle<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    match (|| -> PyResult<()> {
        if n.is_truthy()? {
            return Err(frame.raise(m.global("AppError")?.call1(("bad",))?));
        }
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if frame.matches(&err, || m.global("AppError"))? => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("caught", &e))?;
        }
        Err(err) => return Err(err),
    }
    PyDict::new(py).as_any().get_item("after")
}

pub mod app_error {
    use super::*;

    // except_custom_line.py:1
    /// The body of `class AppError`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

/// Executes the body of `except_custom_line.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "AppError",
        rt::class!(
            m,
            "AppError"(rt::builtin(py, "Exception")?),
            app_error::body,
            line = 1
        )?,
    )?;
    rt::def!(m, handle(n))?;
    handle(m, &rt::int(py, 1))?;
    Ok(())
}
