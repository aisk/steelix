//! Parts of `mixed_mode.py` kept as Python source and run by
//! `rt::exec_python!` in `exec_module` (steelix does not lower them yet):
//!
//! - line 17, `class Counter`: class definition
//! - line 35, `def evens`: yield
//! - line 48, `assign` statement: list comprehension
//! - line 54, `try` statement: try statement
//! - line 61, `if` statement: function definition outside module top level
//! - line 74, `def twice`: function redefinition
//! - line 87, `assign` statement: dict comprehension
//! - line 96, `def chunks`: yield from
//! - line 105, `expr` statement: list comprehension, assignment expression

use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyList;
use steelix_runtime as rt;

// mixed_mode.py:9
pub fn log<'py>(m: &rt::Module<'py>, event: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    m.global("events")?.call_method1("append", (event,))?;
    Ok(event.clone())
}

// mixed_mode.py:31
pub fn double<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    x.mul(2)
}

// mixed_mode.py:41
pub fn use_counter<'py>(
    m: &rt::Module<'py>,
    start: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    // Native code calling the Python class and generator through globals.
    let counter = m
        .global("Counter")?
        .call1((start,))?
        .call_method0("bump")?
        .call_method0("bump")?;
    counter
        .getattr("value")?
        .add(rt::builtin(m.py(), "sum")?.call1((m.global("evens")?.call1((5,))?,))?)
}

// mixed_mode.py:70
pub fn twice<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::str(m.py(), "native"))
}

// mixed_mode.py:81
pub fn set_mode<'py>(m: &rt::Module<'py>, mode: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    m.set_global("MODE", mode)?;
    Ok(rt::none(m.py()))
}

// mixed_mode.py:91
pub fn trace<'py>(m: &rt::Module<'py>, r#fn: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    log(
        m,
        &rt::fstring(
            m.py(),
            ("decorating ", rt::format(&r#fn.getattr("__name__")?, "")?),
        )?,
    )?;
    Ok(r#fn.clone())
}

/// Executes the body of `mixed_mode.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "Mixed mode: what steelix can not lower yet stays Python and runs in place.",
    )?;
    m.set_global("inspect", rt::import(m, "inspect")?)?;
    m.set_global("os", rt::import(m, "os")?)?;
    m.set_global("events", PyList::empty(py))?;
    rt::def!(m, log(event))?;
    log(m, &rt::str(py, "native statement before the class"))?;
    // Kept as Python: class definition at 17:1
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
    // Kept as Python: yield at 38:13
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
    log(m, &rt::str(py, "native statement after the class"))?;
    // Kept as Python: list comprehension at 48:11
    rt::exec_python!(
        m,
        line = 48,
        r#"
squares = [double(n) for n in range(4)]
"#
    )?;
    rt::builtins::print(py, (m.global("squares")?, use_counter(m, &rt::int(py, 3))?))?;
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
    rt::builtins::print(
        py,
        (m.global("inspect")?
            .getattr("getsource")?
            .call1((m.global("evens")?,))?
            .call_method0("splitlines")?
            .get_item(0)?,),
    )?;
    // Kept as Python: try statement at 54:1
    rt::exec_python!(
        m,
        line = 54,
        r#"
try:
    import json
except ImportError:
    json = None
"#
    )?;
    rt::builtins::print(
        py,
        (m.global("json")?
            .getattr("dumps")?
            .call1(([("kept", true)].into_py_dict(py)?,))?,),
    )?;
    m.set_global("FLAG", true)?;
    // Kept as Python: function definition outside module top level at 62:5
    rt::exec_python!(
        m,
        line = 61,
        r#"
if FLAG:
    def pick():
        return "first"
else:
    def pick():
        return "second"
"#
    )?;
    rt::builtins::print(py, (m.global("pick")?.call0()?,))?;
    rt::def!(m, twice())?;
    // Kept as Python: function redefinition at 74:1
    rt::exec_python!(
        m,
        line = 74,
        r#"
def twice():
    return "redefined, kept as Python"
"#
    )?;
    rt::builtins::print(py, (m.global("twice")?.call0()?,))?;
    rt::def!(m, set_mode(mode))?;
    set_mode(m, &rt::str(py, "set by native code"))?;
    // Kept as Python: dict comprehension at 87:9
    rt::exec_python!(
        m,
        line = 87,
        r#"
MODES = {k: MODE for k in "ab"}
"#
    )?;
    rt::builtins::print(py, (m.global("MODES")?,))?;
    rt::def!(m, trace(r#fn))?;
    // Kept as Python: yield from at 98:5
    rt::exec_python!(
        m,
        line = 96,
        r#"
@trace
def chunks(items):
    yield from items
"#
    )?;
    rt::builtins::print(
        py,
        (rt::builtin(py, "list")?
            .call1((m.global("chunks")?.call1((PyList::new(py, [1, 2])?,))?,))?,),
    )?;
    rt::builtins::print(py, (m.global("events")?,))?;
    // `:=` in a comprehension binds a global, here shadowing a builtin.
    // Kept as Python: list comprehension at 105:1, assignment expression at 105:3
    rt::exec_python!(
        m,
        line = 105,
        r#"
[(min := v) for v in (3, 7)]
"#
    )?;
    rt::builtins::print(py, (m.global("min")?,))?;
    Ok(())
}
