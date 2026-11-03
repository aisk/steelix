use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;

// classes_exceptions.py:20
pub fn load<'py>(m: &rt::Module<'py>, key: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "load", 20, |frame| {
        frame.line(21);
        if key.eq("missing")? {
            frame.line(22);
            return Err(frame.raise(m.global("ConfigError")?.call1((key, "not found"))?));
        }
        frame.line(23);
        if key.eq("busy")? {
            frame.line(24);
            return Err(frame.raise(m.global("Retry")?.call1((1, 2))?));
        }
        frame.line(25);
        key.call_method0("upper")
    })
}

/// Base error.
pub mod app_error {
    use super::*;

    // classes_exceptions.py:4
    /// The body of `class AppError`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "AppError", 4, |frame| {
            frame.line(5);
            ns.set("__doc__", "Base error.")?;
            Ok(())
        })
    }
}

pub mod config_error {
    use super::*;

    // classes_exceptions.py:8
    /// The body of `class ConfigError`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "ConfigError", 8, |frame| {
            let __class__ = ns.class_cell();
            frame.line(9);
            ns.set(
                "__init__",
                rt::function!(
                    ns,
                    __init__[&__class__](slf as "self", key, problem),
                    qualname = "ConfigError.__init__"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_exceptions.py:9
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
        key: &Bound<'py, PyAny>,
        problem: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "ConfigError.__init__", 9, |frame| {
            frame.line(10);
            rt::super_(__class__, slf)?
                .getattr("__init__")?
                .call1((rt::fstring(
                    py,
                    (rt::format(key, "")?, ": ", rt::format(problem, "")?),
                )?,))?;
            frame.line(11);
            slf.setattr("key", key)?;
            frame.line(12);
            slf.setattr("problem", problem)?;
            Ok(rt::none(py))
        })
    }
}

pub mod retry {
    use super::*;

    // classes_exceptions.py:15
    /// The body of `class Retry`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Retry", 15, |frame| {
            frame.line(16);
            ns.set(
                "__str__",
                rt::function!(ns, __str__(slf as "self"), qualname = "Retry.__str__")?,
            )?;
            Ok(())
        })
    }

    // classes_exceptions.py:16
    pub fn __str__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Retry.__str__", 16, |frame| {
            frame.line(17);
            rt::str(py, "retry ").add(rt::str(py, ", ").getattr("join")?.call1((
                rt::builtin(py, "map")?.call1((rt::builtin(py, "str")?, slf.getattr("args")?))?,
            ))?)
        })
    }
}

pub mod kept {
    use super::*;

    // classes_exceptions.py:42
    /// The body of `class Kept`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Kept", 42, |frame| {
            frame.line(43);
            ns.set(
                "describe",
                rt::function!(ns, describe(slf as "self"), qualname = "Kept.describe")?,
            )?;
            Ok(())
        })
    }

    // classes_exceptions.py:43
    pub fn describe<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Kept.describe", 43, |frame| {
            frame.line(44);
            let list = PyList::empty(py);
            for a in PyList::new(py, [&slf.getattr("args")?])?.try_iter()? {
                let a = a?;
                list.append(a.len()?)?;
                py.check_signals()?;
            }
            Ok(list.into_any())
        })
    }
}

pub mod broken {
    use super::*;

    // classes_exceptions.py:53
    /// The body of `class Broken`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Broken", 53, |frame| {
            frame.line(54);
            ns.set(
                "__init__",
                rt::function!(
                    ns,
                    __init__(slf as "self", value),
                    qualname = "Broken.__init__"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_exceptions.py:54
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        value: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Broken.__init__", 54, |frame| {
            frame.line(55);
            if value.lt(0)? {
                frame.line(56);
                return Err(frame.raise(PyValueError::new_err("negative")));
            }
            frame.line(57);
            slf.setattr("value", value)?;
            Ok(rt::none(m.py()))
        })
    }
}

pub mod grouped {
    use super::*;

    // classes_exceptions.py:67
    /// The body of `class Grouped`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Grouped", 67, |frame| {
            frame.line(68);
            ns.set(
                "derive",
                rt::function!(
                    ns,
                    derive(slf as "self", excs),
                    qualname = "Grouped.derive"
                )?,
            )?;
            Ok(())
        })
    }

    // classes_exceptions.py:68
    pub fn derive<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        excs: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Grouped.derive", 68, |frame| {
            frame.line(69);
            m.global("Grouped")?.call1((slf.getattr("message")?, excs))
        })
    }
}

