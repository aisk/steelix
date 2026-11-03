use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// `*` and `**` in list, tuple, set and dict displays.
// displays_unpacking.py:4
pub fn trace<'py>(
    m: &rt::Module<'py>,
    label: &Bound<'py, PyAny>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "trace", 4, |_| {
        rt::builtins::print(m.py(), ("eval", label))?;
        Ok(value.clone())
    })
}

// displays_unpacking.py:46
pub fn lists<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "lists", 46, |_| {
        let first = rt::list!(py, *a, b)?.into_any();
        let second = rt::list!(
            py,
            trace(m, &rt::str(py, "x"), &rt::int(py, 0))?,
            *trace(m, &rt::str(py, "a"), &m.global("Noisy")?.call1(("a", a))?)?,
            trace(m, &rt::str(py, "b"), b)?,
            *m.global("Noisy")?.call1(("c", "cd"))?
        )?
        .into_any();
        let nested = rt::list!(
            py,
            *rt::list!(py, *a)?,
            *PyTuple::empty(py),
            *rt::builtin(py, "range")?.call1((2,))?
        )?
        .into_any();
        // The comprehension needs statements first: they run after `Noisy` is iterated.
        let lazy = rt::list!(
            py,
            *m.global("Noisy")?.call1(("g", "g"))?,
            {
                let list = PyList::empty(py);
                for i in rt::builtin(py, "range")?.call1((1,))?.try_iter()? {
                    let i = i?;
                    list.append(trace(m, &rt::str(py, "comp"), &i)?)?;
                    py.check_signals()?;
                }
                list.into_any()
            },
            *{
                let list2 = PyList::empty(py);
                for i in a.try_iter()? {
                    let i = i?;
                    list2.append(&i)?;
                    py.check_signals()?;
                }
                list2.into_any()
            }
        )?
        .into_any();
        rt::builtins::print(py, (&first, &second, &nested, &lazy))?;
        Ok(rt::list!(py, *a)?.into_any())
    })
}

// displays_unpacking.py:56
pub fn tuples<'py>(m: &rt::Module<'py>, a: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "tuples", 56, |_| {
        let one = rt::tuple!(py, *a)?.into_any();
        rt::builtins::print(
            py,
            (
                &one,
                rt::tuple!(py, *one, *a, 9)?,
                one.get_type().attr("__name__")?,
            ),
        )?;
        Ok(rt::none(py))
    })
}

// displays_unpacking.py:61
pub fn sets<'py>(m: &rt::Module<'py>, a: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "sets", 61, |_| {
        let s = rt::set!(
            py,
            trace(m, &rt::str(py, "k1"), &m.global("Key")?.call1(("k1",))?)?,
            trace(m, &rt::str(py, "k2"), &m.global("Key")?.call1(("k2",))?)?,
            *trace(
                m,
                &rt::str(py, "a"),
                &m.global("Noisy")?
                    .call1(("a", PyList::new(py, [&m.global("Key")?.call1(("k3",))?])?))?
            )?
        )?
        .into_any();
        // displays_unpacking.py:63
        fn genexpr<'py>(
            m: &rt::Module<'py>,
            s: &Bound<'py, PyIterator>,
        ) -> PyResult<Option<Bound<'py, PyAny>>> {
            rt::frame!(m, "sets.<locals>.<genexpr>", 63, |_| {
                if let Some(k) = rt::next(s)? {
                    return Ok(Some(k.attr("name")?));
                }
                Ok(None)
            })
        }
        rt::builtins::print(
            py,
            (
                rt::builtin(py, "sorted")?.call1((rt::genexpr!(
                    m,
                    genexpr(s.try_iter()?),
                    qualname = "sets.<locals>.<genexpr>"
                )?,))?,
                rt::set!(py, *a)?
                    .rich_compare(rt::builtin(py, "set")?.call1((a,))?, CompareOp::Eq)?,
                rt::set!(py, 1, *a)?,
            ),
        )?;
        Ok(rt::none(py))
    })
}

// displays_unpacking.py:66
pub fn dicts<'py>(
    m: &rt::Module<'py>,
    d: &Bound<'py, PyAny>,
    v: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "dicts", 66, |_| {
        let merged = rt::dict!(py, **d, ("k", v))?.into_any();
        let order = rt::dict!(
            py,
            ("a", 1),
            ("b", 2),
            **[("a", 3), ("c", 4)].into_py_dict(py)?,
            ("b", 5)
        )?
        .into_any();
        let keys = rt::dict!(
            py,
            (
                trace(m, &rt::str(py, "k1"), &m.global("Key")?.call1(("k1",))?)?,
                trace(m, &rt::str(py, "v1"), &rt::int(py, 1))?
            ),
            (
                trace(m, &rt::str(py, "k2"), &m.global("Key")?.call1(("k2",))?)?,
                2
            ),
            **d
        )?
        .into_any();
        rt::builtins::print(
            py,
            (
                &merged,
                &order,
                rt::builtin(py, "list")?.call1((&order,))?,
                &keys,
            ),
        )?;
        rt::builtins::print(
            py,
            (
                rt::dict!(py, **m.global("Mapping")?.call0()?, ("x", 1))?,
                rt::dict!(py, **PyDict::new(py))?,
                rt::dict!(py, **d, **d)?,
            ),
        )?;
        Ok(rt::none(py))
    })
}

