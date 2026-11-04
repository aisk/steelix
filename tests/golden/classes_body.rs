//! Parts of `classes_body.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 88, `def failing_body`: class definition in a function

use pyo3::exceptions::PyAssertionError;
use pyo3::exceptions::PyKeyError;
use pyo3::exceptions::PyNameError;
use pyo3::exceptions::PyValueError;
use pyo3::exceptions::PyZeroDivisionError;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// classes_body.py:7
pub fn show_tb<'py>(m: &rt::Module<'py>, err: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "show_tb", 7, |_| {
        let list = PyList::empty(py);
        for f in rt::iter(
            &m.global("traceback")?
                .attr("extract_tb")?
                .call1((err.attr("__traceback__")?,))?,
        )? {
            let f = f?;
            list.append(PyTuple::new(py, [&f.attr("name")?, &f.attr("lineno")?])?)?;
        }
        Ok(list.into_any())
    })
}

pub mod body_ {
    use super::*;

    // classes_body.py:11
    /// The body of `class Body`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let py = m.py();
        rt::frame!(ns, "Body", 11, |frame| {
            ns.set("paths", rt::import_module(m, "os.path")?)?;
            let math = rt::import_names(m, "math", &["sqrt"])?;
            ns.set("root", rt::import_from(&math, "sqrt")?)?;
            ns.set("total", 0)?;
            for item in rt::iter(&ns.get("range")?.call1((4,))?)? {
                ns.set("i", item?)?;
                ns.set("total", rt::iadd(&ns.get("total")?, ns.get("i")?)?)?;
            }
            while ns.get("total")?.gt(10)? {
                ns.set("total", rt::isub(&ns.get("total")?, 1)?)?;
                py.check_signals()?;
            }
            ns.set("names", PyList::empty(py))?;
            if ns
                .get("sys")?
                .attr("version_info")?
                .ge(PyTuple::new(py, [3, 0])?)?
            {
                rt::def!(ns, version(slf as "self"))?;
            } else {
                rt::def!(ns, version_ as "version"(slf as "self"))?;
            }
            let result = (|| -> PyResult<()> {
                match PyDict::new(py).as_any().get_item("key") {
                    Ok(value) => {
                        ns.set("missing", &value)?;
                    }
                    Err(err) if err.is_instance_of::<PyKeyError>(py) => {
                        let exception = frame.catch(err);
                        ns.set("err", &exception)?;
                        ns.set("caught", ns.get("repr")?.call1((ns.get("err")?,))?)?;
                        ns.unbind("err")?;
                    }
                    Err(err) => return Err(err),
                }
                Ok(())
            })();
            let finally = frame.finally(result);
            ns.set("cleanup", true)?;
            finally.resume()?;
            frame.with(&ns.get("open")?.call1((ns.get("__file__")?,))?, |value| {
                ns.set("handle", value)?;
                ns.set(
                    "first",
                    ns.get("handle")?.method0("readline")?.method0("strip")?,
                )?;
                Ok(())
            })?;
            ns.set("temp", 1)?;
            ns.del("temp")?;
            match (|| -> PyResult<()> {
                ns.del("temp")?;
                Ok(())
            })() {
                Ok(()) => {}
                Err(err) if err.is_instance_of::<PyNameError>(py) => {
                    let exception = frame.catch(err);
                    ns.set("err", &exception)?;
                    ns.set("name_error", ns.get("str")?.call1((ns.get("err")?,))?)?;
                    ns.unbind("err")?;
                }
                Err(err) => return Err(err),
            }
            ns.set("x", 1)?;
            let list = PyList::empty(py);
            for x in rt::iter(&ns.get("range")?.call1((3,))?)? {
                let x = x?;
                list.append(x.mul(&x)?)?;
            }
            ns.set("squares", list)?;
            ns.set("y", 2)?;
            // classes_body.py:46
            fn lambda<'py>(
                m: &rt::Module<'py>,
                slf: &Bound<'py, PyAny>,
            ) -> PyResult<Bound<'py, PyAny>> {
                rt::frame!(m, "Body.<lambda>", 46, |_| {
                    rt::str(m.py(), "lambda ").add(slf.get_type().attr("__name__")?)
                })
            }
            ns.set(
                "label",
                rt::function!(m, lambda(slf as "self"), qualname = "Body.<lambda>")?,
            )?;
            ns.set("shadow", ns.get("len")?)?;
            ns.set("len", 5)?;
            ns.set(
                "used",
                ns.get("shadow")?.call1((PyList::new(py, [1, 2])?,))?,
            )?;
            ns.set("builtin", ns.get("print")?.attr("__name__")?)?;
            if !ns.get("total")?.eq(6)? {
                return Err(frame.raise(PyAssertionError::new_err("total")));
            }
            Ok(())
        })
    }

    // classes_body.py:23
    pub fn version<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::str(m.py(), "py3"))
    }

    // classes_body.py:28
    pub fn version_<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::str(m.py(), "py2"))
    }
}

pub mod generic {
    use super::*;

