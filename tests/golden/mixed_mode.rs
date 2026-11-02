//! Parts of `mixed_mode.py` kept as Python source and run by
//! `rt::exec_python!` in `exec_module` (steelix does not lower them yet):
//!
//! - line 17, `class Counter`: class definition
//! - line 35, `def evens`: yield
//! - line 48, `assign` statement: list comprehension
//! - line 62, `def pick`: function definition outside module top level
//! - line 65, `def pick`: function definition outside module top level
//! - line 74, `def twice`: function redefinition
//! - line 87, `assign` statement: dict comprehension
//! - line 96, `def chunks`: yield from
//! - line 105, `expr` statement: list comprehension, assignment expression

use pyo3::exceptions::PyImportError;
use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyList;
use steelix_runtime as rt;

// mixed_mode.py:9
pub fn log<'py>(m: &rt::Module<'py>, event: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "log", 9, |frame| {
        frame.line(10);
        m.global("events")?.call_method1("append", (event,))?;
        Ok(event.clone())
    })
}

// mixed_mode.py:31
pub fn double<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "double", 31, |frame| {
        frame.line(32);
        x.mul(2)
    })
}

// mixed_mode.py:41
pub fn use_counter<'py>(
    m: &rt::Module<'py>,
    start: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "use_counter", 41, |frame| {
        // Native code calling the Python class and generator through globals.
        frame.line(43);
        let counter = m
            .global("Counter")?
            .call1((start,))?
            .call_method0("bump")?
            .call_method0("bump")?;
        frame.line(44);
        counter
            .getattr("value")?
            .add(rt::builtin(m.py(), "sum")?.call1((m.global("evens")?.call1((5,))?,))?)
    })
}

// mixed_mode.py:70
pub fn twice<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::str(m.py(), "native"))
}

// mixed_mode.py:81
pub fn set_mode<'py>(m: &rt::Module<'py>, mode: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "set_mode", 81, |frame| {
        frame.line(83);
        m.set_global("MODE", mode)?;
        Ok(rt::none(m.py()))
    })
}

// mixed_mode.py:91
pub fn trace<'py>(m: &rt::Module<'py>, r#fn: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "trace", 91, |frame| {
        frame.line(92);
        log(
            m,
            &rt::fstring(
                m.py(),
                ("decorating ", rt::format(&r#fn.getattr("__name__")?, "")?),
            )?,
        )?;
        Ok(r#fn.clone())
    })
}