// displays_unpacking.py:74
pub fn errors<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "errors", 74, |frame| {
        // displays_unpacking.py:76
        fn lambda<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
            let py = m.py();
            rt::frame!(m, "errors.<locals>.<lambda>", 76, |_| {
                Ok(rt::list!(py, *rt::int(py, 1))?.into_any())
            })
        }
        // displays_unpacking.py:77
        fn lambda2<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
            let py = m.py();
            rt::frame!(m, "errors.<locals>.<lambda>", 77, |_| {
                Ok(rt::tuple!(py, *rt::none(py))?.into_any())
            })
        }
        // displays_unpacking.py:78
        fn lambda3<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
            let py = m.py();
            rt::frame!(m, "errors.<locals>.<lambda>", 78, |_| {
                Ok(rt::set!(py, *rt::float(py, 3.5))?.into_any())
            })
        }
        // displays_unpacking.py:79
        fn lambda4<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
            let py = m.py();
            rt::frame!(m, "errors.<locals>.<lambda>", 79, |_| {
                Ok(rt::dict!(py, **rt::int(py, 1))?.into_any())
            })
        }
        // displays_unpacking.py:80
        fn lambda5<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
            let py = m.py();
            rt::frame!(m, "errors.<locals>.<lambda>", 80, |_| {
                Ok(rt::dict!(
                    py,
                    ("a", 1),
                    **PyList::new(
                        py,
                        [PyTuple::new(py, [&rt::str(py, "b"), &rt::int(py, 2)])?.as_any()]
                    )?
                )?
                .into_any())
            })
        }
        // displays_unpacking.py:81
        fn lambda6<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "errors.<locals>.<lambda>", 81, |_| {
                Ok(rt::list!(m.py(), 0, *m.global("Noisy")?.call1(("bad", 5))?)?.into_any())
            })
        }
        for build in PyTuple::new(
            py,
            [
                &rt::function!(m, lambda(), qualname = "errors.<locals>.<lambda>")?,
                &rt::function!(m, lambda2(), qualname = "errors.<locals>.<lambda>")?,
                &rt::function!(m, lambda3(), qualname = "errors.<locals>.<lambda>")?,
                &rt::function!(m, lambda4(), qualname = "errors.<locals>.<lambda>")?,
                &rt::function!(m, lambda5(), qualname = "errors.<locals>.<lambda>")?,
                &rt::function!(m, lambda6(), qualname = "errors.<locals>.<lambda>")?,
            ],
        )?
        .try_iter()?
        {
            let build = build?;
            match build.call0() {
                Ok(_) => {}
                Err(err) if err.is_instance_of::<PyTypeError>(py) => {
                    let e = frame.catch(err);
                    let _handling = rt::Handling::new(&e);
                    rt::builtins::print(py, (e.get_type().attr("__name__")?, &e))?;
                }
                Err(err) => return Err(err),
            }
            py.check_signals()?;
        }
        Ok(rt::none(py))
    })
}

/// An iterable that reports when it is iterated.
pub mod noisy {
    use super::*;

    // displays_unpacking.py:9
    /// The body of `class Noisy`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Noisy", 9, |_| {
            ns.set("__doc__", "An iterable that reports when it is iterated.")?;
            ns.set(
                "__init__",
                rt::function!(
                    ns,
                    __init__(slf as "self", label, items),
                    qualname = "Noisy.__init__"
                )?,
            )?;
            ns.set(
                "__iter__",
                rt::function!(
                    ns,
                    __iter__(slf as "self"),
                    qualname = "Noisy.__iter__"
                )?,
            )?;
            Ok(())
        })
    }

    // displays_unpacking.py:12
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        label: &Bound<'py, PyAny>,
        items: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Noisy.__init__", 12, |_| {
            slf.set_attr("label", label)?;
            slf.set_attr("items", items)?;
            Ok(rt::none(m.py()))
        })
    }

    // displays_unpacking.py:16
    pub fn __iter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Noisy.__iter__", 16, |_| {
            rt::builtins::print(m.py(), ("iter", slf.attr("label")?))?;
            Ok(slf.attr("items")?.try_iter()?.into_any())
        })
    }
}

