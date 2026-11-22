use pyo3::exceptions::PyStopIteration;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// thread_switching.py:12
#[rt::frame(m, "finish", 12)]
pub fn finish<'py>(m: &rt::Module<'py>) -> PyResult<()> {
    // Starting the thread lets it run; wait until the loop has started.
    m.global("time")?.method1("sleep", (0.001,))?;
    m.set_global("done", true)?;
    Ok(())
}

// thread_switching.py:19
#[rt::frame(m, "race", 19)]
pub fn race<'py>(
    m: &rt::Module<'py>,
    name: &Bound<'py, PyAny>,
    spin: &Bound<'py, PyAny>,
) -> PyResult<()> {
    m.set_global("done", false)?;
    let worker = rt::call!(
        m.global("threading")?.attr("Thread")?,
        target = m.global("finish")?
    )?;
    worker.method0("start")?;
    let result = spin.call0()?;
    worker.method0("join")?;
    rt::builtins::print(m.py(), (name, &result))?;
    Ok(())
}

// thread_switching.py:29
#[rt::frame(m, "spin_while", 29)]
pub fn spin_while<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut n = rt::int(py, 0);
    while !m.global("done")?.is_truthy()? && n.lt(m.global("LIMIT")?)? {
        n = rt::iadd(&n, 1)?;
        rt::checkpoint(py)?;
    }
    m.global("done")
}

// thread_switching.py:36
#[rt::frame(m, "spin_for", 36)]
pub fn spin_for<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    for underscore in rt::iter(&rt::builtins::range(m.py(), (m.global("LIMIT")?,))?)? {
        let underscore = underscore?;
        if m.global("done")?.is_truthy()? {
            break;
        }
    }
    m.global("done")
}

// thread_switching.py:43
#[rt::frame(m, "spin_comprehension", 43)]
pub fn spin_comprehension<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let list = PyList::empty(py);
    for i in rt::iter(&rt::builtins::range(py, (m.global("LIMIT")?,))?)? {
        let i = i?;
        if !m.global("done")?.is_truthy()? {
            list.append(&i)?;
        }
    }
    let seen = list;
    rt::int(py, seen.len()).rich_compare(m.global("LIMIT")?, CompareOp::Lt)
}

// thread_switching.py:48
#[rt::frame(m, "spin_genexpr", 48, frame)]
pub fn spin_genexpr<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    // `any` is C: only the generator expression's resumptions run native code.
    // thread_switching.py:50
    #[rt::frame(m, "spin_genexpr.<locals>.<genexpr>", 50)]
    fn genexpr<'py>(
        m: &rt::Module<'py>,
        iter: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(underscore) = rt::next(iter)? {
            return Ok(Some(m.global("done")?));
        }
        Ok(None)
    }
    rt::builtins::any(
        py,
        (rt::genexpr!(
            frame,
            genexpr(rt::builtins::range(py, (m.global("LIMIT")?,))?.try_iter()?)
        )?,),
    )
}

// thread_switching.py:53
#[rt::frame(m, "flags", 53)]
pub async fn flags<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    for underscore in rt::iter(&rt::builtins::range(py, (m.global("LIMIT")?,))?)? {
        let underscore = underscore?;
        rt::yield_(m.global("done")?).await?;
    }
    Ok(rt::none(py))
}

// thread_switching.py:58
#[rt::frame(m, "spin_generator", 58)]
pub fn spin_generator<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::builtins::any(m.py(), (m.global("flags")?.call0()?,))
}

// thread_switching.py:62
#[rt::frame(m, "spin_async", 62)]
pub async fn spin_async<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut n = rt::int(py, 0);
    while !m.global("done")?.is_truthy()? && n.lt(m.global("LIMIT")?)? {
        n = rt::iadd(&n, 1)?;
        rt::checkpoint(py)?;
    }
    m.global("done")
}

// thread_switching.py:69
#[rt::frame(m, "run_coroutine", 69, frame)]
pub fn run_coroutine<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    match (|| {
        m.global("spin_async")?
            .call0()?
            .method1("send", (py.None(),))
    })() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
            let stop = frame.catch(err);
            return stop.attr("value");
        }
        Err(err) => return Err(err),
    }
    Ok(rt::none(py))
}

// thread_switching.py:76
#[rt::frame(m, "walk", 76, recursive)]
pub fn walk<'py>(m: &rt::Module<'py>, depth: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    // Recursion without loops: only the calls can let the other thread in.
    if m.global("done")?.is_truthy()? || depth.eq(0)? {
        return Ok(rt::int(m.py(), 0));
    }
    walk(m, &depth.sub(1)?)?
        .add(walk(m, &depth.sub(1)?)?)?
        .add(1)
}

// thread_switching.py:83
#[rt::frame(m, "spin_recursion", 83)]
pub fn spin_recursion<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    walk(m, &rt::int(m.py(), 24))?;
    m.global("done")
}

/// Executes the body of `thread_switching.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    // Native loops let other threads take the GIL, as CPython's eval loop does
    // after `sys.getswitchinterval()`: each loop below waits for a flag that
    // another thread sets, and gives up after far more iterations than that
    // takes, so a loop that keeps the GIL reports False.
    m.set_global("threading", rt::import(m, "threading")?)?;
    m.set_global("time", rt::import(m, "time")?)?;
    m.set_global("LIMIT", 20000000)?;
    m.set_global("done", false)?;
    rt::def!(m, finish())?;
    rt::def!(m, race(name, spin))?;
    rt::def!(m, spin_while())?;
    rt::def!(m, spin_for())?;
    rt::def!(m, spin_comprehension())?;
    rt::def!(m, spin_genexpr())?;
    rt::def!(m, gen flags())?;
    rt::def!(m, spin_generator())?;
    rt::def!(m, async spin_async())?;
    rt::def!(m, run_coroutine())?;
    rt::def!(m, walk(depth))?;
    rt::def!(m, spin_recursion())?;
    race(m, &rt::str(py, "while"), &m.global("spin_while")?)?;
    race(m, &rt::str(py, "for"), &m.global("spin_for")?)?;
    race(
        m,
        &rt::str(py, "comprehension"),
        &m.global("spin_comprehension")?,
    )?;
    race(
        m,
        &rt::str(py, "generator expression"),
        &m.global("spin_genexpr")?,
    )?;
    race(m, &rt::str(py, "generator"), &m.global("spin_generator")?)?;
    race(m, &rt::str(py, "coroutine"), &m.global("run_coroutine")?)?;
    race(m, &rt::str(py, "recursion"), &m.global("spin_recursion")?)?;
    Ok(())
}
