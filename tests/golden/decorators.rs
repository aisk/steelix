use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// decorators.py:6
pub fn trace<'py>(
    py: Python<'py>,
    globals: &Bound<'py, PyDict>,
    label: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::builtins::print(py, ("decorator expression", label))?;
    // decorators.py:9
    fn decorate<'py>(
        py: Python<'py>,
        globals: &Bound<'py, PyDict>,
        label: &Bound<'py, PyAny>,
        r#fn: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::builtins::print(py, ("apply", label, "to", r#fn.getattr("__name__")?))?;
        // decorators.py:13
        fn wrapper<'py>(
            py: Python<'py>,
            label: &Bound<'py, PyAny>,
            r#fn: &Bound<'py, PyAny>,
            args: &Bound<'py, PyTuple>,
            kwargs: &Bound<'py, PyDict>,
        ) -> PyResult<Bound<'py, PyAny>> {
            rt::builtins::print(py, ("enter", label))?;
            let result = rt::call!(r#fn, *args, **kwargs)?;
            rt::builtins::print(py, ("leave", label))?;
            Ok(result)
        }
        let decorator = rt::global(globals, "functools")?.call_method1("wraps", (r#fn,))?;
        let function = rt::function!(
            globals,
            wrapper[label, r#fn](*args, **kwargs),
            qualname = "trace.<locals>.decorate.<locals>.wrapper"
        )?;
        let wrapper = decorator.call1((&function,))?;
        Ok(wrapper)
    }
    let decorate = rt::function!(
        globals,
        decorate[label](globals, r#fn),
        qualname = "trace.<locals>.decorate"
    )?;
    Ok(decorate)
}

// decorators.py:24
pub fn default_value<'py>(
    py: Python<'py>,
    label: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::builtins::print(py, ("default", label))?;
    Ok(label.clone())
}

// decorators.py:31
/// Say hello.
pub fn greet<'py>(
    py: Python<'py>,
    name: &Bound<'py, PyAny>,
    punct: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::str(py, "hello ").add(name)?.add(punct)
}

// decorators.py:36
pub fn register<'py>(
    py: Python<'py>,
    globals: &Bound<'py, PyDict>,
    r#fn: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::global(globals, "registry")?
        .getattr("append")?
        .call1((r#fn.getattr("__name__")?,))?;
    Ok(r#fn.clone())
}

// decorators.py:45
pub fn first<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::int(py, 1))
}

// decorators.py:50
pub fn second<'py>(py: Python<'py>, globals: &Bound<'py, PyDict>) -> PyResult<Bound<'py, PyAny>> {
    rt::global(globals, "first")?.call0()?.add(1)
}

// decorators.py:55
pub fn fib<'py>(
    py: Python<'py>,
    globals: &Bound<'py, PyDict>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    if n.lt(2)? {
        return Ok(n.clone());
    }
    rt::global(globals, "fib")?
        .call1((n.sub(1)?,))?
        .add(rt::global(globals, "fib")?.call1((n.sub(2)?,))?)
}

// decorators.py:61
pub fn replace<'py>(py: Python<'py>, r#fn: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::str(py, "not a function any more"))
}

// decorators.py:66
pub fn gone<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::none(py))
}

/// Executes the body of `decorators.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    globals.set_item(
        "__doc__",
        "Decorators: evaluation and application order, functools helpers.",
    )?;
    globals.set_item("functools", rt::import(globals, "functools")?)?;
    rt::def!(globals, trace(globals, label))?;
    rt::def!(globals, default_value(label))?;
    let decorator = trace(py, globals, &rt::str(py, "outer"))?;
    let decorator2 = trace(py, globals, &rt::str(py, "inner"))?;
    let function = rt::function!(
        globals,
        greet(name, punct = default_value(py, &rt::str(py, "!"))?),
        doc = "Say hello."
    )?;
    let function = decorator2.call1((&function,))?;
    globals.set_item("greet", decorator.call1((&function,))?)?;
    rt::def!(globals, register(globals, r#fn))?;
    globals.set_item("registry", PyList::empty(py))?;
    let decorator3 = rt::global(globals, "register")?;
    let function2 = rt::function!(globals, first())?;
    globals.set_item("first", decorator3.call1((&function2,))?)?;
    let decorator4 = rt::global(globals, "register")?;
    let function3 = rt::function!(globals, second(globals))?;
    globals.set_item("second", decorator4.call1((&function3,))?)?;
    let decorator5 = rt::call!(
        rt::global(globals, "functools")?.getattr("lru_cache")?,
        maxsize = py.None()
    )?;
    let function4 = rt::function!(globals, fib(globals, n))?;
    globals.set_item("fib", decorator5.call1((&function4,))?)?;
    rt::def!(globals, replace(r#fn))?;
    let decorator6 = rt::global(globals, "replace")?;
    let function5 = rt::function!(globals, gone())?;
    globals.set_item("gone", decorator6.call1((&function5,))?)?;
    rt::builtins::print(py, (rt::global(globals, "greet")?.call1(("world",))?,))?;
    rt::builtins::print(
        py,
        (
            rt::global(globals, "greet")?.getattr("__name__")?,
            rt::global(globals, "greet")?.getattr("__qualname__")?,
            rt::global(globals, "greet")?.getattr("__doc__")?,
            rt::global(globals, "greet")?
                .getattr("__wrapped__")?
                .getattr("__name__")?,
        ),
    )?;
    rt::builtins::print(
        py,
        (rt::global(globals, "greet")?
            .getattr("__wrapped__")?
            .call_method1("__wrapped__", ("x", "?"))?,),
    )?;
    rt::builtins::print(
        py,
        (
            rt::global(globals, "registry")?,
            rt::global(globals, "second")?.call0()?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            rt::global(globals, "fib")?.call1((80,))?,
            rt::global(globals, "fib")?
                .call_method0("cache_info")?
                .getattr("hits")?,
        ),
    )?;
    rt::builtins::print(py, (rt::global(globals, "gone")?,))?;
    Ok(())
}
