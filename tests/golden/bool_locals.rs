use pyo3::exceptions::PyUnboundLocalError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::IntoPyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// bool_locals.py:26
pub fn find<'py>(
    m: &rt::Module<'py>,
    items: &Bound<'py, PyAny>,
    target: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "find", 26, |_| {
        let mut found = false;
        for item in items.try_iter()? {
            let item = item?;
            if item.is(target) || PyTuple::new(py, [target])?.contains(&item)? {
                found = true;
                break;
            }
            py.check_signals()?;
        }
        if !found {
            rt::builtins::print(py, ("not found", target))?;
        }
        Ok(rt::bool(py, found))
    })
}

// bool_locals.py:37
pub fn flags<'py>(
    m: &rt::Module<'py>,
    x: &Bound<'py, PyAny>,
    y: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "flags", 37, |_| {
        let a = x.is_instance(&rt::builtin(py, "int")?)?;
        let b = !y.is_truthy()?;
        let c = a && b;
        let d = a || b || y.is_truthy()?;
        let e = x.is_none();
        let f = !x.is_none();
        let g = if e { c } else { d };
        let h = m.global("Weird")?.call1(("container",))?.contains("a")?;
        rt::builtins::print(py, (a, b, c, d, e, f, g, h))?;
        rt::builtins::print(
            py,
            (
                PyList::new(py, [a, b])?,
                [("c", c)].into_py_dict(py)?,
                PyTuple::new(py, [d])?,
                rt::fstring(
                    py,
                    (
                        rt::bool(py, e).repr()?,
                        " ",
                        rt::format(&rt::bool(py, f), "")?,
                    ),
                )?,
                rt::bool(py, g).get_type().attr("__name__")?,
                rt::bool(py, h).rich_compare(1, CompareOp::Eq)?,
            ),
        )?;
        Ok(PyTuple::new(py, [a, b, c, d, e, f, g, h])?.into_any())
    })
}

// bool_locals.py:51
pub fn not_bools<'py>(m: &rt::Module<'py>, w: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "not_bools", 51, |_| {
        // The value of a rich comparison is whatever `__eq__` returned.
        let eq = w.rich_compare(1, CompareOp::Eq)?;
        let lt = w.rich_compare(2, CompareOp::Lt)?;
        rt::builtins::print(py, (&eq, &lt))?;
        // A truth test of a non-bool object calls `__bool__` where Python does.
        let as_bool = w.is_truthy()?;
        let negated = !w.is_truthy()?;
        rt::builtins::print(py, (as_bool, negated))?;
        // Assigned something else too: stays an object.
        let mut mixed = rt::bool(py, false);
        if as_bool {
            mixed = w.clone();
        }
        rt::builtins::print(
            py,
            (if mixed.is_truthy()? {
                mixed.attr("name")?
            } else {
                mixed.clone()
            },),
        )?;
        Ok(rt::none(py))
    })
}

// bool_locals.py:67
pub fn shadowed<'py>(
    m: &rt::Module<'py>,
    isinstance: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "shadowed", 67, |_| {
        // Not the builtin: the result is whatever the call returns.
        let r = isinstance.call1((1, rt::builtin(m.py(), "int")?))?;
        Ok(r)
    })
}

// bool_locals.py:73
pub fn flip<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "flip", 73, |_| {
        let mut n = n.clone();
        let mut on = true;
        let mut count = rt::int(py, 0);
        while n.gt(0)? {
            on = !on;
            if on {
                count = rt::iadd(&count, 1)?;
            }
            n = rt::isub(&n, 1)?;
            py.check_signals()?;
        }
        rt::builtins::print(py, ("flipped", on, &count))?;
        let done = rt::bool(py, on).is(rt::bool(py, true));
        Ok(rt::bool(py, done))
    })
}

// bool_locals.py:86
pub fn captured<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "captured", 86, |_| {
        // Captured by a nested function: stays a Python object in a cell.
        let seen = rt::Cell::new(m.py(), "seen", false)?;
        // bool_locals.py:90
        fn mark<'py>(m: &rt::Module<'py>, seen: &rt::Cell<'py>) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "captured.<locals>.mark", 90, |_| {
                seen.set(true)?;
                Ok(rt::none(m.py()))
            })
        }
        let mark = rt::function!(m, mark[&seen](), qualname = "captured.<locals>.mark")?;
        mark.call0()?;
        seen.get()
    })
}

