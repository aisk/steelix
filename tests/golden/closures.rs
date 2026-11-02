use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// closures.py:4
pub fn make_counter<'py>(
    py: Python<'py>,
    globals: &Bound<'py, PyDict>,
    start: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let count = rt::Cell::new(py, "count", start)?;
    // closures.py:7
    fn increment<'py>(
        py: Python<'py>,
        count: &rt::Cell<'py>,
        step: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        count.set(rt::iadd(&count.get()?, step)?)?;
        count.get()
    }
    let increment = rt::function!(
        globals,
        increment[&count](step = 1),
        qualname = "make_counter.<locals>.increment"
    )?;
    // closures.py:12
    fn peek<'py>(py: Python<'py>, count: &rt::Cell<'py>) -> PyResult<Bound<'py, PyAny>> {
        count.get()
    }
    let peek = rt::function!(
        globals,
        peek[&count](),
        qualname = "make_counter.<locals>.peek"
    )?;
    Ok(PyTuple::new(py, [&increment, &peek])?.into_any())
}

// closures.py:18
pub fn late_binding<'py>(
    py: Python<'py>,
    globals: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    let i = rt::Cell::empty(py, "i")?;
    let mut show: Bound<'py, PyAny>;
    // Every closure sees the final value of `i`.
    let funcs = PyList::empty(py).into_any();
    for item in rt::builtin(py, "range")?.call1((3,))?.try_iter()? {
        i.set(item?)?;
        // closures.py:23
        fn show_<'py>(py: Python<'py>, i: &rt::Cell<'py>) -> PyResult<Bound<'py, PyAny>> {
            i.get()
        }
        show = rt::function!(
            globals,
            show_[&i](),
            qualname = "late_binding.<locals>.show"
        )?;
        funcs.call_method1("append", (&show,))?;
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
}

// closures.py:30
pub fn early_binding<'py>(
    py: Python<'py>,
    globals: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    // A default argument captures the value at definition time instead.
    let funcs = PyList::empty(py).into_any();
    for i in rt::builtin(py, "range")?.call1((3,))?.try_iter()? {
        let i = i?;
        // closures.py:34
        fn lambda<'py>(py: Python<'py>, i: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            i.mul(10)
        }
        funcs.getattr("append")?.call1((rt::function!(
            globals,
            lambda(i = &i),
            qualname = "early_binding.<locals>.<lambda>"
        )?,))?;
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
}

// closures.py:38
pub fn factorial<'py>(
    py: Python<'py>,
    globals: &Bound<'py, PyDict>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let go = rt::Cell::empty(py, "go")?;
    // closures.py:39
    fn go_<'py>(
        py: Python<'py>,
        go: &rt::Cell<'py>,
        k: &Bound<'py, PyAny>,
        acc: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        if k.le(1)? {
            return Ok(acc.clone());
        }
        go.get()?.call1((k.sub(1)?, acc.mul(k)?))
    }
    go.set(rt::function!(
        globals,
        go_[&go](k, acc),
        qualname = "factorial.<locals>.go"
    )?)?;
    go.get()?.call1((n, 1))
}

// closures.py:47
pub fn deep<'py>(py: Python<'py>, globals: &Bound<'py, PyDict>) -> PyResult<Bound<'py, PyAny>> {
    let a = rt::Cell::new(py, "a", 1)?;
    // closures.py:50
    fn middle<'py>(
        py: Python<'py>,
        globals: &Bound<'py, PyDict>,
        a: &rt::Cell<'py>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let b = rt::int(py, 2);
        // closures.py:53
        fn inner<'py>(
            py: Python<'py>,
            a: &rt::Cell<'py>,
            b: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            a.set(rt::iadd(&a.get()?, b)?)?;
            a.get()
        }
        let inner = rt::function!(
            globals,
            inner[&a, b](),
            qualname = "deep.<locals>.middle.<locals>.inner"
        )?;
        Ok(inner)
    }
    let middle = rt::function!(
        globals,
        middle[&a](globals),
        qualname = "deep.<locals>.middle"
    )?;
    middle.call0()
}