/// Executes the body of `mixed_mode.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "Mixed mode: what steelix can not lower yet stays Python and runs in place.",
        )?;
        frame.line(3);
        m.set_global("inspect", rt::import(m, "inspect")?)?;
        frame.line(4);
        m.set_global("os", rt::import(m, "os")?)?;
        frame.line(6);
        m.set_global("events", PyList::empty(py))?;
        rt::def!(m, log(event))?;
        frame.line(14);
        log(m, &rt::str(py, "native statement before the class"))?;
        // mixed_mode.py:17, kept as Python: class definition at 17:1
        rt::exec_python!(
            m,
            line = 17,
            r#"
class Counter:
    """Kept as Python: classes are not lowered yet."""

    instances = log("class body")

    def __init__(self, start):
        self.value = start

    def bump(self):
        # Python calling a native function.
        self.value = double(self.value)
        return self
"#
        )?;
        rt::def!(m, double(x))?;
        // mixed_mode.py:35, kept as Python: yield at 38:13
        rt::exec_python!(
            m,
            line = 35,
            r#"
def evens(limit):
    for n in range(limit):
        if n % 2 == 0:
            yield n
"#
        )?;
        rt::def!(m, use_counter(start))?;
        frame.line(47);
        log(m, &rt::str(py, "native statement after the class"))?;
        // mixed_mode.py:48, kept as Python: list comprehension at 48:11
        rt::exec_python!(
            m,
            line = 48,
            r#"
squares = [double(n) for n in range(4)]
"#
        )?;
        frame.line(49);
        rt::builtins::print(py, (m.global("squares")?, use_counter(m, &rt::int(py, 3))?))?;
        frame.line(50);
        rt::builtins::print(
            py,
            (
                m.global("Counter")?.getattr("__module__")?,
                m.global("Counter")?
                    .getattr("bump")?
                    .getattr("__qualname__")?,
                m.global("evens")?.getattr("__qualname__")?,
                m.global("evens")?.getattr("__module__")?,
            ),
        )?;
        frame.line(51);
        rt::builtins::print(
            py,
            (
                m.global("os")?
                    .getattr("path")?
                    .getattr("basename")?
                    .call1((m
                        .global("evens")?
                        .getattr("__code__")?
                        .getattr("co_filename")?,))?,
                m.global("evens")?
                    .getattr("__code__")?
                    .getattr("co_firstlineno")?,
            ),
        )?;
        frame.line(52);
        rt::builtins::print(
            py,
            (m.global("inspect")?
                .getattr("getsource")?
                .call1((m.global("evens")?,))?
                .call_method0("splitlines")?
                .get_item(0)?,),
        )?;
        match (|| -> PyResult<()> {
            frame.line(55);
            m.set_global("json", rt::import(m, "json")?)?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyImportError>(py) => {
                let _handling = rt::Handling::new(&frame.catch(err));
                frame.line(57);
                m.set_global("json", py.None())?;
            }
            Err(err) => return Err(err),
        }
        frame.line(58);
        rt::builtins::print(
            py,
            (m.global("json")?
                .getattr("dumps")?
                .call1(([("kept", true)].into_py_dict(py)?,))?,),
        )?;
        frame.line(60);
        m.set_global("FLAG", true)?;
        frame.line(61);
        if m.global("FLAG")?.is_truthy()? {
            // mixed_mode.py:62, kept as Python: function definition outside module top level at 62:5
            rt::exec_python!(
                m,
                line = 62,
                r#"
    def pick():
        return "first"
"#
            )?;
        } else {
            // mixed_mode.py:65, kept as Python: function definition outside module top level at 65:5
            rt::exec_python!(
                m,
                line = 65,
                r#"
    def pick():
        return "second"
"#
            )?;
        }
        frame.line(67);
        rt::builtins::print(py, (m.global("pick")?.call0()?,))?;
        rt::def!(m, twice())?;
        // mixed_mode.py:74, kept as Python: function redefinition at 74:1
        rt::exec_python!(
            m,
            line = 74,
            r#"
def twice():
    return "redefined, kept as Python"
"#
        )?;
        frame.line(78);
        rt::builtins::print(py, (m.global("twice")?.call0()?,))?;
        rt::def!(m, set_mode(mode))?;
        frame.line(86);
        set_mode(m, &rt::str(py, "set by native code"))?;
        // mixed_mode.py:87, kept as Python: dict comprehension at 87:9
        rt::exec_python!(
            m,
            line = 87,
            r#"
MODES = {k: MODE for k in "ab"}
"#
        )?;
        frame.line(88);
        rt::builtins::print(py, (m.global("MODES")?,))?;
        rt::def!(m, trace(r#fn))?;
        // mixed_mode.py:96, kept as Python: yield from at 98:5
        rt::exec_python!(
            m,
            line = 96,
            r#"
@trace
def chunks(items):
    yield from items
"#
        )?;
        frame.line(101);
        rt::builtins::print(
            py,
            (rt::builtin(py, "list")?
                .call1((m.global("chunks")?.call1((PyList::new(py, [1, 2])?,))?,))?,),
        )?;
        frame.line(102);
        rt::builtins::print(py, (m.global("events")?,))?;
        // `:=` in a comprehension binds a global, here shadowing a builtin.
        // mixed_mode.py:105, kept as Python: list comprehension at 105:1, assignment expression at 105:3
        rt::exec_python!(
            m,
            line = 105,
            r#"
[(min := v) for v in (3, 7)]
"#
        )?;
        frame.line(106);
        rt::builtins::print(py, (m.global("min")?,))?;
        Ok(())
    })
}
