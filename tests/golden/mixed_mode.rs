//! Parts of `mixed_mode.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 35, `def evens`: yield
//! - line 48, `assign` statement: generator expression with several for clauses
//! - line 74, `def twice`: function redefinition
//! - line 96, `def chunks`: yield from

use pyo3::exceptions::PyImportError;
use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// mixed_mode.py:9
pub fn log<'py>(m: &rt::Module<'py>, event: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "log", 9, |_| {
        m.global("events")?.method1("append", (event,))?;
        Ok(event.clone())
    })
}

// mixed_mode.py:31
pub fn double<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "double", 31, |_| { x.mul(2) })
}

// mixed_mode.py:41
pub fn use_counter<'py>(
    m: &rt::Module<'py>,
    start: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "use_counter", 41, |_| {
        // Native code calling the class and the Python generator through globals.
        let counter = m
            .global("Counter")?
            .call1((start,))?
            .method0("bump")?
            .method0("bump")?;
        counter
            .attr("value")?
            .add(rt::builtin(m.py(), "sum")?.call1((m.global("evens")?.call1((5,))?,))?)
    })
}

// mixed_mode.py:70
pub fn twice<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::str(m.py(), "native"))
}

// mixed_mode.py:81
pub fn set_mode<'py>(m: &rt::Module<'py>, mode: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "set_mode", 81, |_| {
        m.set_global("MODE", mode)?;
        Ok(rt::none(m.py()))
    })
}

// mixed_mode.py:91
pub fn trace<'py>(m: &rt::Module<'py>, r#fn: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "trace", 91, |_| {
        log(
            m,
            &rt::fstring(
                m.py(),
                ("decorating ", rt::format(&r#fn.attr("__name__")?, "")?),
            )?,
        )?;
        Ok(r#fn.clone())
    })
}

/// A native class whose methods call native functions.
pub mod counter {
    use super::*;

    // mixed_mode.py:17
    /// The body of `class Counter`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Counter", 17, |_| {
            ns.set(
                "__doc__",
                "A native class whose methods call native functions.",
            )?;
            ns.set("instances", ns.get("log")?.call1(("class body",))?)?;
            rt::def!(ns, __init__(slf as "self", start))?;
            rt::def!(ns, bump(slf as "self"))?;
            Ok(())
        })
    }

    // mixed_mode.py:22
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        start: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Counter.__init__", 22, |_| {
            slf.set_attr("value", start)?;
            Ok(rt::none(m.py()))
        })
    }

    // mixed_mode.py:25
    pub fn bump<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Counter.bump", 25, |_| {
            // A method calling a native function.
            slf.set_attr("value", double(m, &slf.attr("value")?)?)?;
            Ok(slf.clone())
        })
    }
}

/// Executes the body of `mixed_mode.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "Mixed mode: what steelix can not lower yet stays Python and runs in place.",
        )?;
        m.set_global("inspect", rt::import(m, "inspect")?)?;
        m.set_global("os", rt::import(m, "os")?)?;
        m.set_global("events", PyList::empty(py))?;
        rt::def!(m, log(event))?;
        log(m, &rt::str(py, "native statement before the class"))?;
        m.set_global(
            "Counter",
            rt::class!(
                m,
                "Counter"(),
                counter::body,
                line = 17,
                attributes = ["value"]
            )?,
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
        log(m, &rt::str(py, "native statement after the class"))?;
        // mixed_mode.py:48, kept as Python: generator expression with several for clauses at 48:15
        rt::exec_python!(
            m,
            line = 48,
            r#"
squares = list(s for s in map(double, range(4)) for _ in "1")
"#
        )?;
        rt::builtins::print(py, (m.global("squares")?, use_counter(m, &rt::int(py, 3))?))?;
        rt::builtins::print(
            py,
            (
                m.global("Counter")?.attr("__module__")?,
                m.global("Counter")?.attr("bump")?.attr("__qualname__")?,
                m.global("evens")?.attr("__qualname__")?,
                m.global("evens")?.attr("__module__")?,
            ),
        )?;
        rt::builtins::print(
            py,
            (
                m.global("os")?
                    .attr("path")?
                    .attr("basename")?
                    .call1((m.global("evens")?.attr("__code__")?.attr("co_filename")?,))?,
                m.global("evens")?
                    .attr("__code__")?
                    .attr("co_firstlineno")?,
            ),
        )?;
        rt::builtins::print(
            py,
            (m.global("inspect")?
                .attr("getsource")?
                .call1((m.global("evens")?,))?
                .method0("splitlines")?
                .get_item(0)?,),
        )?;
        match (|| -> PyResult<()> {
            m.set_global("json", rt::import(m, "json")?)?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyImportError>(py) => {
                let _handling = rt::Handling::new(&frame.catch(err));
                m.set_global("json", py.None())?;
            }
            Err(err) => return Err(err),
        }
        rt::builtins::print(
            py,
            (m.global("json")?
                .attr("dumps")?
                .call1(([("kept", true)].into_py_dict(py)?,))?,),
        )?;
        m.set_global("FLAG", true)?;
        if m.global("FLAG")?.is_truthy()? {
            // mixed_mode.py:62
            fn pick<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
                Ok(rt::str(m.py(), "first"))
            }
            rt::def!(m, pick())?;
        } else {
            // mixed_mode.py:65
            fn pick_<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
                Ok(rt::str(m.py(), "second"))
            }
            rt::def!(m, pick_ as "pick"())?;
        }
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
        rt::builtins::print(py, (m.global("twice")?.call0()?,))?;
        rt::def!(m, set_mode(mode))?;
        set_mode(m, &rt::str(py, "set by native code"))?;
        m.set_global(
            "MODES",
            rt::dict!(
                py,
                **[("a", &m.global("MODE")?)].into_py_dict(py)?,
                ("b", m.global("MODE")?)
            )?,
        )?;
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
        rt::builtins::print(
            py,
            (rt::builtin(py, "list")?
                .call1((m.global("chunks")?.call1((PyList::new(py, [1, 2])?,))?,))?,),
        )?;
        rt::builtins::print(py, (m.global("events")?,))?;
        // `:=` in a comprehension binds a global, here shadowing a builtin.
        {
            let list = PyList::empty(py);
            for v in PyTuple::new(py, [3, 7])?.try_iter()? {
                let v = v?;
                m.set_global("min", &v)?;
                list.append(&v)?;
                py.check_signals()?;
            }
        }
        rt::builtins::print(py, (m.global("min")?,))?;
        Ok(())
    })
}
