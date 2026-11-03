//! Parts of `frames_logging.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 20, `def kept_step`: type alias
//! - line 38, `def warn_kept`: type alias
//! - line 43, `def kept_calls_native`: type alias

use pyo3::prelude::*;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// frames_logging.py:15
pub fn step<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "step", 15, |_| {
        m.global("logging")?
            .method1("warning", ("native step %d", n))?;
        n.add(1)
    })
}

// frames_logging.py:26
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
    rt::frame!(m, "show_warning", 26, |_| {
        rt::builtins::print(
            py,
            (rt::fstring(
                py,
                (
                    rt::format(&category.attr("__name__")?, "")?,
                    " ",
                    rt::format(&filename.method1("rsplit", ("/", 1))?.get_item(-1)?, "")?,
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

// frames_logging.py:34
pub fn warn_native<'py>(
    m: &rt::Module<'py>,
    message: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "warn_native", 34, |_| {
        m.global("warnings")?.method1("warn", (message,))?;
        Ok(rt::none(m.py()))
    })
}

/// Executes the body of `frames_logging.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        // Native functions get a traceback frame only when an exception passes, so
        // logging and warnings attribute their records to the nearest Python frame
        // (design §12): logging's own frame, the frame of kept code calling the
        // native function, or none. `KNOWN_DIFFERENCES` in tests/test_e2e.py has it.
        m.set_global("logging", rt::import(m, "logging")?)?;
        m.set_global("sys", rt::import(m, "sys")?)?;
        m.set_global("warnings", rt::import(m, "warnings")?)?;
        rt::call!(
            m.global("logging")?.attr("basicConfig")?,
            stream = m.global("sys")?.attr("stdout")?,
            format = "%(levelname)s %(filename)s:%(lineno)d %(funcName)s: %(message)s"
        )?;
        rt::def!(m, step(n))?;
        // frames_logging.py:20, kept as Python: type alias at 21:5
        rt::exec_python!(
            m,
            line = 20,
            r#"
def kept_step(n):
    type Step = int  # kept as Python
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
        let show_warning2 = m.global("show_warning")?;
        m.global("warnings")?
            .set_attr("showwarning", show_warning2)?;
        m.global("warnings")?.method1("simplefilter", ("always",))?;
        rt::def!(m, warn_native(message))?;
        // frames_logging.py:38, kept as Python: type alias at 39:5
        rt::exec_python!(
            m,
            line = 38,
            r#"
def warn_kept():
    type Kept = int  # kept as Python
    warnings.warn("from kept code")
"#
        )?;
        // frames_logging.py:43, kept as Python: type alias at 44:5
        rt::exec_python!(
            m,
            line = 43,
            r#"
def kept_calls_native():
    type Kept = int  # kept as Python
    warn_native("from native code, called by kept code")
"#
        )?;
        step(m, &rt::int(py, 1))?;
        m.global("kept_step")?.call1((2,))?;
        warn_native(m, &rt::str(py, "from native code"))?;
        m.global("warn_kept")?.call0()?;
        m.global("kept_calls_native")?.call0()?;
        Ok(())
    })
}