    // classes_body.py:61
    /// The body of `class Generic`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Generic", 61, |_| {
            rt::def!(ns, __class_getitem__(cls, item))?;
            rt::def!(ns, __init_subclass__(cls, **kwargs))?;
            Ok(())
        })
    }

    // classes_body.py:62
    pub fn __class_getitem__<'py>(
        m: &rt::Module<'py>,
        cls: &Bound<'py, PyAny>,
        item: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Generic.__class_getitem__", 62, |_| {
            rt::fstring(
                m.py(),
                (
                    rt::format(&cls.attr("__name__")?, "")?,
                    "[",
                    rt::format(&item.attr("__name__")?, "")?,
                    "]",
                ),
            )
        })
    }

    // classes_body.py:65
    pub fn __init_subclass__<'py>(
        m: &rt::Module<'py>,
        cls: &Bound<'py, PyAny>,
        kwargs: &Bound<'py, PyDict>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Generic.__init_subclass__", 65, |_| {
            cls.set_attr("created", true)?;
            Ok(rt::none(m.py()))
        })
    }
}

pub mod sub {
    use super::*;

    // classes_body.py:69
    /// The body of `class Sub`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

pub mod new {
    use super::*;

    // classes_body.py:76
    /// The body of `class New`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "New", 76, |_| {
            let __class__ = ns.class_cell();
            rt::def!(ns, __new__[&__class__](cls, *args))?;
            rt::def!(ns, __init__(slf as "self", value))?;
            Ok(())
        })
    }

    // classes_body.py:77
    pub fn __new__<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        cls: &Bound<'py, PyAny>,
        args: &Bound<'py, PyTuple>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "New.__new__", 77, |_| {
            rt::builtins::print(m.py(), ("new", cls.attr("__name__")?, args))?;
            rt::super_(__class__, cls)?.method1("__new__", (cls,))
        })
    }

    // classes_body.py:81
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        value: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "New.__init__", 81, |_| {
            slf.set_attr("value", value)?;
            Ok(rt::none(m.py()))
        })
    }
}

pub mod failing {
    use super::*;

    // classes_body.py:95
    /// The body of `class Failing`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Failing", 95, |_| {
            ns.set("before", 1)?;
            ns.set("value", rt::int(m.py(), 1).div(0)?)?;
            Ok(())
        })
    }
}

pub mod failing_kept {
    use super::*;

    // classes_body.py:104
    /// The body of `class FailingKept`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let py = m.py();
        rt::frame!(ns, "FailingKept", 104, |_| {
            let list = PyList::empty(py);
            for underscore in rt::iter(&rt::str(py, "x"))? {
                let underscore = underscore?;
                list.append(rt::int(py, 1).div(0)?)?;
            }
            ns.set("values", list)?;
            Ok(())
        })
    }
}

pub mod annotated {
    use super::*;

    // classes_body.py:111
    /// The body of `class Annotated`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Annotated", 111, |_| {
            ns.set("alias", ns.get("int")?)?;
            rt::def!(ns, method(slf as "self", x: "alias") -> "'Annotated'")?;
            rt::def!(ns, kept(slf as "self", x: "alias") -> "list[alias]")?;
            Ok(())
        })
    }

    // classes_body.py:114
    pub fn method<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        x: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(x.clone())
    }

    // classes_body.py:117
    pub fn kept<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        x: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Annotated.kept", 117, |_| {
            let list = PyList::empty(py);
            for underscore in rt::iter(&rt::str(py, "x"))? {
                let underscore = underscore?;
                list.append(x)?;
            }
            Ok(list.into_any())
        })
    }
}

pub mod uses {
    use super::*;

    // classes_body.py:124
    /// The body of `class Uses`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Uses", 124, |_| {
            let __class__ = ns.class_cell();
            rt::def!(ns, __getattr__(slf as "self", name))?;
            rt::def!(ns, __setattr__[&__class__](slf as "self", name, value))?;
            rt::def!(ns, __contains__(slf as "self", item))?;
            rt::def!(ns, __lt__(slf as "self", other))?;
            rt::def!(ns, __enter__(slf as "self"))?;
            rt::def!(ns, __exit__(slf as "self", *exc))?;
            Ok(())
        })
    }

    // classes_body.py:125
    pub fn __getattr__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Uses.__getattr__", 125, |_| { name.method0("upper") })
    }

    // classes_body.py:128
    pub fn __setattr__<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
        value: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Uses.__setattr__", 128, |_| {
            rt::super_(__class__, slf)?
                .attr("__setattr__")?
                .call1((name, value.mul(2)?))?;
            Ok(rt::none(m.py()))
        })
    }

    // classes_body.py:131
    pub fn __contains__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        item: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Uses.__contains__", 131, |_| {
            item.rich_compare("x", CompareOp::Eq)
        })
    }

    // classes_body.py:134
    pub fn __lt__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        other: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::bool(m.py(), true))
    }

    // classes_body.py:137
    pub fn __enter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(slf.clone())
    }

    // classes_body.py:140
    pub fn __exit__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        exc: &Bound<'py, PyTuple>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Uses.__exit__", 140, |_| {
            rt::builtins::print(py, ("exit", exc.as_any().get_item(0)?))?;
            Ok(rt::bool(py, true))
        })
    }
}

