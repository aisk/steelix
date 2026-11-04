use pyo3::exceptions::PyException;
use pyo3::exceptions::PyRuntimeError;
use pyo3::exceptions::PyValueError;
use pyo3::exceptions::PyZeroDivisionError;
use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// match_errors.py:6
pub fn attempt<'py>(
    m: &rt::Module<'py>,
    label: &Bound<'py, PyAny>,
    function: &Bound<'py, PyAny>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "attempt", 6, |frame| {
        match (|| rt::builtins::print(py, (label, function.call1((value,))?)))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyException>(py) => {
                let err = frame.catch(err);
                rt::builtins::print(
                    py,
                    (
                        label,
                        rt::fstring(
                            py,
                            (
                                rt::format(&err.get_type().attr("__name__")?, "")?,
                                ": ",
                                rt::format(&err, "")?,
                            ),
                        )?,
                    ),
                )?;
            }
            Err(err2) => return Err(err2),
        }
        Ok(rt::none(py))
    })
}

// match_errors.py:35
pub fn not_a_class<'py>(
    m: &rt::Module<'py>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "not_a_class", 35, |_| {
        let cls = rt::str(py, "str");
        if rt::match_instance(value, &cls.attr("upper")?)? {
            return Ok(rt::str(py, "no"));
        }
        Ok(rt::str(py, "unreachable"))
    })
}

// match_errors.py:43
pub fn positional<'py>(
    m: &rt::Module<'py>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "positional", 43, |_| {
        if let Some([a]) = rt::match_class(value, &m.global("NoArgs")?, 1, [])? {
            return Ok(a);
        }
        Ok(rt::none(m.py()))
    })
}

// match_errors.py:49
pub fn builtin_positional<'py>(
    m: &rt::Module<'py>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "builtin_positional", 49, |_| {
        if let Some([a, b]) = rt::match_class(value, &rt::builtin(py, "int")?, 2, [])? {
            return Ok(PyTuple::new(py, [&a, &b])?.into_any());
        }
        Ok(rt::none(py))
    })
}

// match_errors.py:55
pub fn bad_match_args<'py>(
    m: &rt::Module<'py>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "bad_match_args", 55, |_| {
        if let Some([x]) = rt::match_class(value, &m.global("BadArgs")?, 1, [])? {
            return Ok(x);
        }
        Ok(rt::none(m.py()))
    })
}

// match_errors.py:61
pub fn match_args_elements<'py>(
    m: &rt::Module<'py>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "match_args_elements", 61, |_| {
        if let Some([x, y]) = rt::match_class(value, &m.global("NotStrings")?, 2, [])? {
            return Ok(PyTuple::new(py, [&x, &y])?.into_any());
        }
        Ok(rt::none(py))
    })
}

// match_errors.py:67
pub fn multiple<'py>(
    m: &rt::Module<'py>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "multiple", 67, |_| {
        if let Some([x, other]) = rt::match_class(value, &m.global("Dup")?, 1, ["x"])? {
            return Ok(PyTuple::new(py, [&x, &other])?.into_any());
        }
        Ok(rt::none(py))
    })
}

// match_errors.py:73
pub fn duplicate_keys<'py>(
    m: &rt::Module<'py>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "duplicate_keys", 73, |_| {
        if rt::match_mapping(value, 2)?
            && let Some([a, b]) = rt::match_keys(value, (1, m.global("Value")?.attr("one")?))?
        {
            return Ok(PyTuple::new(py, [&a, &b])?.into_any());
        }
        Ok(rt::none(py))
    })
}

// match_errors.py:99
pub fn rest<'py>(m: &rt::Module<'py>, value: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "rest", 99, |_| {
        if let Some([a]) = rt::match_map(value, ["a"])?
            && let others = rt::match_rest(value, ["a"])?
        {
            return Ok(PyTuple::new(py, [&a, &others])?.into_any());
        }
        Ok(rt::none(py))
    })
}

