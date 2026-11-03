use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// closures.py:4
pub fn make_counter<'py>(
    m: &rt::Module<'py>,
    start: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "make_counter", 4, |_| {
        let count = rt::Cell::new(py, "count", start)?;
        // closures.py:7
        fn increment<'py>(
            m: &rt::Module<'py>,
            count: &rt::Cell<'py>,
            step: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "make_counter.<locals>.increment", 7, |_| {
                count.set(rt::iadd(&count.get()?, step)?)?;
                count.get()
            })
        }
        let increment = rt::function!(
            m,
            increment[&count](step = 1),
            qualname = "make_counter.<locals>.increment"
        )?;
        // closures.py:12
        fn peek<'py>(m: &rt::Module<'py>, count: &rt::Cell<'py>) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "make_counter.<locals>.peek", 12, |_| { count.get() })
        }
        let peek = rt::function!(m, peek[&count](), qualname = "make_counter.<locals>.peek")?;
        Ok(PyTuple::new(py, [&increment, &peek])?.into_any())
    })
}

// closures.py:18
pub fn late_binding<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "late_binding", 18, |_| {
        let i = rt::Cell::empty(py, "i")?;
        let mut show: Bound<'py, PyAny>;
        // Every closure sees the final value of `i`.
        let funcs = PyList::empty(py).into_any();
        for item in rt::builtin(py, "range")?.call1((3,))?.try_iter()? {
            i.set(item?)?;
            // closures.py:23
            fn show_<'py>(m: &rt::Module<'py>, i: &rt::Cell<'py>) -> PyResult<Bound<'py, PyAny>> {
                rt::frame!(m, "late_binding.<locals>.show", 23, |_| { i.get() })
            }
            show = rt::function!(m, show_[&i](), qualname = "late_binding.<locals>.show")?;
            funcs.call_method1("append", (&show,))?;
            py.check_signals()?;
        }
        Ok(PyList::new(
            py,
            [
                &funcs.get_item(0)?.call0()?,
                &funcs.get_item(1)?.call0()?,
                &funcs.get_item(2)?.call0()?,
            ],
        )?
        .into_any())
    })
}

// closures.py:30
pub fn early_binding<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "early_binding", 30, |_| {
        // A default argument captures the value at definition time instead.
        let funcs = PyList::empty(py).into_any();
        for i in rt::builtin(py, "range")?.call1((3,))?.try_iter()? {
            let i = i?;
            // closures.py:34
            fn lambda<'py>(
                m: &rt::Module<'py>,
                i: &Bound<'py, PyAny>,
            ) -> PyResult<Bound<'py, PyAny>> {
                rt::frame!(m, "early_binding.<locals>.<lambda>", 34, |_| { i.mul(10) })
            }
            funcs.getattr("append")?.call1((rt::function!(
                m,
                lambda(i = &i),
                qualname = "early_binding.<locals>.<lambda>"
            )?,))?;
            py.check_signals()?;
        }
        Ok(PyList::new(
            py,
            [
                &funcs.get_item(0)?.call0()?,
                &funcs.get_item(1)?.call0()?,
                &funcs.get_item(2)?.call0()?,
            ],
        )?
        .into_any())
    })
}

// closures.py:38
pub fn factorial<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "factorial", 38, |_| {
        let go = rt::Cell::empty(m.py(), "go")?;
        // closures.py:39
        fn go_<'py>(
            m: &rt::Module<'py>,
            go: &rt::Cell<'py>,
            k: &Bound<'py, PyAny>,
            acc: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "factorial.<locals>.go", 39, |_| {
                if k.le(1)? {
                    return Ok(acc.clone());
                }
                go.get()?.call1((k.sub(1)?, acc.mul(k)?))
            })
        }
        go.set(rt::function!(
            m,
            go_[&go](k, acc),
            qualname = "factorial.<locals>.go"
        )?)?;
        go.get()?.call1((n, 1))
    })
}