/// Executes the body of `classes_body.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "Statements in a class body run in the class namespace.",
        )?;
        m.set_global("sys", rt::import(m, "sys")?)?;
        m.set_global("traceback", rt::import(m, "traceback")?)?;
        rt::def!(m, show_tb(err))?;
        m.set_global("Body", rt::class!(m, "Body"(), body_::body, line = 11)?)?;
        m.set_global("b", m.global("Body")?.call0()?)?;
        rt::builtins::print(
            py,
            (
                m.global("Body")?.attr("total")?,
                m.global("Body")?.attr("i")?,
                m.global("Body")?
                    .attr("version")?
                    .call1((m.global("b")?,))?,
                m.global("Body")?.attr("caught")?,
                m.global("Body")?.attr("cleanup")?,
                m.global("Body")?.attr("first")?,
            ),
        )?;
        rt::builtins::print(
            py,
            (
                m.global("Body")?.attr("name_error")?,
                m.global("Body")?.attr("squares")?,
                m.global("Body")?.attr("x")?,
                m.global("Body")?.attr("y")?,
                m.global("b")?.method0("label")?,
                m.global("Body")?.attr("used")?,
                m.global("Body")?.attr("len")?,
            ),
        )?;
        rt::builtins::print(
            py,
            (
                m.global("Body")?
                    .attr("paths")?
                    .method1("join", ("a", "b"))?,
                m.global("Body")?.method1("root", (16,))?,
                m.global("Body")?.attr("builtin")?,
                rt::builtin(py, "hasattr")?.call1((m.global("Body")?, "err"))?,
            ),
        )?;
        rt::builtins::print(
            py,
            (
                m.global("Body")?.attr("label")?.attr("__qualname__")?,
                m.global("Body")?.attr("__static_attributes__")?,
            ),
        )?;
        m.set_global(
            "Generic",
            rt::class!(m, "Generic"(), generic::body, line = 61)?,
        )?;
        m.set_global(
            "Sub",
            rt::class!(m, "Sub"(m.global("Generic")?), sub::body, line = 69)?,
        )?;
        rt::builtins::print(
            py,
            (
                m.global("Generic")?.get_item(rt::builtin(py, "int")?)?,
                m.global("Sub")?.attr("created")?,
                m.global("Generic")?
                    .attr("__dict__")?
                    .get_item("__class_getitem__")?
                    .get_type()
                    .attr("__name__")?,
            ),
        )?;
        m.set_global(
            "New",
            rt::class!(m, "New"(), new::body, line = 76, attributes = ["value"])?,
        )?;
        rt::builtins::print(
            py,
            (
                m.global("New")?.call1((3,))?.attr("value")?,
                m.global("New")?
                    .attr("__dict__")?
                    .get_item("__new__")?
                    .get_type()
                    .attr("__name__")?,
            ),
        )?;
        // classes_body.py:88, kept as Python: class definition in a function at 89:5
        rt::exec_python!(
            m,
            line = 88,
            r#"
def failing_body():
    class Failing:
        value = 1 / 0
"#
        )?;
        match (|| -> PyResult<()> {
            m.set_global(
                "Failing",
                rt::class!(m, "Failing"(), failing::body, line = 95)?,
            )?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyZeroDivisionError>(py) => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                rt::builtins::print(py, ("ZeroDivisionError", show_tb(m, &m.global("err")?)?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        match (|| -> PyResult<()> {
            m.set_global(
                "FailingKept",
                rt::class!(m, "FailingKept"(), failing_kept::body, line = 104)?,
            )?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyZeroDivisionError>(py) => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                rt::builtins::print(py, ("ZeroDivisionError", show_tb(m, &m.global("err")?)?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        m.set_global(
            "Annotated",
            rt::class!(m, "Annotated"(), annotated::body, line = 111)?,
        )?;
        rt::builtins::print(
            py,
            (
                m.global("Annotated")?
                    .attr("method")?
                    .attr("__annotations__")?,
                m.global("Annotated")?
                    .attr("kept")?
                    .attr("__annotations__")?,
            ),
        )?;
        m.set_global("Uses", rt::class!(m, "Uses"(), uses::body, line = 124)?)?;
        m.set_global("u", m.global("Uses")?.call0()?)?;
        m.global("u")?.set_attr("v", 2)?;
        rt::builtins::print(
            py,
            (
                m.global("u")?.attr("v")?,
                m.global("u")?.attr("missing")?,
                m.global("u")?.contains("x")?,
                m.global("u")?.rich_compare(m.global("u")?, CompareOp::Lt)?,
                rt::builtin(py, "sorted")?
                    .call1((PyList::new(py, [&m.global("u")?, &m.global("u")?])?,))?
                    .get_item(0)?
                    .is(m.global("u")?),
            ),
        )?;
        frame.with(&m.global("u")?, |value| -> PyResult<()> {
            m.set_global("same", value)?;
            Err(frame.raise(PyValueError::new_err("suppressed")))
        })?;
        rt::builtins::print(py, (m.global("same")?.is(m.global("u")?),))?;
        Ok(())
    })
}