// match_errors.py:105
pub fn failing_eq<'py>(
    m: &rt::Module<'py>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "failing_eq", 105, |_| {
        if let Some([item]) = rt::match_seq(value)?
            && item.eq(m.global("Value")?.attr("one")?)?
        {
            return Ok(rt::str(py, "one"));
        }
        Ok(rt::none(py))
    })
}

// match_errors.py:116
pub fn unhashable<'py>(
    m: &rt::Module<'py>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "unhashable", 116, |_| {
        if rt::match_mapping(value, 1)?
            && let Some([_]) = rt::match_keys(value, (m.global("Value")?.attr("unhashable")?,))?
        {
            return Ok(rt::str(py, "no"));
        }
        Ok(rt::none(py))
    })
}

pub mod no_args {
    use super::*;

    // match_errors.py:13
    /// The body of `class NoArgs`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

pub mod bad_args {
    use super::*;

    // match_errors.py:17
    /// The body of `class BadArgs`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "BadArgs", 17, |_| {
            ns.set("__match_args__", PyList::new(m.py(), ["x"])?)?;
            Ok(())
        })
    }
}

pub mod not_strings {
    use super::*;

    // match_errors.py:21
    /// The body of `class NotStrings`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let py = m.py();
        rt::frame!(ns, "NotStrings", 21, |_| {
            ns.set(
                "__match_args__",
                PyTuple::new(py, [&rt::str(py, "x"), &rt::int(py, 1)])?,
            )?;
            rt::def!(ns, __init__(slf as "self"))?;
            Ok(())
        })
    }

    // match_errors.py:24
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "NotStrings.__init__", 24, |_| {
            slf.set_attr("x", 0)?;
            Ok(rt::none(m.py()))
        })
    }
}

pub mod dup {
    use super::*;

    // match_errors.py:28
    /// The body of `class Dup`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Dup", 28, |_| {
            ns.set("__match_args__", PyTuple::new(m.py(), ["x"])?)?;
            rt::def!(ns, __init__(slf as "self"))?;
            Ok(())
        })
    }

    // match_errors.py:31
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Dup.__init__", 31, |_| {
            slf.set_attr("x", 1)?;
            Ok(rt::none(m.py()))
        })
    }
}

pub mod value {
    use super::*;

    // match_errors.py:79
    /// The body of `class Value`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Value", 79, |_| {
            ns.set("one", 1)?;
            Ok(())
        })
    }
}

/// A mapping whose keys cannot be copied for `**rest`.
pub mod bad_mapping {
    use super::*;

    // match_errors.py:83
    /// The body of `class BadMapping`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "BadMapping", 83, |_| {
            ns.set(
                "__doc__",
                "A mapping whose keys cannot be copied for `**rest`.",
            )?;
            rt::def!(ns, __getitem__(slf as "self", key))?;
            rt::def!(ns, __len__(slf as "self"))?;
            rt::def!(ns, __iter__(slf as "self"))?;
            rt::def!(ns, get(slf as "self", key, default = m.py().None()))?;
            Ok(())
        })
    }

    // match_errors.py:86
    pub fn __getitem__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        key: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(key.clone())
    }

    // match_errors.py:89
    pub fn __len__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::int(m.py(), 1))
    }

    // match_errors.py:92
    pub fn __iter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "BadMapping.__iter__", 92, |frame| {
            Err(frame.raise(PyRuntimeError::new_err("cannot iterate")))
        })
    }

    // match_errors.py:95
    pub fn get<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        key: &Bound<'py, PyAny>,
        default: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(key.clone())
    }
}

pub mod explodes {
    use super::*;

    // match_errors.py:111
    /// The body of `class Explodes`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Explodes", 111, |_| {
            rt::def!(ns, __eq__(slf as "self", other))?;
            Ok(())
        })
    }

    // match_errors.py:112
    pub fn __eq__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        other: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Explodes.__eq__", 112, |frame| {
            Err(frame.raise(PyValueError::new_err("no comparing")))
        })
    }
}

