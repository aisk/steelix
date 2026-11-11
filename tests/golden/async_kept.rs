//! Parts of `async_kept.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 8, `def ticks`: asynchronous generator
//! - line 15, `def collect`: asynchronous comprehension
//! - line 20, `def doubled`: await in a comprehension
//! - line 25, `def lazy`: yield in a lambda
//! - line 30, `def legacy`: types.coroutine generator
//! - line 37, `def handled`: await in an except clause
//! - line 45, `def with_default`: await in a decorator or default

use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// async_kept.py:53
#[rt::frame(m, "native", 53)]
pub async fn native<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut total = rt::int(py, 0);
    {
        let iter = rt::aiter(&m.global("ticks")?.call1((n,))?)?;
        while let Some(i) = rt::anext(&iter).await? {
            total = rt::iadd(&total, &i)?;
            py.check_signals()?;
        }
    }
    let collect = m.global("collect")?.call1((n,))?;
    let value = rt::await_(collect).await?;
    let doubled = m.global("doubled")?.call1((PyList::new(py, [1, 2])?,))?;
    Ok(PyTuple::new(py, [&total, &value, &rt::await_(doubled).await?])?.into_any())
}

// async_kept.py:60
#[rt::frame(m, "native_gen", 60)]
pub async fn native_gen<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let iterable = m.global("lazy")?.call0()?.call0()?;
    rt::yield_from(iterable).await?;
    rt::yield_(rt::str(py, "native")).await?;
    Ok(rt::none(py))
}

// async_kept.py:65
#[rt::frame(m, "main", 65)]
pub async fn main<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::builtins::print(py, (rt::delegate(native(m, &rt::int(py, 3))).await?,))?;
    rt::builtins::print(
        py,
        (rt::builtins::list(py, (m.global("native_gen")?.call0()?,))?,),
    )?;
    let legacy = m.global("legacy")?.call0()?;
    rt::builtins::print(py, (rt::await_(legacy).await?,))?;
    let handled = m
        .global("handled")?
        .call1((rt::builtin(py, "KeyError")?,))?;
    let value = rt::await_(handled).await?;
    let with_default = m.global("with_default")?.call0()?;
    let value2 = rt::await_(with_default).await?;
    let awaitable = m.global("with_default")?.call1((1,))?;
    rt::builtins::print(py, (value, value2, rt::await_(awaitable).await?))?;
    Ok(rt::none(py))
}

/// Executes the body of `async_kept.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    m.set_global(
        "__doc__",
        "What stays Python in coroutines and generators (design §8, §13), next\n\
            to native ones that call it.",
    )?;
    m.set_global("asyncio", rt::import(m, "asyncio")?)?;
    m.set_global("types", rt::import(m, "types")?)?;
    // async_kept.py:8, kept as Python: asynchronous generator at 8:1
    rt::exec_python!(
        m,
        line = 8,
        r#"
async def ticks(n):
    # An asynchronous generator.
    for i in range(n):
        await asyncio.sleep(0)
        yield i
"#
    )?;
    // async_kept.py:15, kept as Python: asynchronous comprehension at 17:12
    rt::exec_python!(
        m,
        line = 15,
        r#"
async def collect(n):
    # An asynchronous comprehension.
    return [i async for i in ticks(n)]
"#
    )?;
    // async_kept.py:20, kept as Python: await in a comprehension at 22:13
    rt::exec_python!(
        m,
        line = 20,
        r#"
async def doubled(values):
    # `await` in a comprehension.
    return [await asyncio.sleep(0, v * 2) for v in values]
"#
    )?;
    // async_kept.py:25, kept as Python: yield in a lambda at 27:21
    rt::exec_python!(
        m,
        line = 25,
        r#"
def lazy():
    # `yield` in a lambda.
    return lambda: (yield 1)
"#
    )?;
    // async_kept.py:30, kept as Python: types.coroutine generator at 31:1
    rt::exec_python!(
        m,
        line = 30,
        r#"
@types.coroutine
def legacy():
    # A generator that is a coroutine.
    value = yield from asyncio.sleep(0, "legacy").__await__()
    return value
"#
    )?;
    // async_kept.py:37, kept as Python: await in an except clause at 41:12
    rt::exec_python!(
        m,
        line = 37,
        r#"
async def handled(make_error):
    # `await` in an `except` clause.
    try:
        raise make_error()
    except await asyncio.sleep(0, KeyError):
        return "caught"
"#
    )?;
    // async_kept.py:45, kept as Python: await in a decorator or default at 47:17
    rt::exec_python!(
        m,
        line = 45,
        r#"
async def with_default(x=None):
    # `await` in the default of a nested function.
    def inner(y=await asyncio.sleep(0, 5)):
        return y

    return inner() if x is None else x
"#
    )?;
    rt::def!(m, async native(n))?;
    rt::def!(m, gen native_gen())?;
    rt::def!(m, async main())?;
    m.global("asyncio")?
        .attr("run")?
        .call1((m.global("main")?.call0()?,))?;
    Ok(())
}
