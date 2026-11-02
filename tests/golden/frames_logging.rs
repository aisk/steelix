//! Parts of `frames_logging.py` kept as Python source and run by
//! `rt::exec_python!` in `exec_module` (steelix does not lower them yet):
//!
//! - line 20, `def kept_step`: try statement
//! - line 40, `def warn_kept`: try statement
//! - line 47, `def kept_calls_native`: try statement

use pyo3::prelude::*;
use steelix_runtime as rt;

// frames_logging.py:15
pub fn step<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    m.global("logging")?
        .call_method1("warning", ("native step %d", n))?;
    n.add(1)
}

// frames_logging.py:28
pub fn show_warning<'py>(
    m: &rt::Module<'py>,
    message: &Bound<'py, PyAny>,
    category: &Bound<'py, PyAny>,
    filename: &Bound<'py, PyAny>,
    lineno: &Bound<'py, PyAny>,
    file: &Bound<'py, PyAny>,
    line: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::builtins::print(
        py,
        (rt::fstring(
            py,
            (
                rt::format(&category.getattr("__name__")?, "")?,
                " ",
                rt::format(
                    &filename.call_method1("rsplit", ("/", 1))?.get_item(-1)?,
                    "",
                )?,
                ":",
                rt::format(lineno, "")?,
                ": ",
                rt::format(message, "")?,
            ),
        )?,),
    )?;
    Ok(rt::none(py))
}

// frames_logging.py:36
pub fn warn_native<'py>(
    m: &rt::Module<'py>,
    message: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    m.global("warnings")?.call_method1("warn", (message,))?;
    Ok(rt::none(m.py()))
}

/// Executes the body of `frames_logging.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    // Native functions have no Python frame, so logging and warnings attribute
    // their records to the nearest Python frame instead (design §12): logging's
    // own frame, the frame of kept code calling the native function, or none.
    // `KNOWN_DIFFERENCES` in tests/test_e2e.py spells out the current output.
    m.set_global("logging", rt::import(m, "logging")?)?;
    m.set_global("sys", rt::import(m, "sys")?)?;
    m.set_global("warnings", rt::import(m, "warnings")?)?;
    rt::call!(
        m.global("logging")?.getattr("basicConfig")?,
        stream = m.global("sys")?.getattr("stdout")?,
        format = "%(levelname)s %(filename)s:%(lineno)d %(funcName)s: %(message)s"
    )?;
    rt::def!(m, step(n))?;
    // frames_logging.py:20, kept as Python: try statement at 21:5
    rt::exec_python!(
        m,
        line = 20,
        r#"
def kept_step(n):
    try:
        logging.warning("kept step %d", n)
    finally:
        pass
    return n + 1
"#
    )?;
    rt::def!(
        m,
        show_warning(
            message,
            category,
            filename,
            lineno,
            file = py.None(),
            line = py.None()
        )
    )?;
    let show_warning2 = m.global("show_warning")?;
    m.global("warnings")?
        .setattr("showwarning", &show_warning2)?;
    m.global("warnings")?
        .call_method1("simplefilter", ("always",))?;
    rt::def!(m, warn_native(message))?;
    // frames_logging.py:40, kept as Python: try statement at 41:5
    rt::exec_python!(
        m,
        line = 40,
        r#"
def warn_kept():
    try:
        warnings.warn("from kept code")
    finally:
        pass
"#
    )?;
    // frames_logging.py:47, kept as Python: try statement at 48:5
    rt::exec_python!(
        m,
        line = 47,
        r#"
def kept_calls_native():
    try:
        warn_native("from native code, called by kept code")
    finally:
        pass
"#
    )?;
    step(m, &rt::int(py, 1))?;
    m.global("kept_step")?.call1((2,))?;
    warn_native(m, &rt::str(py, "from native code"))?;
    m.global("warn_kept")?.call0()?;
    m.global("kept_calls_native")?.call0()?;
    Ok(())
}
