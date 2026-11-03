//! Parts of `frames_logging.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 20, `def kept_step`: match statement
//! - line 39, `def warn_kept`: match statement
//! - line 45, `def kept_calls_native`: match statement

use pyo3::prelude::*;
use steelix_runtime as rt;

// frames_logging.py:15
pub fn step<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "step", 15, |frame| {
        frame.line(16);
        m.global("logging")?
            .call_method1("warning", ("native step %d", n))?;
        frame.line(17);
        n.add(1)
    })
}

// frames_logging.py:27
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
    rt::frame!(m, "show_warning", 27, |frame| {
        frame.line(28);
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
    })
}

// frames_logging.py:35
pub fn warn_native<'py>(
    m: &rt::Module<'py>,
    message: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "warn_native", 35, |frame| {
        frame.line(36);
        m.global("warnings")?.call_method1("warn", (message,))?;
        Ok(rt::none(m.py()))
    })
}

/// Executes the body of `frames_logging.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        // Native functions get a traceback frame only when an exception passes, so
        // logging and warnings attribute their records to the nearest Python frame
        // (design §12): logging's own frame, the frame of kept code calling the
        // native function, or none. `KNOWN_DIFFERENCES` in tests/test_e2e.py has it.
        frame.line(5);
        m.set_global("logging", rt::import(m, "logging")?)?;
        frame.line(6);
        m.set_global("sys", rt::import(m, "sys")?)?;
        frame.line(7);
        m.set_global("warnings", rt::import(m, "warnings")?)?;
        frame.line(9);
        rt::call!(
            m.global("logging")?.getattr("basicConfig")?,
            stream = m.global("sys")?.getattr("stdout")?,
            format = "%(levelname)s %(filename)s:%(lineno)d %(funcName)s: %(message)s"
        )?;
        rt::def!(m, step(n))?;
        // frames_logging.py:20, kept as Python: match statement at 21:5
        rt::exec_python!(
            m,
            line = 20,
            r#"
def kept_step(n):
    match n:  # kept as Python
        case _:
            logging.warning("kept step %d", n)
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
        frame.line(31);
        let show_warning2 = m.global("show_warning")?;
        m.global("warnings")?
            .setattr("showwarning", show_warning2)?;
        frame.line(32);
        m.global("warnings")?
            .call_method1("simplefilter", ("always",))?;
        rt::def!(m, warn_native(message))?;
        // frames_logging.py:39, kept as Python: match statement at 40:5
        rt::exec_python!(
            m,
            line = 39,
            r#"
def warn_kept():
    match 0:  # kept as Python
        case _:
            warnings.warn("from kept code")
"#
        )?;
        // frames_logging.py:45, kept as Python: match statement at 46:5
        rt::exec_python!(
            m,
            line = 45,
            r#"
def kept_calls_native():
    match 0:  # kept as Python
        case _:
            warn_native("from native code, called by kept code")
"#
        )?;
        frame.line(51);
        step(m, &rt::int(py, 1))?;
        frame.line(52);
        m.global("kept_step")?.call1((2,))?;
        frame.line(53);
        warn_native(m, &rt::str(py, "from native code"))?;
        frame.line(54);
        m.global("warn_kept")?.call0()?;
        frame.line(55);
        m.global("kept_calls_native")?.call0()?;
        Ok(())
    })
}
