use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// decorators.py:6
pub fn trace<'py>(m: &rt::Module<'py>, label: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "trace", 6, |frame| {
        frame.line(7);
        rt::builtins::print(m.py(), ("decorator expression", label))?;
        // decorators.py:9
        fn decorate<'py>(
            m: &rt::Module<'py>,
            label: &Bound<'py, PyAny>,
            r#fn: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "trace.<locals>.decorate", 9, |frame| {
                frame.line(10);
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
                    rt::frame!(m, "trace.<locals>.decorate.<locals>.wrapper", 12, |frame| {
                        frame.line(14);
                        rt::builtins::print(py, ("enter", label))?;
                        frame.line(15);
                        let result = rt::call!(r#fn, *args, **kwargs)?;
                        frame.line(16);
                        rt::builtins::print(py, ("leave", label))?;
                        Ok(result)
                    })
                }
                frame.line(13);
                let wrapper = {
                    let decorator = m.global("functools")?.call_method1("wraps", (r#fn,))?;
                    let function = rt::function!(
                        m,
                        wrapper[label, r#fn](*args, **kwargs),
                        qualname = "trace.<locals>.decorate.<locals>.wrapper"
                    )?;
                    decorator.call1((function,))?
                };
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
    rt::frame!(m, "default_value", 24, |frame| {
        frame.line(25);
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
    rt::frame!(m, "greet", 29, |frame| {
        frame.line(33);
        rt::str(m.py(), "hello ").add(name)?.add(punct)
    })
}

// decorators.py:36
pub fn register<'py>(m: &rt::Module<'py>, r#fn: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "register", 36, |frame| {
        frame.line(37);
        m.global("registry")?
            .getattr("append")?
            .call1((r#fn.getattr("__name__")?,))?;
        Ok(r#fn.clone())
    })
}

// decorators.py:45
pub fn first<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::int(m.py(), 1))
}

// decorators.py:50
pub fn second<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "second", 49, |frame| {
        frame.line(51);
        m.global("first")?.call0()?.add(1)
    })
}

// decorators.py:55
pub fn fib<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "fib", 54, |frame| {
        frame.line(56);
        if n.lt(2)? {
            return Ok(n.clone());
        }
        frame.line(58);
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
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "Decorators: evaluation and application order, functools helpers.",
        )?;
        frame.line(3);
        m.set_global("functools", rt::import(m, "functools")?)?;
        rt::def!(m, trace(label))?;
        rt::def!(m, default_value(label))?;
        frame.line(31);
        {
            let decorator = trace(m, &rt::str(py, "outer"))?;
            let decorator2 = trace(m, &rt::str(py, "inner"))?;
            let function = rt::function!(
                m,
                greet(name, punct = default_value(m, &rt::str(py, "!"))?),
                doc = "Say hello."
            )?;
            let function = decorator2.call1((&function,))?;
            m.set_global("greet", decorator.call1((function,))?)?;
        }
        rt::def!(m, register(r#fn))?;
        frame.line(41);
        m.set_global("registry", PyList::empty(py))?;
        frame.line(45);
        {
            let decorator = m.global("register")?;
            let function = rt::function!(m, first())?;
            m.set_global("first", decorator.call1((function,))?)?;
        }
        frame.line(50);
        {
            let decorator = m.global("register")?;
            let function = rt::function!(m, second())?;
            m.set_global("second", decorator.call1((function,))?)?;
        }
        frame.line(55);
        {
            let decorator = rt::call!(
                m.global("functools")?.getattr("lru_cache")?,
                maxsize = py.None()
            )?;
            let function = rt::function!(m, fib(n))?;
            m.set_global("fib", decorator.call1((function,))?)?;
        }
        rt::def!(m, replace(r#fn))?;
        frame.line(66);
        {
            let decorator = m.global("replace")?;
            let function = rt::function!(m, gone())?;
            m.set_global("gone", decorator.call1((function,))?)?;
        }
        frame.line(70);
        rt::builtins::print(py, (m.global("greet")?.call1(("world",))?,))?;
        frame.line(71);
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
        frame.line(72);
        rt::builtins::print(
            py,
            (m.global("greet")?
                .getattr("__wrapped__")?
                .call_method1("__wrapped__", ("x", "?"))?,),
        )?;
        frame.line(73);
        rt::builtins::print(py, (m.global("registry")?, m.global("second")?.call0()?))?;
        frame.line(74);
        rt::builtins::print(
            py,
            (
                m.global("fib")?.call1((80,))?,
                m.global("fib")?
                    .call_method0("cache_info")?
                    .getattr("hits")?,
            ),
        )?;
        frame.line(75);
        rt::builtins::print(py, (m.global("gone")?,))?;
        Ok(())
    })
}