/// A key that reports when it is hashed.
pub mod key {
    use super::*;

    // displays_unpacking.py:21
    /// The body of `class Key`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Key", 21, |_| {
            ns.set("__doc__", "A key that reports when it is hashed.")?;
            ns.set(
                "__init__",
                rt::function!(
                    ns,
                    __init__(slf as "self", name),
                    qualname = "Key.__init__"
                )?,
            )?;
            ns.set(
                "__hash__",
                rt::function!(ns, __hash__(slf as "self"), qualname = "Key.__hash__")?,
            )?;
            ns.set(
                "__eq__",
                rt::function!(
                    ns,
                    __eq__(slf as "self", other),
                    qualname = "Key.__eq__"
                )?,
            )?;
            ns.set(
                "__repr__",
                rt::function!(ns, __repr__(slf as "self"), qualname = "Key.__repr__")?,
            )?;
            Ok(())
        })
    }

    // displays_unpacking.py:24
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Key.__init__", 24, |_| {
            slf.set_attr("name", name)?;
            Ok(rt::none(m.py()))
        })
    }

    // displays_unpacking.py:27
    pub fn __hash__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Key.__hash__", 27, |_| {
            rt::builtins::print(py, ("hash", slf.attr("name")?))?;
            rt::builtin(py, "hash")?.call1((slf.attr("name")?,))
        })
    }

    // displays_unpacking.py:31
    pub fn __eq__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        other: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Key.__eq__", 31, |_| {
            Ok(if other.is_instance(&m.global("Key")?)? {
                other
                    .attr("name")?
                    .rich_compare(slf.attr("name")?, CompareOp::Eq)?
            } else {
                rt::bool(m.py(), false)
            })
        })
    }

    // displays_unpacking.py:34
    pub fn __repr__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Key.__repr__", 34, |_| {
            rt::fstring(m.py(), ("Key(", slf.attr("name")?.repr()?, ")"))
        })
    }
}

pub mod mapping {
    use super::*;

    // displays_unpacking.py:38
    /// The body of `class Mapping`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Mapping", 38, |_| {
            ns.set(
                "keys",
                rt::function!(ns, keys(slf as "self"), qualname = "Mapping.keys")?,
            )?;
            ns.set(
                "__getitem__",
                rt::function!(
                    ns,
                    __getitem__(slf as "self", key),
                    qualname = "Mapping.__getitem__"
                )?,
            )?;
            Ok(())
        })
    }

    // displays_unpacking.py:39
    pub fn keys<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Mapping.keys", 39, |_| {
            Ok(PyList::new(m.py(), ["m1", "m2"])?.into_any())
        })
    }

    // displays_unpacking.py:42
    pub fn __getitem__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        key: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Mapping.__getitem__", 42, |_| { key.method0("upper") })
    }
}

/// Executes the body of `displays_unpacking.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        rt::def!(m, trace(label, value))?;
        m.set_global(
            "Noisy",
            rt::class!(
                m,
                "Noisy"(),
                noisy::body,
                line = 9,
                attributes = ["items", "label"]
            )?,
        )?;
        m.set_global(
            "Key",
            rt::class!(m, "Key"(), key::body, line = 21, attributes = ["name"])?,
        )?;
        m.set_global(
            "Mapping",
            rt::class!(m, "Mapping"(), mapping::body, line = 38)?,
        )?;
        rt::def!(m, lists(a, b))?;
        rt::def!(m, tuples(a))?;
        rt::def!(m, sets(a))?;
        rt::def!(m, dicts(d, v))?;
        rt::def!(m, errors())?;
        lists(m, PyList::new(py, [1, 2])?.as_any(), &rt::int(py, 3))?;
        tuples(m, &rt::str(py, "ab"))?;
        sets(m, PyList::new(py, [1, 2])?.as_any())?;
        dicts(m, [("x", 1)].into_py_dict(py)?.as_any(), &rt::int(py, 2))?;
        errors(m)?;
        rt::call!(
            rt::builtin(py, "print")?,
            *PyList::new(py, [1, 2])?,
            *rt::str(py, "ab"),
            sep = "-"
        )?;
        m.set_global(
            "values",
            rt::list!(
                py,
                *rt::builtin(py, "range")?.call1((3,))?,
                *rt::str(py, "xy")
            )?,
        )?;
        rt::builtins::print(
            py,
            (
                m.global("values")?,
                rt::tuple!(py, *m.global("values")?)?,
                rt::dict!(py, **[("m", 1)].into_py_dict(py)?)?,
            ),
        )?;
        Ok(())
    })
}
