use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// decorators.py:6
pub fn trace<'py>(m: &rt::Module<'py>, label: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::builtins::print(m.py(), ("decorator expression", label))?;
    // decorators.py:9
    fn decorate<'py>(
        m: &rt::Module<'py>,
        label: &Bound<'py, PyAny>,
        r#fn: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::builtins::print(m.py(), ("apply", label, "to", r#fn.getattr("__name__")?))?;
        // decorators.py:13
        fn wrapper<'py>(
            m: &rt::Module<'py>,
            label: &Bound<'py, PyAny>,
            r#fn: &Bound<'py, PyAny>,
            args: &Bound<'py, PyTuple>,
            kwargs: &Bound<'py, PyDict>,
        ) -> PyResult<Bound<'py, PyAny>> {
            let py = m.py();
            rt::builtins::print(py, ("enter", label))?;
            let result = rt::call!(r#fn, *args, **kwargs)?;
            rt::builtins::print(py, ("leave", label))?;
            Ok(result)
        }
        let decorator = m.global("functools")?.call_method1("wraps", (r#fn,))?;
        let function = rt::function!(
            m,
            wrapper[label, r#fn](*args, **kwargs),
            qualname = "trace.<locals>.decorate.<locals>.wrapper"
        )?;
        let wrapper = decorator.call1((&function,))?;
        Ok(wrapper)
    }
    let decorate = rt::function!(
        m,
        decorate[label](r#fn),
        qualname = "trace.<locals>.decorate"
    )?;
    Ok(decorate)
}

// decorators.py:24
pub fn default_value<'py>(
    m: &rt::Module<'py>,
    label: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::builtins::print(m.py(), ("default", label))?;
    Ok(label.clone())
}

// decorators.py:31
/// Say hello.
pub fn greet<'py>(
    m: &rt::Module<'py>,
    name: &Bound<'py, PyAny>,
    punct: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::str(m.py(), "hello ").add(name)?.add(punct)
}

// decorators.py:36
pub fn register<'py>(m: &rt::Module<'py>, r#fn: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    m.global("registry")?
        .getattr("append")?
        .call1((r#fn.getattr("__name__")?,))?;
    Ok(r#fn.clone())
}

// decorators.py:45
pub fn first<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::int(m.py(), 1))
}

// decorators.py:50
pub fn second<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    m.global("first")?.call0()?.add(1)
}

// decorators.py:55
pub fn fib<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    if n.lt(2)? {
        return Ok(n.clone());
    }
    m.global("fib")?
        .call1((n.sub(1)?,))?
        .add(m.global("fib")?.call1((n.sub(2)?,))?)
}

// decorators.py:61
pub fn replace<'py>(m: &rt::Module<'py>, r#fn: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::str(m.py(), "not a function any more"))
}

// decorators.py:66
pub fn gone<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::none(m.py()))
}

/// Executes the body of `decorators.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "Decorators: evaluation and application order, functools helpers.",
    )?;
    m.set_global("functools", rt::import(m, "functools")?)?;
    rt::def!(m, trace(label))?;
    rt::def!(m, default_value(label))?;
    let decorator = trace(m, &rt::str(py, "outer"))?;
    let decorator2 = trace(m, &rt::str(py, "inner"))?;
    let function = rt::function!(
        m,
        greet(name, punct = default_value(m, &rt::str(py, "!"))?),
        doc = "Say hello."
    )?;
    let function = decorator2.call1((&function,))?;
    m.set_global("greet", decorator.call1((&function,))?)?;
    rt::def!(m, register(r#fn))?;
    m.set_global("registry", PyList::empty(py))?;
    let decorator3 = m.global("register")?;
    let function2 = rt::function!(m, first())?;
    m.set_global("first", decorator3.call1((&function2,))?)?;
    let decorator4 = m.global("register")?;
    let function3 = rt::function!(m, second())?;
    m.set_global("second", decorator4.call1((&function3,))?)?;
    let decorator5 = rt::call!(
        m.global("functools")?.getattr("lru_cache")?,
        maxsize = py.None()
    )?;
    let function4 = rt::function!(m, fib(n))?;
    m.set_global("fib", decorator5.call1((&function4,))?)?;
    rt::def!(m, replace(r#fn))?;
    let decorator6 = m.global("replace")?;
    let function5 = rt::function!(m, gone())?;
    m.set_global("gone", decorator6.call1((&function5,))?)?;
    rt::builtins::print(py, (m.global("greet")?.call1(("world",))?,))?;
    rt::builtins::print(
        py,
        (
            m.global("greet")?.getattr("__name__")?,
            m.global("greet")?.getattr("__qualname__")?,
            m.global("greet")?.getattr("__doc__")?,
            m.global("greet")?
                .getattr("__wrapped__")?
                .getattr("__name__")?,
        ),
    )?;
    rt::builtins::print(
        py,
        (m.global("greet")?
            .getattr("__wrapped__")?
            .call_method1("__wrapped__", ("x", "?"))?,),
    )?;
    rt::builtins::print(py, (m.global("registry")?, m.global("second")?.call0()?))?;
    rt::builtins::print(
        py,
        (
            m.global("fib")?.call1((80,))?,
            m.global("fib")?
                .call_method0("cache_info")?
                .getattr("hits")?,
        ),
    )?;
    rt::builtins::print(py, (m.global("gone")?,))?;
    Ok(())
}
