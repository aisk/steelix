use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// coroutine_warnings.py:11
pub async fn r#gen<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "gen", 11, async |frame| {
        match rt::yield_(rt::int(py, 1)).await {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyValueError>(py) => {
                let _caught = frame.catch(err);
                rt::yield_(rt::int(py, 2)).await?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

// coroutine_warnings.py:18
pub async fn coro<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "coro", 18, async |_| {
        let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
        rt::await_(sleep).await?;
        Ok(rt::none(m.py()))
    })
}

// coroutine_warnings.py:22
pub fn make_coroutines<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "make_coroutines", 22, |_| {
        let mut first = Some(m.global("coro")?.call0()?);
        first = None;
        let mut second = Some(m.global("coro")?.call0()?);
        second = None;
        m.global("gc")?.method0("collect")?;
        Ok(rt::none(m.py()))
    })
}

// coroutine_warnings.py:30
pub fn throw_three_arguments<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "throw_three_arguments", 30, |_| {
        let g = m.global("gen")?.call0()?;
        rt::builtin(py, "next")?.call1((&g,))?;
        g.attr("throw")?.call1((
            rt::builtin(py, "ValueError")?,
            rt::builtin(py, "ValueError")?.call1(("v",))?,
            py.None(),
        ))
    })
}

// coroutine_warnings.py:52
pub async fn main<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "main", 52, async |frame| {
        // coroutine_warnings.py:53
        async fn pending<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "main.<locals>.pending", 53, async |_| {
                let sleep = m.global("asyncio")?.method1("sleep", (10,))?;
                rt::await_(sleep).await?;
                Ok(rt::none(m.py()))
            })
        }
        let pending = rt::function!(m, async pending(), qualname = "main.<locals>.pending")?;
        let task = rt::call!(
            m.global("asyncio")?.attr("create_task")?,
            pending.call0()?,
            name = "T"
        )?;
        let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
        rt::await_(sleep).await?;
        rt::builtins::print(
            py,
            (
                "task repr says where:",
                task.repr()?.contains("running at")?,
            ),
        )?;
        task.method0("cancel")?;
        match rt::await_(&task).await {
            Ok(_) => {}
            Err(err)
                if frame.matches(&err, 52, || m.global("asyncio")?.attr("CancelledError"))? =>
            {
                let _caught = frame.catch(err);
                rt::builtins::print(py, ("cancelled",))?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

/// Executes the body of `coroutine_warnings.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "What follows from native coroutines and generators having no Python frame\n\
                (design §12): warnings they cause are attributed to `<sys>:0`, so the\n\
                default filters hide `DeprecationWarning`s and repeated warnings are shown\n\
                once, and an asyncio task's repr does not say where the coroutine is.",
        )?;
        m.set_global("asyncio", rt::import(m, "asyncio")?)?;
        m.set_global("gc", rt::import(m, "gc")?)?;
        m.set_global("warnings", rt::import(m, "warnings")?)?;
        rt::def!(m, gen r#gen())?;
        rt::def!(m, async coro())?;
        rt::def!(m, make_coroutines())?;
        rt::def!(m, throw_three_arguments())?;
        // With the default filters, as a program runs.
        frame.with(
            &rt::call!(m.global("warnings")?.attr("catch_warnings")?, record = true)?,
            |value| {
                m.set_global("caught", value)?;
                rt::builtins::print(py, ("throw returned", throw_three_arguments(m)?))?;
                make_coroutines(m)?;
                Ok(())
            },
        )?;
        for item in rt::iter(
            PyList::new(
                py,
                [
                    &rt::builtin(py, "DeprecationWarning")?,
                    &rt::builtin(py, "RuntimeWarning")?,
                ],
            )?
            .as_any(),
        )? {
            m.set_global("category", item?)?;
            let list = PyList::empty(py);
            for w in rt::iter(&m.global("caught")?)? {
                let w = w?;
                if w.attr("category")?.is(m.global("category")?) {
                    list.append(&w)?;
                }
            }
            m.set_global("shown", list)?;
            rt::builtins::print(
                py,
                (
                    m.global("category")?.attr("__name__")?,
                    "shown with the default filters:",
                    m.global("shown")?.len()?,
                ),
            )?;
        }
        // Everything shown, each time.
        frame.with(
            &rt::call!(m.global("warnings")?.attr("catch_warnings")?, record = true)?,
            |value| {
                m.set_global("caught", value)?;
                m.global("warnings")?.method1("simplefilter", ("always",))?;
                throw_three_arguments(m)?;
                make_coroutines(m)?;
                Ok(())
            },
        )?;
        rt::builtins::print(py, ("warnings with 'always':", m.global("caught")?.len()?))?;
        rt::def!(m, async main())?;
        m.global("asyncio")?
            .attr("run")?
            .call1((m.global("main")?.call0()?,))?;
        Ok(())
    })
}
