use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// closures.py:4
#[rt::frame(m, "make_counter", 4, frame)]
pub fn make_counter<'py>(
    m: &rt::Module<'py>,
    start: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let count = rt::Cell::new(py, "count", start)?;
    // closures.py:7
    #[rt::frame(m, "make_counter.<locals>.increment", 7)]
    fn increment<'py>(
        m: &rt::Module<'py>,
        count: &rt::Cell<'py>,
        step: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        count.set(rt::iadd(&count.get()?, step)?)?;
        count.get()
    }
    let increment = rt::function!(frame, increment[&count](step = 1))?;
    // closures.py:12
    #[rt::frame(m, "make_counter.<locals>.peek", 12)]
    fn peek<'py>(m: &rt::Module<'py>, count: &rt::Cell<'py>) -> PyResult<Bound<'py, PyAny>> {
        count.get()
    }
    let peek = rt::function!(frame, peek[&count]())?;
    Ok(PyTuple::new(py, [&increment, &peek])?.into_any())
}

// closures.py:18
#[rt::frame(m, "late_binding", 18, frame)]
pub fn late_binding<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let i = rt::Cell::empty(py, "i")?;
    // Every closure sees the final value of `i`.
    let funcs = PyList::empty(py).into_any();
    let mut show: Bound<'py, PyAny>;
    for item in rt::iter(&rt::builtin(py, "range")?.call1((3,))?)? {
        i.set(item?)?;
        // closures.py:23
        #[rt::frame(m, "late_binding.<locals>.show", 23)]
        fn show_<'py>(m: &rt::Module<'py>, i: &rt::Cell<'py>) -> PyResult<Bound<'py, PyAny>> {
            i.get()
        }
        show = rt::function!(frame, show_ as "show"[&i]())?;
        funcs.method1("append", (&show,))?;
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
#[rt::frame(m, "early_binding", 30, frame)]
pub fn early_binding<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    // A default argument captures the value at definition time instead.
    let funcs = PyList::empty(py).into_any();
    for i in rt::iter(&rt::builtin(py, "range")?.call1((3,))?)? {
        let i = i?;
        // closures.py:34
        #[rt::frame(m, "early_binding.<locals>.<lambda>", 34)]
        fn lambda<'py>(m: &rt::Module<'py>, i: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            i.mul(10)
        }
        funcs
            .attr("append")?
            .call1((rt::function!(frame, lambda as "<lambda>"(i = &i))?,))?;
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
#[rt::frame(m, "factorial", 38, frame)]
pub fn factorial<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let go = rt::Cell::empty(m.py(), "go")?;
    // closures.py:39
    #[rt::frame(m, "factorial.<locals>.go", 39)]
    fn go_<'py>(
        m: &rt::Module<'py>,
        go: &rt::Cell<'py>,
        k: &Bound<'py, PyAny>,
        acc: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        if k.le(1)? {
            return Ok(acc.clone());
        }
        go.get()?.call1((k.sub(1)?, acc.mul(k)?))
    }
    go.set(rt::function!(frame, go_ as "go"[&go](k, acc))?)?;
    go.get()?.call1((n, 1))
}

// closures.py:47
#[rt::frame(m, "deep", 47, frame)]
pub fn deep<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let a = rt::Cell::new(m.py(), "a", 1)?;
    // closures.py:50
    #[rt::frame(m, "deep.<locals>.middle", 50, frame)]
    fn middle<'py>(m: &rt::Module<'py>, a: &rt::Cell<'py>) -> PyResult<Bound<'py, PyAny>> {
        let b = rt::int(m.py(), 2);
        // closures.py:53
        #[rt::frame(m, "deep.<locals>.middle.<locals>.inner", 53)]
        fn inner<'py>(
            m: &rt::Module<'py>,
            a: &rt::Cell<'py>,
            b: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            a.set(rt::iadd(&a.get()?, b)?)?;
            a.get()
        }
        let inner = rt::function!(frame, inner[&a, b]())?;
        Ok(inner)
    }
    let middle = rt::function!(frame, middle[&a]())?;
    middle.call0()
}

// closures.py:63
#[rt::frame(m, "unassigned", 63, frame)]
pub fn unassigned<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let value = rt::Cell::empty(m.py(), "value")?;
    // closures.py:64
    #[rt::frame(m, "unassigned.<locals>.read", 64)]
    fn read<'py>(m: &rt::Module<'py>, value: &rt::Cell<'py>) -> PyResult<Bound<'py, PyAny>> {
        value.get()
    }
    let read = rt::function!(frame, read[&value]())?;
    let try_first = read.clone();
    value.set("now set")?;
    try_first.call0()
}

// closures.py:72
#[rt::frame(m, "too_early", 72, frame)]
pub fn too_early<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let secret = rt::Cell::empty(m.py(), "secret")?;
    // closures.py:73
    #[rt::frame(m, "too_early.<locals>.read", 73)]
    fn read<'py>(m: &rt::Module<'py>, secret: &rt::Cell<'py>) -> PyResult<Bound<'py, PyAny>> {
        secret.get()
    }
    let read = rt::function!(frame, read[&secret]())?;
    let result = read.call0()?;
    secret.set(1)?;
    Ok(result)
}

/// Executes the body of `closures.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
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
            m.global("increment")?.attr("__qualname__")?,
            m.global("increment")?.attr("__name__")?,
            !m.global("peek")?.attr("__closure__")?.is_none(),
        ),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("increment")?
                .attr("__closure__")?
                .get_item(0)?
                .get_type()
                .attr("__name__")?,
            m.global("increment")?
                .attr("__closure__")?
                .get_item(0)?
                .attr("cell_contents")?,
        ),
    )?;
    too_early(m)?;
    Ok(())
}