// bool_locals.py:98
pub fn maybe_unbound<'py>(
    m: &rt::Module<'py>,
    flag: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "maybe_unbound", 98, |frame| {
        let mut ok: Option<Bound<'py, PyAny>> = None;
        if flag.is_truthy()? {
            ok = Some(rt::bool(py, true));
        }
        match (|| -> PyResult<Bound<'py, PyAny>> { Ok(rt::local(&ok, "ok")?.clone()) })() {
            Ok(value) => Ok(value),
            Err(err) if err.is_instance_of::<PyUnboundLocalError>(py) => {
                let err = frame.catch(err);
                let _handling = rt::Handling::new(&err);
                rt::builtins::print(py, ("UnboundLocalError:", &err))?;
                Ok(rt::none(py))
            }
            Err(err2) => Err(err2),
        }
    })
}

// bool_locals.py:108
pub fn in_try<'py>(m: &rt::Module<'py>, values: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "in_try", 108, |frame| {
        let mut failed = false;
        let mut good: Option<Bound<'py, PyAny>> = None;
        let result = (|| -> PyResult<()> {
            match (|| -> PyResult<()> {
                rt::builtin(py, "int")?.call1((values.get_item(0)?,))?;
                good = Some(rt::bool(py, true));
                Ok(())
            })() {
                Ok(()) => {}
                Err(err) if err.is_instance_of::<PyValueError>(py) => {
                    failed = true;
                    good = Some(rt::bool(py, false));
                }
                Err(err) => return Err(err),
            }
            Ok(())
        })();
        let finally = frame.finally(result);
        rt::builtins::print(py, ("finally", failed))?;
        finally.resume()?;
        Ok(PyTuple::new(py, [rt::local(&good, "good")?, &rt::bool(py, failed)])?.into_any())
    })
}

// bool_locals.py:121
pub fn in_with<'py>(m: &rt::Module<'py>, lock: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "in_with", 121, |frame| {
        let mut held = false;
        frame.with(lock, |_| {
            held = true;
            Ok(())
        })?;
        Ok(rt::bool(m.py(), held))
    })
}

// bool_locals.py:128
pub fn redundant<'py>(m: &rt::Module<'py>, c: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "redundant", 128, |_| {
        // Python code the generated Rust keeps as it is (clippy is told so).
        let a = c.is_truthy()?;
        let b = !c.is_truthy()?;
        let done = false;
        let mut n = rt::int(py, 0);
        while !done {
            n = rt::iadd(&n, 1)?;
            if n.gt(2)? {
                break;
            }
            py.check_signals()?;
        }
        let same = a && a;
        let never = a && !a;
        Ok(PyTuple::new(
            py,
            [
                &rt::bool(py, a),
                &rt::bool(py, b),
                &rt::bool(py, same),
                &rt::bool(py, never),
                &n,
            ],
        )?
        .into_any())
    })
}

/// Comparisons that do not return bool, and a truth value that counts.
pub mod weird {
    use super::*;

    // bool_locals.py:4
    /// The body of `class Weird`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Weird", 4, |_| {
            ns.set(
                "__doc__",
                "Comparisons that do not return bool, and a truth value that counts.",
            )?;
            ns.set(
                "__init__",
                rt::function!(
                    ns,
                    __init__(slf as "self", name),
                    qualname = "Weird.__init__"
                )?,
            )?;
            ns.set(
                "__eq__",
                rt::function!(
                    ns,
                    __eq__(slf as "self", other),
                    qualname = "Weird.__eq__"
                )?,
            )?;
            ns.set(
                "__lt__",
                rt::function!(
                    ns,
                    __lt__(slf as "self", other),
                    qualname = "Weird.__lt__"
                )?,
            )?;
            ns.set(
                "__bool__",
                rt::function!(
                    ns,
                    __bool__(slf as "self"),
                    qualname = "Weird.__bool__"
                )?,
            )?;
            ns.set(
                "__contains__",
                rt::function!(
                    ns,
                    __contains__(slf as "self", item),
                    qualname = "Weird.__contains__"
                )?,
            )?;
            ns.set("__hash__", ns.get("object")?.attr("__hash__")?)?;
            Ok(())
        })
    }

    // bool_locals.py:7
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Weird.__init__", 7, |_| {
            slf.set_attr("name", name)?;
            Ok(rt::none(m.py()))
        })
    }

    // bool_locals.py:10
    pub fn __eq__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        other: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Weird.__eq__", 10, |_| {
            rt::fstring(
                m.py(),
                (
                    rt::format(&slf.attr("name")?, "")?,
                    " == ",
                    rt::format(other, "")?,
                ),
            )
        })
    }

    // bool_locals.py:13
    pub fn __lt__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        other: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Weird.__lt__", 13, |_| {
            Ok(PyList::new(m.py(), [&slf.attr("name")?, other])?.into_any())
        })
    }

    // bool_locals.py:16
    pub fn __bool__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Weird.__bool__", 16, |_| {
            rt::builtins::print(py, ("bool of", slf.attr("name")?))?;
            Ok(rt::bool(py, true))
        })
    }

    // bool_locals.py:20
    pub fn __contains__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        item: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // `in` turns this into a bool
        Ok(item.clone())
    }
}