// closures.py:63
pub fn unassigned<'py>(
    py: Python<'py>,
    globals: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    let value = rt::Cell::empty(py, "value")?;
    // closures.py:64
    fn read<'py>(py: Python<'py>, value: &rt::Cell<'py>) -> PyResult<Bound<'py, PyAny>> {
        value.get()
    }
    let read = rt::function!(
        globals,
        read[&value](),
        qualname = "unassigned.<locals>.read"
    )?;
    let try_first = read.clone();
    value.set("now set")?;
    try_first.call0()
}

// closures.py:72
pub fn too_early<'py>(
    py: Python<'py>,
    globals: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    let secret = rt::Cell::empty(py, "secret")?;
    // closures.py:73
    fn read<'py>(py: Python<'py>, secret: &rt::Cell<'py>) -> PyResult<Bound<'py, PyAny>> {
        secret.get()
    }
    let read = rt::function!(
        globals,
        read[&secret](),
        qualname = "too_early.<locals>.read"
    )?;
    let result = read.call0()?;
    secret.set(1)?;
    Ok(result)
}

/// Executes the body of `closures.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    globals.set_item("__doc__", "Nested functions, closures and `nonlocal`.")?;
    rt::def!(globals, make_counter(globals, start = 0))?;
    rt::def!(globals, late_binding(globals))?;
    rt::def!(globals, early_binding(globals))?;
    rt::def!(globals, factorial(globals, n))?;
    rt::def!(globals, deep(globals))?;
    rt::def!(globals, unassigned(globals))?;
    rt::def!(globals, too_early(globals))?;
    let [increment, peek] = rt::unpack(&make_counter(py, globals, &rt::int(py, 10))?)?;
    globals.set_item("increment", increment)?;
    globals.set_item("peek", peek)?;
    rt::builtins::print(
        py,
        (
            rt::global(globals, "increment")?.call0()?,
            rt::global(globals, "increment")?.call1((5,))?,
            rt::global(globals, "peek")?.call0()?,
        ),
    )?;
    let [other, __] = rt::unpack(&make_counter(
        py,
        globals,
        &rt::default(globals, "make_counter", "start")?,
    )?)?;
    globals.set_item("other", other)?;
    globals.set_item("_", __)?;
    rt::builtins::print(
        py,
        (
            rt::global(globals, "other")?.call0()?,
            rt::global(globals, "increment")?.call0()?,
            rt::global(globals, "peek")?.call0()?,
        ),
    )?;
    rt::builtins::print(
        py,
        (late_binding(py, globals)?, early_binding(py, globals)?),
    )?;
    rt::builtins::print(py, (factorial(py, globals, &rt::int(py, 10))?,))?;
    globals.set_item("inner", deep(py, globals)?)?;
    rt::builtins::print(
        py,
        (
            rt::global(globals, "inner")?.call0()?,
            rt::global(globals, "inner")?.call0()?,
        ),
    )?;
    rt::builtins::print(py, (unassigned(py, globals)?,))?;
    rt::builtins::print(
        py,
        (
            rt::global(globals, "increment")?.getattr("__qualname__")?,
            rt::global(globals, "increment")?.getattr("__name__")?,
            !rt::global(globals, "peek")?
                .getattr("__closure__")?
                .is_none(),
        ),
    )?;
    rt::builtins::print(
        py,
        (
            rt::global(globals, "increment")?
                .getattr("__closure__")?
                .get_item(0)?
                .get_type()
                .getattr("__name__")?,
            rt::global(globals, "increment")?
                .getattr("__closure__")?
                .get_item(0)?
                .getattr("cell_contents")?,
        ),
    )?;
    too_early(py, globals)?;
    Ok(())
}
