use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// decorators.py:6
pub fn trace<'py>(m: &rt::Module<'py>, label: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "trace", 6, |_| {
        rt::builtins::print(m.py(), ("decorator expression", label))?;
        // decorators.py:9
        fn decorate<'py>(
            m: &rt::Module<'py>,
            label: &Bound<'py, PyAny>,
            r#fn: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "trace.<locals>.decorate", 9, |_| {
                rt::builtins::print(m.py(), ("apply", label, "to", r#fn.attr("__name__")?))?;
                // decorators.py:13
                fn wrapper<'py>(
                    m: &rt::Module<'py>,
                    label: &Bound<'py, PyAny>,
                    r#fn: &Bound<'py, PyAny>,
                    args: &Bound<'py, PyTuple>,
                    kwargs: &Bound<'py, PyDict>,
                ) -> PyResult<Bound<'py, PyAny>> {
                    let py = m.py();
                    rt::frame!(m, "trace.<locals>.decorate.<locals>.wrapper", 12, |_| {
                        rt::builtins::print(py, ("enter", label))?;
                        let result = rt::call!(r#fn, *args, **kwargs)?;
                        rt::builtins::print(py, ("leave", label))?;
                        Ok(result)
                    })
                }
                let wrapper = rt::function!(
                    m,
                    @(m.global("functools")?.method1("wraps", (r#fn,))?)
                    wrapper[label, r#fn](*args, **kwargs),
                    qualname = "trace.<locals>.decorate.<locals>.wrapper"
                )?;
                Ok(wrapper)
            })
        }
        let decorate = rt::function!(
            m,
            decorate[label](r#fn),
            qualname = "trace.<locals>.decorate"
        )?;
        Ok(decorate)
    })
}

// decorators.py:24
pub fn default_value<'py>(
    m: &rt::Module<'py>,
    label: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "default_value", 24, |_| {
        rt::builtins::print(m.py(), ("default", label))?;
        Ok(label.clone())
    })
}

// decorators.py:31
/// Say hello.
pub fn greet<'py>(
    m: &rt::Module<'py>,
    name: &Bound<'py, PyAny>,
    punct: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "greet", 29, |_| {
        rt::str(m.py(), "hello ").add(name)?.add(punct)
    })
}

// decorators.py:36
pub fn register<'py>(m: &rt::Module<'py>, r#fn: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "register", 36, |_| {
        m.global("registry")?
            .attr("append")?
            .call1((r#fn.attr("__name__")?,))?;
        Ok(r#fn.clone())
    })
}

// decorators.py:45
pub fn first<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::int(m.py(), 1))
}

// decorators.py:50
pub fn second<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "second", 49, |_| { m.global("first")?.call0()?.add(1) })
}

// decorators.py:55
pub fn fib<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "fib", 54, |_| {
        if n.lt(2)? {
            return Ok(n.clone());
        }
        m.global("fib")?
            .call1((n.sub(1)?,))?
            .add(m.global("fib")?.call1((n.sub(2)?,))?)
    })
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
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global(
            "__doc__",
            "Decorators: evaluation and application order, functools helpers.",
        )?;
        m.set_global("functools", rt::import(m, "functools")?)?;
        rt::def!(m, trace(label))?;
        rt::def!(m, default_value(label))?;
        rt::def!(
            m,
            @(trace(m, &rt::str(py, "outer"))?)
            @(trace(m, &rt::str(py, "inner"))?)
            greet(name, punct = default_value(m, &rt::str(py, "!"))?),
            doc = "Say hello."
        )?;
        rt::def!(m, register(r#fn))?;
        m.set_global("registry", PyList::empty(py))?;
        rt::def!(m, @register first())?;
        rt::def!(m, @register second())?;
        rt::def!(
            m,
            @(rt::call!(
                m.global("functools")?.attr("lru_cache")?,
                maxsize = py.None()
            )?)
            fib(n)
        )?;
        rt::def!(m, replace(r#fn))?;
        rt::def!(m, @replace gone())?;
        rt::builtins::print(py, (m.global("greet")?.call1(("world",))?,))?;
        rt::builtins::print(
            py,
            (
                m.global("greet")?.attr("__name__")?,
                m.global("greet")?.attr("__qualname__")?,
                m.global("greet")?.attr("__doc__")?,
                m.global("greet")?.attr("__wrapped__")?.attr("__name__")?,
            ),
        )?;
        rt::builtins::print(
            py,
            (m.global("greet")?
                .attr("__wrapped__")?
                .method1("__wrapped__", ("x", "?"))?,),
        )?;
        rt::builtins::print(py, (m.global("registry")?, m.global("second")?.call0()?))?;
        rt::builtins::print(
            py,
            (
                m.global("fib")?.call1((80,))?,
                m.global("fib")?.method0("cache_info")?.attr("hits")?,
            ),
        )?;
        rt::builtins::print(py, (m.global("gone")?,))?;
        Ok(())
    })
}