pub mod lock {
    use super::*;

    // bool_locals.py:143
    /// The body of `class Lock`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Lock", 143, |_| {
            ns.set(
                "__enter__",
                rt::function!(
                    ns,
                    __enter__(slf as "self"),
                    qualname = "Lock.__enter__"
                )?,
            )?;
            ns.set(
                "__exit__",
                rt::function!(
                    ns,
                    __exit__(slf as "self", *exc),
                    qualname = "Lock.__exit__"
                )?,
            )?;
            Ok(())
        })
    }

    // bool_locals.py:144
    pub fn __enter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(slf.clone())
    }

    // bool_locals.py:147
    pub fn __exit__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        exc: &Bound<'py, PyTuple>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::bool(m.py(), false))
    }
}

/// Executes the body of `bool_locals.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global(
            "__doc__",
            "Locals that only ever hold a bool are Rust `bool`s (design §5).",
        )?;
        m.set_global(
            "Weird",
            rt::class!(m, "Weird"(), weird::body, line = 4, attributes = ["name"])?,
        )?;
        rt::def!(m, find(items, target))?;
        rt::def!(m, flags(x, y))?;
        rt::def!(m, not_bools(w))?;
        rt::def!(m, shadowed(isinstance))?;
        rt::def!(m, flip(n))?;
        rt::def!(m, captured())?;
        rt::def!(m, maybe_unbound(flag))?;
        rt::def!(m, in_try(values))?;
        rt::def!(m, in_with(lock))?;
        rt::def!(m, redundant(c))?;
        m.set_global("Lock", rt::class!(m, "Lock"(), lock::body, line = 143)?)?;
        rt::builtins::print(
            py,
            (
                find(m, PyList::new(py, [1, 2, 3])?.as_any(), &rt::int(py, 2))?,
                find(m, PyList::new(py, [1, 2, 3])?.as_any(), &rt::int(py, 5))?,
            ),
        )?;
        flags(m, &rt::int(py, 1), &rt::int(py, 0))?;
        flags(m, &rt::none(py), &rt::str(py, "y"))?;
        not_bools(m, &m.global("Weird")?.call1(("w",))?)?;
        // bool_locals.py:155
        fn lambda<'py>(
            m: &rt::Module<'py>,
            a: &Bound<'py, PyAny>,
            b: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            Ok(rt::str(m.py(), "called"))
        }
        rt::builtins::print(
            py,
            (shadowed(
                m,
                &rt::function!(m, lambda(a, b), qualname = "<lambda>")?,
            )?,),
        )?;
        rt::builtins::print(py, (flip(m, &rt::int(py, 3))?, flip(m, &rt::int(py, 0))?))?;
        rt::builtins::print(py, (captured(m)?,))?;
        rt::builtins::print(
            py,
            (
                maybe_unbound(m, &rt::bool(py, true))?,
                maybe_unbound(m, &rt::bool(py, false))?,
            ),
        )?;
        rt::builtins::print(
            py,
            (
                in_try(m, PyList::new(py, ["1"])?.as_any())?,
                in_try(m, PyList::new(py, ["x"])?.as_any())?,
            ),
        )?;
        rt::builtins::print(py, (in_with(m, &m.global("Lock")?.call0()?)?,))?;
        rt::builtins::print(
            py,
            (
                redundant(m, &rt::int(py, 1))?,
                redundant(m, &rt::int(py, 0))?,
            ),
        )?;
        Ok(())
    })
}