// closures.py:47
pub fn deep<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "deep", 47, |_| {
        let a = rt::Cell::new(m.py(), "a", 1)?;
        // closures.py:50
        fn middle<'py>(m: &rt::Module<'py>, a: &rt::Cell<'py>) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "deep.<locals>.middle", 50, |_| {
                let b = rt::int(m.py(), 2);
                // closures.py:53
                fn inner<'py>(
                    m: &rt::Module<'py>,
                    a: &rt::Cell<'py>,
                    b: &Bound<'py, PyAny>,
                ) -> PyResult<Bound<'py, PyAny>> {
                    rt::frame!(m, "deep.<locals>.middle.<locals>.inner", 53, |_| {
                        a.set(rt::iadd(&a.get()?, b)?)?;
                        a.get()
                    })
                }
                let inner = rt::function!(
                    m,
                    inner[&a, b](),
                    qualname = "deep.<locals>.middle.<locals>.inner"
                )?;
                Ok(inner)
            })
        }
        let middle = rt::function!(m, middle[&a](), qualname = "deep.<locals>.middle")?;
        middle.call0()
    })
}

// closures.py:63
pub fn unassigned<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "unassigned", 63, |_| {
        let value = rt::Cell::empty(m.py(), "value")?;
        // closures.py:64
        fn read<'py>(m: &rt::Module<'py>, value: &rt::Cell<'py>) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "unassigned.<locals>.read", 64, |_| { value.get() })
        }
        let read = rt::function!(m, read[&value](), qualname = "unassigned.<locals>.read")?;
        let try_first = read.clone();
        value.set("now set")?;
        try_first.call0()
    })
}

// closures.py:72
pub fn too_early<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "too_early", 72, |_| {
        let secret = rt::Cell::empty(m.py(), "secret")?;
        // closures.py:73
        fn read<'py>(m: &rt::Module<'py>, secret: &rt::Cell<'py>) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "too_early.<locals>.read", 73, |_| { secret.get() })
        }
        let read = rt::function!(m, read[&secret](), qualname = "too_early.<locals>.read")?;
        let result = read.call0()?;
        secret.set(1)?;
        Ok(result)
    })
}

/// Executes the body of `closures.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global("__doc__", "Nested functions, closures and `nonlocal`.")?;
        rt::def!(m, make_counter(start = 0))?;
        rt::def!(m, late_binding())?;
        rt::def!(m, early_binding())?;
        rt::def!(m, factorial(n))?;
        rt::def!(m, deep())?;
        rt::def!(m, unassigned())?;
        rt::def!(m, too_early())?;
        let [increment, peek] = rt::unpack(&make_counter(m, &rt::int(py, 10))?)?;
        m.set_global("increment", increment)?;
        m.set_global("peek", peek)?;
        rt::builtins::print(
            py,
            (
                m.global("increment")?.call0()?,
                m.global("increment")?.call1((5,))?,
                m.global("peek")?.call0()?,
            ),
        )?;
        let [other, underscore] = rt::unpack(&make_counter(m, &rt::int(py, 0))?)?;
        m.set_global("other", other)?;
        m.set_global("_", underscore)?;
        rt::builtins::print(
            py,
            (
                m.global("other")?.call0()?,
                m.global("increment")?.call0()?,
                m.global("peek")?.call0()?,
            ),
        )?;
        rt::builtins::print(py, (late_binding(m)?, early_binding(m)?))?;
        rt::builtins::print(py, (factorial(m, &rt::int(py, 10))?,))?;
        m.set_global("inner", deep(m)?)?;
        rt::builtins::print(
            py,
            (m.global("inner")?.call0()?, m.global("inner")?.call0()?),
        )?;
        rt::builtins::print(py, (unassigned(m)?,))?;
        rt::builtins::print(
            py,
            (
                m.global("increment")?.getattr("__qualname__")?,
                m.global("increment")?.getattr("__name__")?,
                !m.global("peek")?.getattr("__closure__")?.is_none(),
            ),
        )?;
        rt::builtins::print(
            py,
            (
                m.global("increment")?
                    .getattr("__closure__")?
                    .get_item(0)?
                    .get_type()
                    .getattr("__name__")?,
                m.global("increment")?
                    .getattr("__closure__")?
                    .get_item(0)?
                    .getattr("cell_contents")?,
            ),
        )?;
        too_early(m)?;
        Ok(())
    })
}
