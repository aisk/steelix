//! Parts of `classes_body.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 7, `def show_tb`: list comprehension
//! - line 45, `assign` statement: list comprehension
//! - line 88, `def failing_body`: class definition in a function
//! - line 102, `try` statement: list comprehension
//! - line 117, `def kept`: list comprehension

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

pub mod body_ {
    use super::*;

    // classes_body.py:11
    /// The body of `class Body`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let py = m.py();
        rt::frame!(ns, "Body", 11, |frame| {
            frame.line(12);
            ns.set("paths", rt::import_module(m, "os.path")?)?;
            frame.line(13);
            let math = rt::import_names(m, "math", &["sqrt"])?;
            ns.set("root", rt::import_from(&math, "sqrt")?)?;
            frame.line(15);
            ns.set("total", 0)?;
            frame.line(16);
            for item in ns.get("range")?.call1((4,))?.try_iter()? {
                ns.set("i", item?)?;
                frame.line(17);
                ns.set("total", rt::iadd(&ns.get("total")?, ns.get("i")?)?)?;
                py.check_signals()?;
                frame.line(16);
            }
            frame.line(18);
            while ns.get("total")?.gt(10)? {
                frame.line(19);
                ns.set("total", rt::isub(&ns.get("total")?, 1)?)?;
                py.check_signals()?;
                frame.line(18);
            }
            frame.line(20);
            ns.set("names", PyList::empty(py))?;
            frame.line(21);
            if ns
                .get("sys")?
                .getattr("version_info")?
                .ge(PyTuple::new(py, [3, 0])?)?
            {
                frame.line(23);
                ns.set(
                    "version",
                    rt::function!(ns, version(slf as "self"), qualname = "Body.version")?,
                )?;
            } else {
                frame.line(28);
                ns.set(
                    "version",
                    rt::function!(ns, version_(slf as "self"), qualname = "Body.version")?,
                )?;
            }
            let result = (|| -> PyResult<()> {
                frame.line(32);
                match PyDict::new(py).as_any().get_item("key") {
                    Ok(value) => {
                        ns.set("missing", &value)?;
                    }
                    Err(err) if err.is_instance_of::<PyKeyError>(py) => {
                        let exception = frame.catch(err);
                        ns.set("err", &exception)?;
                        let _handling = rt::Handling::new(&exception);
                        frame.line(34);
                        ns.set("caught", ns.get("repr")?.call1((ns.get("err")?,))?)?;
                        ns.unbind("err")?;
                    }
                    Err(err) => return Err(err),
                }
                Ok(())
            })();
            let finally = frame.finally(result);
            frame.line(36);
            ns.set("cleanup", true)?;
            finally.resume()?;
            frame.line(37);
            frame.with(&ns.get("open")?.call1((ns.get("__file__")?,))?, |value| {
                ns.set("handle", value)?;
                frame.line(38);
                ns.set(
                    "first",
                    ns.get("handle")?
                        .call_method0("readline")?
                        .call_method0("strip")?,
                )?;
                Ok(())
            })?;
            frame.line(39);
            ns.set("temp", 1)?;
            frame.line(40);
            ns.del("temp")?;
            match (|| -> PyResult<()> {
                frame.line(42);
                ns.del("temp")?;
                Ok(())
            })() {
                Ok(()) => {}
                Err(err) if err.is_instance_of::<PyNameError>(py) => {
                    let exception = frame.catch(err);
                    ns.set("err", &exception)?;
                    let _handling = rt::Handling::new(&exception);
                    frame.line(44);
                    ns.set("name_error", ns.get("str")?.call1((ns.get("err")?,))?)?;
                    ns.unbind("err")?;
                }
                Err(err) => return Err(err),
            }
            frame.line(45);
            ns.set("x", 1)?;
            // classes_body.py:45, kept as Python: list comprehension at 45:22
            rt::exec_python!(
                ns,
                line = 45,
                r#"
           squares = [x * x for x in range(3)]
"#
            )?;
            ns.set("y", 2)?;
            // classes_body.py:46
            fn lambda<'py>(
                m: &rt::Module<'py>,
                slf: &Bound<'py, PyAny>,
            ) -> PyResult<Bound<'py, PyAny>> {
                rt::frame!(m, "Body.<lambda>", 46, |_| rt::str(m.py(), "lambda ")
                    .add(slf.get_type().getattr("__name__")?))
            }
            frame.line(46);
            ns.set(
                "label",
                rt::function!(m, lambda(slf as "self"), qualname = "Body.<lambda>")?,
            )?;
            frame.line(47);
            ns.set("shadow", ns.get("len")?)?;
            frame.line(48);
            ns.set("len", 5)?;
            frame.line(49);
            ns.set(
                "used",
                ns.get("shadow")?.call1((PyList::new(py, [1, 2])?,))?,
            )?;
            frame.line(50);
            ns.set("builtin", ns.get("print")?.getattr("__name__")?)?;
            frame.line(51);
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
        rt::frame!(ns, "Generic", 61, |frame| {
            frame.line(62);
            ns.set(
                "__class_getitem__",
                rt::function!(
                    ns,
                    __class_getitem__(cls, item),
                    qualname = "Generic.__class_getitem__"
                )?,
            )?;
            frame.line(65);
            ns.set(
                "__init_subclass__",
                rt::function!(
                    ns,
                    __init_subclass__(cls, **kwargs),
                    qualname = "Generic.__init_subclass__"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_body.py:62
    pub fn __class_getitem__<'py>(
        m: &rt::Module<'py>,
        cls: &Bound<'py, PyAny>,
        item: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Generic.__class_getitem__", 62, |frame| {
            frame.line(63);
            rt::fstring(
                m.py(),
                (
                    rt::format(&cls.getattr("__name__")?, "")?,
                    "[",
                    rt::format(&item.getattr("__name__")?, "")?,
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
        rt::frame!(m, "Generic.__init_subclass__", 65, |frame| {
            frame.line(66);
            cls.setattr("created", true)?;
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
        rt::frame!(ns, "New", 76, |frame| {
            let __class__ = ns.class_cell();
            frame.line(77);
            ns.set(
                "__new__",
                rt::function!(
                    ns,
                    __new__[&__class__](cls, *args),
                    qualname = "New.__new__"
                )?,
            )?;
            frame.line(81);
            ns.set(
                "__init__",
                rt::function!(
                    ns,
                    __init__(slf as "self", value),
                    qualname = "New.__init__"
                )?,
            )?;
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
        rt::frame!(m, "New.__new__", 77, |frame| {
            frame.line(78);
            rt::builtins::print(m.py(), ("new", cls.getattr("__name__")?, args))?;
            frame.line(79);
            rt::super_(__class__, cls)?.call_method1("__new__", (cls,))
        })
    }

    // classes_body.py:81
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        value: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "New.__init__", 81, |frame| {
            frame.line(82);
            slf.setattr("value", value)?;
            Ok(rt::none(m.py()))
        })
    }
}

pub mod failing {
    use super::*;

    // classes_body.py:95
    /// The body of `class Failing`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Failing", 95, |frame| {
            frame.line(96);
            ns.set("before", 1)?;
            frame.line(97);
            ns.set("value", rt::int(m.py(), 1).div(0)?)?;
            Ok(())
        })
    }
}

pub mod annotated {
    use super::*;

    // classes_body.py:111
    /// The body of `class Annotated`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Annotated", 111, |frame| {
            frame.line(112);
            ns.set("alias", ns.get("int")?)?;
            frame.line(114);
            ns.set(
                "method",
                rt::function!(
                    ns,
                    method(slf as "self", x: "alias") -> "'Annotated'",
                    qualname = "Annotated.method"
                )?,
            )?;
            // classes_body.py:117, kept as Python: list comprehension at 118:16
            rt::exec_python!(
                ns,
                line = 117,
                r#"
    def kept(self, x: alias) -> list[alias]:
        return [x for _ in "x"]
"#
            )?;
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
}

pub mod uses {
    use super::*;

    // classes_body.py:124
    /// The body of `class Uses`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Uses", 124, |frame| {
            let __class__ = ns.class_cell();
            frame.line(125);
            ns.set(
                "__getattr__",
                rt::function!(
                    ns,
                    __getattr__(slf as "self", name),
                    qualname = "Uses.__getattr__"
                )?,
            )?;
            frame.line(128);
            ns.set(
                "__setattr__",
                rt::function!(
                    ns,
                    __setattr__[&__class__](slf as "self", name, value),
                    qualname = "Uses.__setattr__"
                )?,
            )?;
            frame.line(131);
            ns.set(
                "__contains__",
                rt::function!(
                    ns,
                    __contains__(slf as "self", item),
                    qualname = "Uses.__contains__"
                )?,
            )?;
            frame.line(134);
            ns.set(
                "__lt__",
                rt::function!(
                    ns,
                    __lt__(slf as "self", other),
                    qualname = "Uses.__lt__"
                )?,
            )?;
            frame.line(137);
            ns.set(
                "__enter__",
                rt::function!(
                    ns,
                    __enter__(slf as "self"),
                    qualname = "Uses.__enter__"
                )?,
            )?;
            frame.line(140);
            ns.set(
                "__exit__",
                rt::function!(
                    ns,
                    __exit__(slf as "self", *exc),
                    qualname = "Uses.__exit__"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_body.py:125
    pub fn __getattr__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Uses.__getattr__", 125, |frame| {
            frame.line(126);
            name.call_method0("upper")
        })
    }

    // classes_body.py:128
    pub fn __setattr__<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
        value: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Uses.__setattr__", 128, |frame| {
            frame.line(129);
            rt::super_(__class__, slf)?
                .getattr("__setattr__")?
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
        rt::frame!(m, "Uses.__contains__", 131, |frame| {
            frame.line(132);
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
        rt::frame!(m, "Uses.__exit__", 140, |frame| {
            frame.line(141);
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
        frame.line(3);
        m.set_global("sys", rt::import(m, "sys")?)?;
        frame.line(4);
        m.set_global("traceback", rt::import(m, "traceback")?)?;
        // classes_body.py:7, kept as Python: list comprehension at 8:12
        rt::exec_python!(
            m,
            line = 7,
            r#"
def show_tb(err):
    return [(f.name, f.lineno) for f in traceback.extract_tb(err.__traceback__)]
"#
        )?;
        frame.line(11);
        m.set_global("Body", rt::class!(m, "Body"(), body_::body, line = 11)?)?;
        frame.line(54);
        m.set_global("b", m.global("Body")?.call0()?)?;
        frame.line(55);
        rt::builtins::print(
            py,
            (
                m.global("Body")?.getattr("total")?,
                m.global("Body")?.getattr("i")?,
                m.global("Body")?
                    .getattr("version")?
                    .call1((m.global("b")?,))?,
                m.global("Body")?.getattr("caught")?,
                m.global("Body")?.getattr("cleanup")?,
                m.global("Body")?.getattr("first")?,
            ),
        )?;
        frame.line(56);
        rt::builtins::print(
            py,
            (
                m.global("Body")?.getattr("name_error")?,
                m.global("Body")?.getattr("squares")?,
                m.global("Body")?.getattr("x")?,
                m.global("Body")?.getattr("y")?,
                m.global("b")?.call_method0("label")?,
                m.global("Body")?.getattr("used")?,
                m.global("Body")?.getattr("len")?,
            ),
        )?;
        frame.line(57);
        rt::builtins::print(
            py,
            (
                m.global("Body")?
                    .getattr("paths")?
                    .call_method1("join", ("a", "b"))?,
                m.global("Body")?.call_method1("root", (16,))?,
                m.global("Body")?.getattr("builtin")?,
                rt::builtin(py, "hasattr")?.call1((m.global("Body")?, "err"))?,
            ),
        )?;
        frame.line(58);
        rt::builtins::print(
            py,
            (
                m.global("Body")?
                    .getattr("label")?
                    .getattr("__qualname__")?,
                m.global("Body")?.getattr("__static_attributes__")?,
            ),
        )?;
        frame.line(61);
        m.set_global(
            "Generic",
            rt::class!(m, "Generic"(), generic::body, line = 61)?,
        )?;
        frame.line(69);
        m.set_global(
            "Sub",
            rt::class!(m, "Sub"(m.global("Generic")?), sub::body, line = 69)?,
        )?;
        frame.line(73);
        rt::builtins::print(
            py,
            (
                m.global("Generic")?.get_item(rt::builtin(py, "int")?)?,
                m.global("Sub")?.getattr("created")?,
                m.global("Generic")?
                    .getattr("__dict__")?
                    .get_item("__class_getitem__")?
                    .get_type()
                    .getattr("__name__")?,
            ),
        )?;
        frame.line(76);
        m.set_global(
            "New",
            rt::class!(m, "New"(), new::body, line = 76, attributes = ["value"])?,
        )?;
        frame.line(85);
        rt::builtins::print(
            py,
            (
                m.global("New")?.call1((3,))?.getattr("value")?,
                m.global("New")?
                    .getattr("__dict__")?
                    .get_item("__new__")?
                    .get_type()
                    .getattr("__name__")?,
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
            frame.line(95);
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
                let _handling = rt::Handling::new(&exception);
                frame.line(100);
                rt::builtins::print(
                    py,
                    (
                        "ZeroDivisionError",
                        m.global("show_tb")?.call1((m.global("err")?,))?,
                    ),
                )?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        // classes_body.py:102, kept as Python: list comprehension at 105:18
        rt::exec_python!(
            m,
            line = 102,
            r#"
try:

    class FailingKept:
        values = [1 / 0 for _ in "x"]

except ZeroDivisionError as err:
    print("ZeroDivisionError", show_tb(err))
"#
        )?;
        frame.line(111);
        m.set_global(
            "Annotated",
            rt::class!(m, "Annotated"(), annotated::body, line = 111)?,
        )?;
        frame.line(121);
        rt::builtins::print(
            py,
            (
                m.global("Annotated")?
                    .getattr("method")?
                    .getattr("__annotations__")?,
                m.global("Annotated")?
                    .getattr("kept")?
                    .getattr("__annotations__")?,
            ),
        )?;
        frame.line(124);
        m.set_global("Uses", rt::class!(m, "Uses"(), uses::body, line = 124)?)?;
        frame.line(145);
        m.set_global("u", m.global("Uses")?.call0()?)?;
        frame.line(146);
        m.global("u")?.setattr("v", 2)?;
        frame.line(147);
        rt::builtins::print(
            py,
            (
                m.global("u")?.getattr("v")?,
                m.global("u")?.getattr("missing")?,
                m.global("u")?.contains("x")?,
                m.global("u")?.rich_compare(m.global("u")?, CompareOp::Lt)?,
                rt::builtin(py, "sorted")?
                    .call1((PyList::new(py, [&m.global("u")?, &m.global("u")?])?,))?
                    .get_item(0)?
                    .is(m.global("u")?),
            ),
        )?;
        frame.line(148);
        frame.with(&m.global("u")?, |value| -> PyResult<()> {
            m.set_global("same", value)?;
            frame.line(149);
            return Err(frame.raise(PyValueError::new_err("suppressed")));
        })?;
        frame.line(150);
        rt::builtins::print(py, (m.global("same")?.is(m.global("u")?),))?;
        Ok(())
    })
}
