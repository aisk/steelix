use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// classes_exceptions.py:20
pub fn load<'py>(m: &rt::Module<'py>, key: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "load", 20, |frame| {
        if key.eq("missing")? {
            return Err(frame.raise(m.global("ConfigError")?.call1((key, "not found"))?));
        }
        if key.eq("busy")? {
            return Err(frame.raise(m.global("Retry")?.call1((1, 2))?));
        }
        key.method0("upper")
    })
}

/// Base error.
pub mod app_error {
    use super::*;

    // classes_exceptions.py:4
    /// The body of `class AppError`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "AppError", 4, |_| {
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
        rt::frame!(ns, "ConfigError", 8, |_| {
            let __class__ = ns.class_cell();
            rt::def!(ns, __init__[&__class__](slf as "self", key, problem))?;
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
        rt::frame!(m, "ConfigError.__init__", 9, |_| {
            rt::super_(__class__, slf)?
                .attr("__init__")?
                .call1((rt::fstring(
                    py,
                    (rt::format(key, "")?, ": ", rt::format(problem, "")?),
                )?,))?;
            slf.set_attr("key", key)?;
            slf.set_attr("problem", problem)?;
            Ok(rt::none(py))
        })
    }
}

pub mod retry {
    use super::*;

    // classes_exceptions.py:15
    /// The body of `class Retry`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Retry", 15, |_| {
            rt::def!(ns, __str__(slf as "self"))?;
            Ok(())
        })
    }

    // classes_exceptions.py:16
    pub fn __str__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Retry.__str__", 16, |_| {
            rt::str(py, "retry ").add(rt::str(py, ", ").attr("join")?.call1((
                rt::builtin(py, "map")?.call1((rt::builtin(py, "str")?, slf.attr("args")?))?,
            ))?)
        })
    }
}

pub mod kept {
    use super::*;

    // classes_exceptions.py:42
    /// The body of `class Kept`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Kept", 42, |_| {
            rt::def!(ns, describe(slf as "self"))?;
            Ok(())
        })
    }

    // classes_exceptions.py:43
    pub fn describe<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Kept.describe", 43, |_| {
            let list = PyList::empty(py);
            for a in PyList::new(py, [&slf.attr("args")?])?.try_iter()? {
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
        rt::frame!(ns, "Broken", 53, |_| {
            rt::def!(ns, __init__(slf as "self", value))?;
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
            if value.lt(0)? {
                return Err(frame.raise(PyValueError::new_err("negative")));
            }
            slf.set_attr("value", value)?;
            Ok(rt::none(m.py()))
        })
    }
}

pub mod grouped {
    use super::*;

    // classes_exceptions.py:67
    /// The body of `class Grouped`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Grouped", 67, |_| {
            rt::def!(ns, derive(slf as "self", excs))?;
            Ok(())
        })
    }

    // classes_exceptions.py:68
    pub fn derive<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        excs: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Grouped.derive", 68, |_| {
            m.global("Grouped")?.call1((slf.attr("message")?, excs))
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
        m.set_global(
            "AppError",
            rt::class!(
                m,
                "AppError"(rt::builtin(py, "Exception")?),
                app_error::body,
                line = 4
            )?,
        )?;
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
        m.set_global(
            "Retry",
            rt::class!(m, "Retry"(m.global("AppError")?), retry::body, line = 15)?,
        )?;
        rt::def!(m, load(key))?;
        for item in PyList::new(py, ["ok", "missing", "busy"])?.try_iter()? {
            m.set_global("key", item?)?;
            match (|| rt::builtins::print(py, (load(m, &m.global("key")?)?,)))() {
                Ok(_) => {}
                Err(err) if frame.matches(&err, 1, || m.global("ConfigError"))? => {
                    let exception = frame.catch(err);
                    m.set_global("err", &exception)?;
                    rt::builtins::print(
                        py,
                        (
                            "config:",
                            m.global("err")?,
                            m.global("err")?.attr("key")?,
                            m.global("err")?.attr("problem")?,
                            m.global("err")?.attr("args")?,
                            m.global("err")?.is_instance(&m.global("AppError")?)?,
                        ),
                    )?;
                    m.unbind_global("err")?;
                }
                Err(err) if frame.matches(&err, 1, || m.global("AppError"))? => {
                    let exception = frame.catch(err);
                    m.set_global("err", &exception)?;
                    rt::builtins::print(
                        py,
                        (
                            "app:",
                            m.global("err")?,
                            m.global("err")?.repr()?,
                            m.global("err")?
                                .get_type()
                                .attr("__mro__")?
                                .get_item(1)?
                                .attr("__name__")?,
                        ),
                    )?;
                    m.unbind_global("err")?;
                }
                Err(err) => return Err(err),
            }
            py.check_signals()?;
        }
        match (|| -> PyResult<()> {
            Err(frame.raise_from(
                m.global("ConfigError")?.call1(("a", "b"))?,
                rt::builtin(py, "KeyError")?.call1(("k",))?,
            ))
        })() {
            Ok(()) => unreachable!(),
            Err(err) if frame.matches(&err, 1, || m.global("AppError"))? => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                rt::builtins::print(
                    py,
                    (
                        m.global("err")?.attr("__cause__")?.repr()?,
                        m.global("err")?.attr("__suppress_context__")?,
                    ),
                )?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        m.set_global(
            "Kept",
            rt::class!(
                m,
                "Kept"(rt::builtin(py, "Exception")?),
                kept::body,
                line = 42
            )?,
        )?;
        match (|| -> PyResult<()> { Err(frame.raise(m.global("Kept")?.call1(("x", "y"))?)) })() {
            Ok(()) => unreachable!(),
            Err(err) if frame.matches(&err, 1, || m.global("Kept"))? => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                rt::builtins::print(
                    py,
                    (
                        m.global("err")?.method0("describe")?,
                        m.global("err")?.attr("args")?,
                    ),
                )?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
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
        match (|| m.global("Broken")?.call1((-1,)))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyValueError>(py) => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                rt::builtins::print(py, ("ValueError:", m.global("err")?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        rt::builtins::print(py, (m.global("Broken")?.call1((2,))?.attr("value")?,))?;
        m.set_global(
            "Grouped",
            rt::class!(
                m,
                "Grouped"(rt::builtin(py, "ExceptionGroup")?),
                grouped::body,
                line = 67
            )?,
        )?;
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
        let [match_, rest] = rt::unpack(
            &m.global("g")?
                .attr("split")?
                .call1((rt::builtin(py, "ValueError")?,))?,
        )?;
        m.set_global("match_", match_)?;
        m.set_global("rest", rest)?;
        rt::builtins::print(
            py,
            (
                m.global("match_")?.get_type().attr("__name__")?,
                m.global("match_")?.attr("exceptions")?,
                m.global("rest")?.attr("exceptions")?,
            ),
        )?;
        Ok(())
    })
}