/// Executes the body of `classes_exceptions.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "Custom exception classes raised and caught by native and kept code.",
        )?;
        frame.line(4);
        m.set_global(
            "AppError",
            rt::class!(
                m,
                "AppError"(rt::builtin(py, "Exception")?),
                app_error::body,
                line = 4
            )?,
        )?;
        frame.line(8);
        m.set_global(
            "ConfigError",
            rt::class!(
                m,
                "ConfigError"(m.global("AppError")?),
                config_error::body,
                line = 8,
                attributes = ["key", "problem"]
            )?,
        )?;
        frame.line(15);
        m.set_global(
            "Retry",
            rt::class!(m, "Retry"(m.global("AppError")?), retry::body, line = 15)?,
        )?;
        rt::def!(m, load(key))?;
        frame.line(28);
        for item in PyList::new(py, ["ok", "missing", "busy"])?.try_iter()? {
            m.set_global("key", item?)?;
            frame.line(30);
            match (|| rt::builtins::print(py, (load(m, &m.global("key")?)?,)))() {
                Ok(_) => {}
                Err(err) if frame.matches(&err, 31, || m.global("ConfigError"))? => {
                    let exception = frame.catch(err);
                    m.set_global("err", &exception)?;
                    let _handling = rt::Handling::new(&exception);
                    frame.line(32);
                    rt::builtins::print(
                        py,
                        (
                            "config:",
                            m.global("err")?,
                            m.global("err")?.getattr("key")?,
                            m.global("err")?.getattr("problem")?,
                            m.global("err")?.getattr("args")?,
                            m.global("err")?.is_instance(&m.global("AppError")?)?,
                        ),
                    )?;
                    m.unbind_global("err")?;
                }
                Err(err) if frame.matches(&err, 33, || m.global("AppError"))? => {
                    let exception = frame.catch(err);
                    m.set_global("err", &exception)?;
                    let _handling = rt::Handling::new(&exception);
                    frame.line(34);
                    rt::builtins::print(
                        py,
                        (
                            "app:",
                            m.global("err")?,
                            m.global("err")?.repr()?,
                            m.global("err")?
                                .get_type()
                                .getattr("__mro__")?
                                .get_item(1)?
                                .getattr("__name__")?,
                        ),
                    )?;
                    m.unbind_global("err")?;
                }
                Err(err) => return Err(err),
            }
            py.check_signals()?;
            frame.line(28);
        }
        match (|| -> PyResult<()> {
            frame.line(37);
            Err(frame.raise_from(
                m.global("ConfigError")?.call1(("a", "b"))?,
                rt::builtin(py, "KeyError")?.call1(("k",))?,
            ))
        })() {
            Ok(()) => unreachable!(),
            Err(err) if frame.matches(&err, 38, || m.global("AppError"))? => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(39);
                rt::builtins::print(
                    py,
                    (
                        m.global("err")?.getattr("__cause__")?.repr()?,
                        m.global("err")?.getattr("__suppress_context__")?,
                    ),
                )?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        frame.line(42);
        m.set_global(
            "Kept",
            rt::class!(
                m,
                "Kept"(rt::builtin(py, "Exception")?),
                kept::body,
                line = 42
            )?,
        )?;
        match (|| -> PyResult<()> {
            frame.line(48);
            Err(frame.raise(m.global("Kept")?.call1(("x", "y"))?))
        })() {
            Ok(()) => unreachable!(),
            Err(err) if frame.matches(&err, 49, || m.global("Kept"))? => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(50);
                rt::builtins::print(
                    py,
                    (
                        m.global("err")?.call_method0("describe")?,
                        m.global("err")?.getattr("args")?,
                    ),
                )?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        frame.line(53);
        m.set_global(
            "Broken",
            rt::class!(
                m,
                "Broken"(),
                broken::body,
                line = 53,
                attributes = ["value"]
            )?,
        )?;
        frame.line(61);
        match (|| m.global("Broken")?.call1((-1,)))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyValueError>(py) => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(63);
                rt::builtins::print(py, ("ValueError:", m.global("err")?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        frame.line(64);
        rt::builtins::print(py, (m.global("Broken")?.call1((2,))?.getattr("value")?,))?;
        frame.line(67);
        m.set_global(
            "Grouped",
            rt::class!(
                m,
                "Grouped"(rt::builtin(py, "ExceptionGroup")?),
                grouped::body,
                line = 67
            )?,
        )?;
        frame.line(72);
        m.set_global(
            "g",
            m.global("Grouped")?.call1((
                "many",
                PyList::new(
                    py,
                    [
                        &rt::builtin(py, "ValueError")?.call1((1,))?,
                        &rt::builtin(py, "TypeError")?.call1((2,))?,
                    ],
                )?,
            ))?,
        )?;
        frame.line(73);
        let [match_, rest] = rt::unpack(
            &m.global("g")?
                .getattr("split")?
                .call1((rt::builtin(py, "ValueError")?,))?,
        )?;
        m.set_global("match_", match_)?;
        m.set_global("rest", rest)?;
        frame.line(74);
        rt::builtins::print(
            py,
            (
                m.global("match_")?.get_type().getattr("__name__")?,
                m.global("match_")?.getattr("exceptions")?,
                m.global("rest")?.getattr("exceptions")?,
            ),
        )?;
        Ok(())
    })
}
