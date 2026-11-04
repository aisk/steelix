use pyo3::exceptions::PyLookupError;
use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// async_uncaught.py:7
pub async fn numbers<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "numbers", 7, async |frame| {
        for i in rt::iter(&rt::builtin(py, "range")?.call1((n,))?)? {
            let i = i?;
            if i.eq(2)? {
                return Err(frame.raise(PyLookupError::new_err((rt::fstring(
                    py,
                    ("no ", rt::format(&i, "")?),
                )?
                .unbind(),))));
            }
            rt::yield_(&i).await?;
        }
        Ok(rt::none(py))
    })
}

// async_uncaught.py:14
pub async fn delegate<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "delegate", 14, async |_| {
        let mut total = rt::int(py, 0);
        for value in rt::iter(&m.global("numbers")?.call1((n,))?)? {
            let value = value?;
            total = rt::iadd(&total, &value)?;
        }
        rt::yield_(&total).await?;
        rt::delegate(numbers(m, n)).await?;
        Ok(rt::none(py))
    })
}

// async_uncaught.py:31
pub async fn leaf<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "leaf", 31, async |_| {
        let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
        rt::await_(sleep).await?;
        let values = PyList::empty(m.py()).into_any();
        for value in rt::iter(&m.global("delegate")?.call1((n,))?)? {
            let value = value?;
            values.method1("append", (&value,))?;
        }
        Ok(values)
    })
}

// async_uncaught.py:39
pub async fn middle<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "middle", 39, async |frame| {
        if let rt::Flow::Return(value) = frame
            .async_with(&m.global("Guard")?.call0()?, async |_| {
                let result: PyResult<Bound<'py, PyAny>> =
                    async { rt::delegate(leaf(m, n)).await }.await;
                let finally = frame.finally(result);
                rt::builtins::print(py, ("middle finally",))?;
                Ok(rt::Flow::Return(finally.resume()?))
            })
            .await?
        {
            return Ok(value);
        }
        Ok(rt::none(py))
    })
}

// async_uncaught.py:47
pub async fn top<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "top", 47, async |_| {
        rt::builtins::print(py, (rt::delegate(leaf(m, &rt::int(py, 2))).await?,))?;
        rt::delegate(middle(m, &rt::int(py, 5))).await
    })
}

pub mod guard {
    use super::*;

    // async_uncaught.py:22
    /// The body of `class Guard`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Guard", 22, |_| {
            rt::def!(ns, async __aenter__(slf as "self"))?;
            rt::def!(ns, async __aexit__(slf as "self", typ, exc, tb))?;
            Ok(())
        })
    }

    // async_uncaught.py:23
    pub async fn __aenter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Guard.__aenter__", 23, async |_| { Ok(slf.clone()) })
    }

    // async_uncaught.py:26
    pub async fn __aexit__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        typ: &Bound<'py, PyAny>,
        exc: &Bound<'py, PyAny>,
        tb: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Guard.__aexit__", 26, async |_| {
            rt::builtins::print(py, ("exit with", typ.attr("__name__")?))?;
            Ok(rt::bool(py, false))
        })
    }
}

/// Executes the body of `async_uncaught.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global(
            "__doc__",
            "An exception raised through awaits, `yield from` and `async with`:\n\
                the traceback has CPython's frames and lines (design §8, §9).",
        )?;
        m.set_global("asyncio", rt::import(m, "asyncio")?)?;
        rt::def!(m, gen numbers(n))?;
        rt::def!(m, gen delegate(n))?;
        m.set_global("Guard", rt::class!(m, "Guard"(), guard::body, line = 22)?)?;
        rt::def!(m, async leaf(n))?;
        rt::def!(m, async middle(n))?;
        rt::def!(m, async top())?;
        m.global("asyncio")?
            .attr("run")?
            .call1((m.global("top")?.call0()?,))?;
        Ok(())
    })
}