/// Executes the body of `match_errors.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        // Every error a `match` can raise at run time, with CPython's messages.
        m.set_global("collections", rt::import(m, "collections.abc")?)?;
        rt::def!(m, attempt(label, function, value))?;
        m.set_global(
            "NoArgs",
            rt::class!(m, "NoArgs"(), no_args::body, line = 13)?,
        )?;
        m.set_global(
            "BadArgs",
            rt::class!(m, "BadArgs"(), bad_args::body, line = 17)?,
        )?;
        m.set_global(
            "NotStrings",
            rt::class!(
                m,
                "NotStrings"(),
                not_strings::body,
                line = 21,
                attributes = ["x"]
            )?,
        )?;
        m.set_global(
            "Dup",
            rt::class!(m, "Dup"(), dup::body, line = 28, attributes = ["x"])?,
        )?;
        rt::def!(m, not_a_class(value))?;
        rt::def!(m, positional(value))?;
        rt::def!(m, builtin_positional(value))?;
        rt::def!(m, bad_match_args(value))?;
        rt::def!(m, match_args_elements(value))?;
        rt::def!(m, multiple(value))?;
        rt::def!(m, duplicate_keys(value))?;
        m.set_global("Value", rt::class!(m, "Value"(), value::body, line = 79)?)?;
        m.set_global(
            "BadMapping",
            rt::class!(
                m,
                "BadMapping"(m.global("collections")?.attr("abc")?.attr("Mapping")?),
                bad_mapping::body,
                line = 83
            )?,
        )?;
        rt::def!(m, rest(value))?;
        rt::def!(m, failing_eq(value))?;
        m.set_global(
            "Explodes",
            rt::class!(m, "Explodes"(), explodes::body, line = 111)?,
        )?;
        rt::def!(m, unhashable(value))?;
        let value = PyList::empty(py);
        m.global("Value")?.set_attr("unhashable", value)?;
        attempt(
            m,
            &rt::str(py, "not a class:"),
            &m.global("not_a_class")?,
            &rt::int(py, 1),
        )?;
        attempt(
            m,
            &rt::str(py, "no __match_args__:"),
            &m.global("positional")?,
            &m.global("NoArgs")?.call0()?,
        )?;
        attempt(
            m,
            &rt::str(py, "builtin:"),
            &m.global("builtin_positional")?,
            &rt::int(py, 5),
        )?;
        attempt(
            m,
            &rt::str(py, "list __match_args__:"),
            &m.global("bad_match_args")?,
            &m.global("BadArgs")?.call0()?,
        )?;
        attempt(
            m,
            &rt::str(py, "non-str element:"),
            &m.global("match_args_elements")?,
            &m.global("NotStrings")?.call0()?,
        )?;
        attempt(
            m,
            &rt::str(py, "multiple:"),
            &m.global("multiple")?,
            &m.global("Dup")?.call0()?,
        )?;
        attempt(
            m,
            &rt::str(py, "duplicate key:"),
            &m.global("duplicate_keys")?,
            [(1, "a"), (2, "b")].into_py_dict(py)?.as_any(),
        )?;
        attempt(
            m,
            &rt::str(py, "rest:"),
            &m.global("rest")?,
            &m.global("BadMapping")?.call0()?,
        )?;
        attempt(
            m,
            &rt::str(py, "__eq__ raises:"),
            &m.global("failing_eq")?,
            PyList::new(py, [&m.global("Explodes")?.call0()?])?.as_any(),
        )?;
        attempt(
            m,
            &rt::str(py, "unhashable key:"),
            &m.global("unhashable")?,
            [("k", 1)].into_py_dict(py)?.as_any(),
        )?;
        // Errors raised by the guard propagate as usual.
        match (|| -> PyResult<()> {
            {
                let subject = PyList::new(py, [0])?.into_any();
                if let Some([x]) = rt::match_seq(&subject)? {
                    m.set_global("x", &x)?;
                    if rt::int(py, 1).div(m.global("x")?)?.is_truthy()? {}
                }
            }
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyZeroDivisionError>(py) => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                rt::builtins::print(py, ("guard:", m.global("err")?, "x is", m.global("x")?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        Ok(())
    })
}
